// lyrics_widget.rs
// 渲染核心：Pango 缓存 + Cairo 双层绘制 + frame clock 缓动 + 翻译 + Seek

use pangocairo::pango;
use relm4::gtk;
use relm4::gtk::cairo;
use relm4::gtk::prelude::*;
use relm4::gtk::DrawingArea;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

// ─── 样式常量 ──────────────────────────────────────────────────────────────────

// 透明度
const ALPHA_ACTIVE: f64      = 1.0;
const ALPHA_DIM: f64         = 0.28;
// 字体
const FONT_SIZE_PT: i32      = 17;
const FONT_SIZE_TL_PT: i32   = 12;

const GRADIENT_EDGE_PX: f64  = 6.0;
const LINE_SPACING: f64      = 20.0;
const TL_GAP: f64            = 3.0;
const PADDING_H: f64         = 24.0;
const ACTIVE_LINE_RATIO: f64 = 0.32;
const SCROLL_EASE: f64       = 0.12;

// ─── 缓存结构 ──────────────────────────────────────────────────────────────────

pub struct CachedLine {
    pub line: LyricLine,
    pub layout: pango::Layout,
    pub char_x_offsets: Vec<f64>,
    pub char_widths: Vec<f64>,
    pub layout_height: f64,
    pub tl_layout: Option<pango::Layout>,
    pub tl_height: f64,
    pub total_height: f64,
}

impl CachedLine {
    pub fn build(line: LyricLine, pango_ctx: &pango::Context, available_width: i32) -> Self {
        let layout = make_layout(pango_ctx, FONT_SIZE_PT, available_width, true);
        layout.set_text(&line.text);

        let (char_x_offsets, char_widths) = match &line.kind {
            LyricLineKind::Verbatim(chars) => compute_char_metrics(&layout, chars),
            LyricLineKind::Plain => (Vec::new(), Vec::new()),
        };

        let layout_height = layout_h(&layout);

        let (tl_layout, tl_height) = if let Some(tl_text) = &line.translation {
            let tl = make_layout(pango_ctx, FONT_SIZE_TL_PT, available_width, false);
            tl.set_text(tl_text);
            let h = layout_h(&tl);
            (Some(tl), h)
        } else {
            (None, 0.0)
        };

        let total_height = if tl_height > 0.0 {
            layout_height + TL_GAP + tl_height
        } else {
            layout_height
        };

        CachedLine {
            line,
            layout,
            char_x_offsets,
            char_widths,
            layout_height,
            tl_layout,
            tl_height,
            total_height,
        }
    }

    pub fn highlight_progress(&self, current_ms: u64) -> (usize, f64) {
        let chars = match &self.line.kind {
            LyricLineKind::Verbatim(c) => c,
            LyricLineKind::Plain => return (0, 0.0),
        };

        let mut fully_lit = 0usize;
        let mut progress = 0.0f64;

        for (i, ch) in chars.iter().enumerate() {
            if current_ms < ch.start {
                break;
            }
            let end = ch.start + ch.duration;
            if current_ms >= end {
                fully_lit = i + 1;
            } else {
                progress = ((current_ms - ch.start) as f64 / ch.duration as f64).clamp(0.0, 1.0);
                break;
            }
        }
        (fully_lit, progress)
    }

    pub fn clip_x(&self, fully_lit: usize, char_progress: f64) -> f64 {
        let n = self.char_x_offsets.len();
        if n == 0 { return 0.0; }
        if fully_lit >= n {
            return self.char_x_offsets[n - 1] + self.char_widths[n - 1];
        }
        self.char_x_offsets[fully_lit] + self.char_widths[fully_lit] * char_progress
    }
}

// ─── 组件状态 ──────────────────────────────────────────────────────────────────

pub struct LyricsWidgetState {
    pub cached_lines: Vec<CachedLine>,
    pub current_ms: u64,
    scroll_y_target: f64,
    scroll_y_current: f64,
    last_frame_time: Option<Instant>,
}

impl LyricsWidgetState {
    pub fn new() -> Self {
        Self {
            cached_lines: Vec::new(),
            current_ms: 0,
            scroll_y_target: 0.0,
            scroll_y_current: 0.0,
            last_frame_time: None,
        }
    }

    pub fn load_lines(
        &mut self,
        lines: Vec<LyricLine>,
        pango_ctx: &pango::Context,
        available_width: i32,
    ) {
        self.cached_lines = lines
            .into_iter()
            .map(|l| CachedLine::build(l, pango_ctx, available_width))
            .collect();
        self.scroll_y_current = 0.0;
        self.scroll_y_target = 0.0;
        self.current_ms = 0;
        self.last_frame_time = None;
    }

    /// gst 只写时间戳，不触发重绘
    pub fn update_time(&mut self, ms: u64) {
        self.current_ms = ms;
    }

    pub fn active_line_index(&self) -> Option<usize> {
        let ms = self.current_ms;
        self.cached_lines
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.line.start <= ms)
            .map(|(i, _)| i)
    }

    pub fn line_y_positions(&self) -> Vec<f64> {
        let mut y = 0.0;
        self.cached_lines.iter().map(|l| {
            let pos = y;
            y += l.total_height + LINE_SPACING;
            pos
        }).collect()
    }

    fn update_scroll_target(&mut self, widget_h: f64, active_idx: usize) {
        let positions = self.line_y_positions();
        if let Some(&line_y) = positions.get(active_idx) {
            let lh = self.cached_lines[active_idx].layout_height;
            self.scroll_y_target = line_y + lh / 2.0 - widget_h * ACTIVE_LINE_RATIO;
        }
    }

    /// frame clock 驱动的帧率无关缓动
    fn tick_scroll(&mut self, dt: f64) {
        let factor = 1.0 - (1.0 - SCROLL_EASE).powf(dt / 0.016);
        self.scroll_y_current += (self.scroll_y_target - self.scroll_y_current) * factor;
    }

    pub fn line_at_y(&self, click_y: f64) -> Option<usize> {
        let positions = self.line_y_positions();
        for (i, cached) in self.cached_lines.iter().enumerate() {
            let top = positions[i] - self.scroll_y_current;
            let bottom = top + cached.total_height;
            if click_y >= top && click_y < bottom {
                return Some(i);
            }
        }
        None
    }
}

// ─── 绘制 ──────────────────────────────────────────────────────────────────────

pub fn draw(
    state: &LyricsWidgetState,
    widget: &DrawingArea,
    cr: &cairo::Context,
    width: i32,
    height: i32,
) {
    let w = width as f64;
    let h = height as f64;
    let scroll_y = state.scroll_y_current;
    let positions = state.line_y_positions();
    let active_idx = state.active_line_index();
    let (fr, fg, fb, fa) = fg_color(widget);

    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.clip();

    for (i, cached) in state.cached_lines.iter().enumerate() {
        let line_y = positions[i] - scroll_y;
        if line_y + cached.total_height < 0.0 || line_y > h { continue; }

        if active_idx == Some(i) {
            draw_active_line(cr, cached, state.current_ms, line_y, w, (fr, fg, fb, fa));
        } else {
            draw_dim_line(cr, cached, line_y, w, (fr, fg, fb, fa));
        }
    }
}

fn draw_dim_line(
    cr: &cairo::Context,
    cached: &CachedLine,
    y: f64,
    widget_w: f64,
    (r, g, b, fa): (f64, f64, f64, f64),
) {
    cr.save().unwrap();
    cr.move_to(center_x(widget_w, &cached.layout), y);
    cr.set_source_rgba(r, g, b, fa * ALPHA_DIM);
    pangocairo::functions::show_layout(cr, &cached.layout);
    draw_translation(cr, cached, y + cached.layout_height + TL_GAP, widget_w, r, g, b, fa * ALPHA_DIM);
    cr.restore().unwrap();
}

fn draw_active_line(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    y: f64,
    widget_w: f64,
    (r, g, b, fa): (f64, f64, f64, f64),
) {
    let x = center_x(widget_w, &cached.layout);

    match &cached.line.kind {
        LyricLineKind::Verbatim(_) => {
            let (fully_lit, char_progress) = cached.highlight_progress(current_ms);
            let clip_right = cached.clip_x(fully_lit, char_progress);

            // 底层：暗色全文
            cr.save().unwrap();
            cr.move_to(x, y);
            cr.set_source_rgba(r, g, b, fa * ALPHA_DIM);
            pangocairo::functions::show_layout(cr, &cached.layout);
            cr.restore().unwrap();

            // 顶层：渐变亮色，clip 到已点亮宽度
            if clip_right > 0.0 {
                cr.save().unwrap();
                cr.rectangle(x, y, clip_right + GRADIENT_EDGE_PX, cached.layout_height);
                let _ = cr.clip();

                let gx0 = x + clip_right - GRADIENT_EDGE_PX;
                let gx1 = x + clip_right + GRADIENT_EDGE_PX;
                let grad = cairo::LinearGradient::new(gx0, 0.0, gx1, 0.0);
                grad.add_color_stop_rgba(0.0, r, g, b, fa * ALPHA_ACTIVE);
                grad.add_color_stop_rgba(1.0, r, g, b, fa * ALPHA_DIM);

                cr.move_to(x, y);
                pangocairo::functions::layout_path(cr, &cached.layout);
                cr.set_source(&grad).unwrap();
                cr.fill().unwrap();
                cr.restore().unwrap();
            }
        }

        LyricLineKind::Plain => {
            cr.save().unwrap();
            cr.move_to(x, y);
            cr.set_source_rgba(r, g, b, fa * ALPHA_ACTIVE);
            pangocairo::functions::show_layout(cr, &cached.layout);
            cr.restore().unwrap();
        }
    }

    // 翻译（当前行翻译样式与非当前行一致）
    draw_translation(cr, cached, y + cached.layout_height + TL_GAP, widget_w, r, g, b, fa * ALPHA_DIM);
}

fn draw_translation(
    cr: &cairo::Context,
    cached: &CachedLine,
    tl_y: f64,
    widget_w: f64,
    r: f64, g: f64, b: f64, a: f64,
) {
    let Some(tl) = &cached.tl_layout else { return; };
    cr.save().unwrap();
    cr.move_to(center_x(widget_w, tl), tl_y);
    cr.set_source_rgba(r, g, b, a);
    pangocairo::functions::show_layout(cr, tl);
    cr.restore().unwrap();
}

// ─── 工厂函数 ──────────────────────────────────────────────────────────────────

pub fn create_lyrics_widget(
    state: Rc<RefCell<LyricsWidgetState>>,
    on_seek: impl Fn(u64) + 'static,
) -> DrawingArea {
    let da = DrawingArea::new();
    da.set_hexpand(true);
    da.set_vexpand(true);

    // draw_func
    da.set_draw_func({
        let state = state.clone();
        move |widget, cr, w, h| {
            draw(&state.borrow(), widget, cr, w, h);
        }
    });

    // tick_callback：frame clock 驱动，稳定 60fps
    da.add_tick_callback({
        let state = state.clone();
        move |widget, _frame_clock| {
            let mut st = state.borrow_mut();

            let now = Instant::now();
            let dt = st.last_frame_time
                .map(|t| now.duration_since(t).as_secs_f64())
                .unwrap_or(0.016)
                .min(0.1); // 防止失焦恢复时跳帧
            st.last_frame_time = Some(now);

            let h = widget.height() as f64;
            if let Some(idx) = st.active_line_index() {
                st.update_scroll_target(h, idx);
            }
            st.tick_scroll(dt);

            // 有激活行（播放中）就持续重绘
            if st.active_line_index().is_some() || !st.cached_lines.is_empty() {
                widget.queue_draw();
            }

            gtk::glib::ControlFlow::Continue
        }
    });

    // GestureClick：点击行 -> Seek
    let gesture = gtk::GestureClick::new();
    gesture.connect_pressed({
        let state = state.clone();
        move |_, _, _x, click_y| {
            let st = state.borrow();
            if let Some(idx) = st.line_at_y(click_y) {
                on_seek(st.cached_lines[idx].line.start);
            }
        }
    });
    da.add_controller(gesture);

    da
}

// ─── 辅助 ─────────────────────────────────────────────────────────────────────

fn fg_color(widget: &DrawingArea) -> (f64, f64, f64, f64) {
    let c = widget.style_context().color();
    (c.red() as f64, c.green() as f64, c.blue() as f64, c.alpha() as f64)
}

fn center_x(widget_w: f64, layout: &pango::Layout) -> f64 {
    let (_, logical) = layout.extents();
    let text_w = logical.width() as f64 / pango::SCALE as f64;
    ((widget_w - text_w) / 2.0).max(PADDING_H)
}

fn make_layout(ctx: &pango::Context, size_pt: i32, available_width: i32, bold: bool) -> pango::Layout {
    let layout = pango::Layout::new(ctx);
    let mut desc = pango::FontDescription::new();
    desc.set_family("Sans");
    desc.set_weight(if bold { pango::Weight::Bold } else { pango::Weight::Normal });
    desc.set_size(size_pt * pango::SCALE);
    layout.set_font_description(Some(&desc));
    layout.set_width(available_width * pango::SCALE);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout
}

fn layout_h(layout: &pango::Layout) -> f64 {
    layout.pixel_size().1 as f64
}

fn compute_char_metrics(layout: &pango::Layout, chars: &[LyricChar]) -> (Vec<f64>, Vec<f64>) {
    let mut offsets = Vec::with_capacity(chars.len());
    let mut widths = Vec::with_capacity(chars.len());
    let mut byte_idx: i32 = 0;
    for ch in chars {
        let rect = layout.index_to_pos(byte_idx);
        offsets.push(rect.x() as f64 / pango::SCALE as f64);
        widths.push((rect.width() as f64 / pango::SCALE as f64).abs());
        byte_idx += ch.ch.len() as i32;
    }
    (offsets, widths)
}