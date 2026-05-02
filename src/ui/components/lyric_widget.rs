// lyrics_widget.rs
// 渲染核心：弹簧动画 + 逐视觉行 clip + 对齐方式 + 翻译 + Seek + 间奏点

use pangocairo::pango;
use relm4::gtk;
use relm4::gtk::cairo;
use relm4::gtk::prelude::*;
use relm4::gtk::glib;
use relm4::gtk::DrawingArea;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

use super::interlude_dots::{InterludeDots, LyricLineInfo};
use super::lyric_line::LyricLineState;
use super::spring::{Spring, SpringParams};

// ─── 样式常量 ──────────────────────────────────────────────────────────────────

const ALPHA_ACTIVE: f64      = 1.0;
const ALPHA_DIM: f64         = 0.28;
const FONT_SIZE_PT: i32      = 17;
const FONT_SIZE_TL_PT: i32   = 12;
const GRADIENT_EDGE_PX: f64  = 6.0;
const LINE_SPACING: f64      = 20.0;  // 歌词句间距
const TL_GAP: f64            = 3.0;   // 主歌词与翻译间距
const PADDING_H: f64         = 24.0;  // 左右内边距
const ACTIVE_LINE_RATIO: f64 = 0.32;

// 垂直滚动弹簧参数（临界阻尼，无振荡）
const SCROLL_SPRING: SpringParams = SpringParams::new(1.0, 20.0, 100.0);

// ─── 对齐方式 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LyricAlign {
    Left,
    #[default]
    Center,
    Right,
}

// ─── 视觉行信息 ────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct VisualLineInfo {
    byte_start: usize,
    byte_end: usize,
    y_offset: f64,
    height: f64,
}

// ─── 缓存结构 ──────────────────────────────────────────────────────────────────

pub struct CachedLine {
    pub line: LyricLine,
    pub layout: pango::Layout,

    pub char_x_offsets: Vec<f64>,
    pub char_widths: Vec<f64>,

    char_visual_line: Vec<usize>,
    visual_lines: Vec<VisualLineInfo>,

    pub layout_height: f64,
    pub tl_layout: Option<pango::Layout>,
    pub tl_height: f64,
    pub total_height: f64,
}

impl CachedLine {
    pub fn build(line: LyricLine, pango_ctx: &pango::Context, available_width: i32) -> Self {
        let layout = make_layout(pango_ctx, FONT_SIZE_PT, available_width, true);
        layout.set_text(&line.text);

        let visual_lines = collect_visual_lines(&layout);

        let (char_x_offsets, char_widths, char_visual_line) = match &line.kind {
            LyricLineKind::Verbatim(chars) => {
                compute_char_metrics(&layout, chars, &visual_lines)
            }
            LyricLineKind::Plain => (Vec::new(), Vec::new(), Vec::new()),
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
            char_visual_line,
            visual_lines,
            layout_height,
            tl_layout,
            tl_height,
            total_height,
        }
    }

    /// 给定当前时间，返回 (已完全点亮字数, 当前字进度 0..1)
    pub fn highlight_progress(&self, current_ms: u64) -> (usize, f64) {
        let chars = match &self.line.kind {
            LyricLineKind::Verbatim(c) => c,
            LyricLineKind::Plain => return (0, 0.0),
        };
        let mut fully_lit = 0usize;
        let mut progress = 0.0f64;
        for (i, ch) in chars.iter().enumerate() {
            if current_ms < ch.start { break; }
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
}

// ─── 组件状态 ──────────────────────────────────────────────────────────────────

pub struct LyricsWidgetState {
    pub cached_lines: Vec<CachedLine>,
    pub current_ms: u64,
    pub align: LyricAlign,
    /// 垂直滚动弹簧（替代原来的指数平滑）
    scroll_spring: Spring,
    /// 每行的动画状态
    line_states: Vec<LyricLineState>,
    /// 间奏动画
    interlude_dots: InterludeDots,
    last_frame_time: Option<Instant>,
    /// 上一次的活跃行索引（用于检测切换）
    last_active_idx: Option<usize>,
    /// 用户正在手动拖拽滚动
    user_scrolling: bool,
    /// 拖拽开始时的滚动位置
    drag_start_scroll: f64,
}

impl LyricsWidgetState {
    pub fn new() -> Self {
        Self {
            cached_lines: Vec::new(),
            current_ms: 0,
            align: LyricAlign::Left,
            scroll_spring: Spring::new(SCROLL_SPRING, 0.0),
            line_states: Vec::new(),
            interlude_dots: InterludeDots::new(),
            last_frame_time: None,
            last_active_idx: None,
            user_scrolling: false,
            drag_start_scroll: 0.0,
        }
    }

    pub fn set_align(&mut self, align: LyricAlign) {
        self.align = align;
    }

    pub fn load_lines(
        &mut self,
        lines: Vec<LyricLine>,
        pango_ctx: &pango::Context,
        available_width: i32,
    ) {
        self.cached_lines = lines
            .iter()
            .map(|l| CachedLine::build(l.clone(), pango_ctx, available_width))
            .collect();

        // 创建每行的动画状态
        let mut y = 0.0;
        self.line_states = self.cached_lines.iter().map(|l| {
            let state = LyricLineState::new(y);
            y += l.total_height + LINE_SPACING;
            state
        }).collect();

        self.scroll_spring.snap_to(0.0);
        self.current_ms = 0;
        self.last_frame_time = None;
        self.last_active_idx = None;
        self.interlude_dots.reset();

        // 检测间奏区间
        let line_infos: Vec<LyricLineInfo> = self.cached_lines.iter().map(|l| LyricLineInfo {
            start: l.line.start,
            duration: l.line.duration,
        }).collect();
        self.interlude_dots.detect(&line_infos, self.current_ms);
    }

    pub fn update_time(&mut self, ms: u64) {
        self.current_ms = ms;
        // 更新间奏检测
        let line_infos: Vec<LyricLineInfo> = self.cached_lines.iter().map(|l| LyricLineInfo {
            start: l.line.start,
            duration: l.line.duration,
        }).collect();
        self.interlude_dots.detect(&line_infos, ms);
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

    /// 计算每行的静态 y 位置（用于滚动计算）
    fn static_y_positions(&self) -> Vec<f64> {
        let mut y = 0.0;
        self.cached_lines.iter().map(|l| {
            let pos = y;
            y += l.total_height + LINE_SPACING;
            pos
        }).collect()
    }

    fn update_scroll_target(&mut self, widget_h: f64, active_idx: usize) {
        let positions = self.static_y_positions();
        if let Some(&line_y) = positions.get(active_idx) {
            let lh = self.cached_lines[active_idx].layout_height;
            let target = line_y + lh / 2.0 - widget_h * ACTIVE_LINE_RATIO;
            self.scroll_spring.set_target(target);
        }
    }

    fn tick_springs(&mut self, dt: f64) {
        // 滚动弹簧
        self.scroll_spring.tick(dt);

        // 每行动画弹簧
        for state in &mut self.line_states {
            state.tick(dt);
        }

        // 间奏动画
        self.interlude_dots.tick(dt);
    }

    /// 更新每行的活跃状态和距离
    fn update_line_states(&mut self) {
        let active_idx = self.active_line_index();

        // 检测活跃行切换
        if active_idx != self.last_active_idx {
            // 旧活跃行取消活跃
            if let Some(old_idx) = self.last_active_idx {
                if old_idx < self.line_states.len() {
                    self.line_states[old_idx].set_active(false);
                }
            }
            // 新活跃行设为活跃
            if let Some(new_idx) = active_idx {
                if new_idx < self.line_states.len() {
                    self.line_states[new_idx].set_active(true);
                }
            }
            self.last_active_idx = active_idx;
        }

        // 计算每行的目标 y 位置
        let positions = self.static_y_positions();
        for (i, state) in self.line_states.iter_mut().enumerate() {
            state.set_target_y(positions[i]);

            // 计算与活跃行的距离
            if let Some(ai) = active_idx {
                let distance = i as i32 - ai as i32;
                state.set_distance(distance);
            }
        }
    }

    pub fn line_at_y(&self, click_y: f64) -> Option<usize> {
        let scroll_y = self.scroll_spring.current_position;
        for (i, cached) in self.cached_lines.iter().enumerate() {
            let top = self.line_states[i].y() - scroll_y;
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
    let scroll_y = state.scroll_spring.current_position;
    let active_idx = state.active_line_index();
    let (fr, fg, fb, fa) = fg_color(widget);
    let align = state.align;

    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.clip();

    for (i, cached) in state.cached_lines.iter().enumerate() {
        let line_state = &state.line_states[i];
        let line_y = line_state.y() - scroll_y;

        // 跳过不在可见区域的行
        if line_y + cached.total_height < 0.0 || line_y > h { continue; }

        let opacity = line_state.opacity();
        let scale = line_state.scale();
        let x_offset = line_state.x_offset();

        if active_idx == Some(i) {
            draw_active_line(
                cr, cached, state.current_ms, line_y, w, align,
                (fr, fg, fb, fa * opacity), scale, x_offset,
            );
        } else {
            draw_dim_line(
                cr, cached, line_y, w, align,
                (fr, fg, fb, fa * opacity), scale, x_offset,
            );
        }
    }

    // 绘制间奏点
    if state.interlude_dots.visible {
        // 间奏点显示在活跃行下方
        if let Some(ai) = active_idx {
            let line_y = state.line_states[ai].y() - scroll_y;
            let dot_y = line_y + state.cached_lines[ai].total_height + 30.0;
            state.interlude_dots.draw(cr, dot_y, w, (fr, fg, fb));
        }
    }
}

fn draw_dim_line(
    cr: &cairo::Context,
    cached: &CachedLine,
    y: f64,
    widget_w: f64,
    align: LyricAlign,
    (r, g, b, fa): (f64, f64, f64, f64),
    scale: f64,
    x_offset: f64,
) {
    cr.save().unwrap();

    let x = x_for_layout(widget_w, &cached.layout, align) + x_offset;

    // 应用缩放变换
    if (scale - 1.0).abs() > 0.001 {
        let center_x = x + cached.layout.pixel_size().0 as f64 / 2.0;
        let center_y = y + cached.layout_height / 2.0;
        cr.translate(center_x, center_y);
        cr.scale(scale, scale);
        cr.translate(-center_x, -center_y);
    }

    cr.move_to(x, y);
    cr.set_source_rgba(r, g, b, fa * ALPHA_DIM);
    pangocairo::functions::show_layout(cr, &cached.layout);
    draw_translation(cr, cached, y + cached.layout_height + TL_GAP, widget_w, align, r, g, b, fa * ALPHA_DIM);
    cr.restore().unwrap();
}

fn draw_active_line(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    y: f64,
    widget_w: f64,
    align: LyricAlign,
    (r, g, b, fa): (f64, f64, f64, f64),
    scale: f64,
    x_offset: f64,
) {
    cr.save().unwrap();

    // 应用缩放变换
    if (scale - 1.0).abs() > 0.001 {
        let x = x_for_layout(widget_w, &cached.layout, align) + x_offset;
        let center_x = x + cached.layout.pixel_size().0 as f64 / 2.0;
        let center_y = y + cached.layout_height / 2.0;
        cr.translate(center_x, center_y);
        cr.scale(scale, scale);
        cr.translate(-center_x, -center_y);
    }

    match &cached.line.kind {
        LyricLineKind::Verbatim(_) => {
            draw_active_verbatim(cr, cached, current_ms, y, widget_w, align, r, g, b, fa, x_offset);
        }
        LyricLineKind::Plain => {
            let x = x_for_layout(widget_w, &cached.layout, align) + x_offset;
            cr.move_to(x, y);
            cr.set_source_rgba(r, g, b, fa * ALPHA_ACTIVE);
            pangocairo::functions::show_layout(cr, &cached.layout);
        }
    }
    draw_translation(cr, cached, y + cached.layout_height + TL_GAP, widget_w, align, r, g, b, fa * ALPHA_DIM);
    cr.restore().unwrap();
}

/// 逐字渐变绘制：逐视觉行独立 clip，修复多行高亮 bug
fn draw_active_verbatim(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    base_y: f64,
    widget_w: f64,
    align: LyricAlign,
    r: f64, g: f64, b: f64, fa: f64,
    x_offset: f64,
) {
    let (fully_lit, char_progress) = cached.highlight_progress(current_ms);
    let n_chars = cached.char_x_offsets.len();

    let layout_x = x_for_layout(widget_w, &cached.layout, align) + x_offset;

    // ── 第一层：暗色全文 ──
    cr.save().unwrap();
    cr.move_to(layout_x, base_y);
    cr.set_source_rgba(r, g, b, fa * ALPHA_DIM);
    pangocairo::functions::show_layout(cr, &cached.layout);
    cr.restore().unwrap();

    // ── 第二层：逐视觉行亮色 clip ──
    for (vl_idx, vl) in cached.visual_lines.iter().enumerate() {
        let chars_in_line: Vec<usize> = (0..n_chars)
            .filter(|&ci| cached.char_visual_line[ci] == vl_idx)
            .collect();

        if chars_in_line.is_empty() { continue; }

        let first_char = *chars_in_line.first().unwrap();
        let last_char  = *chars_in_line.last().unwrap();

        let clip_right: Option<f64> = if fully_lit > last_char {
            let right = cached.char_x_offsets[last_char] + cached.char_widths[last_char];
            Some(right)
        } else if fully_lit >= first_char && fully_lit <= last_char {
            if fully_lit == first_char && char_progress == 0.0 {
                None
            } else {
                let clip = if fully_lit < n_chars && cached.char_visual_line[fully_lit] == vl_idx {
                    cached.char_x_offsets[fully_lit] + cached.char_widths[fully_lit] * char_progress
                } else {
                    cached.char_x_offsets[last_char] + cached.char_widths[last_char]
                };
                Some(clip)
            }
        } else {
            None
        };

        let Some(clip_right) = clip_right else { continue; };
        if clip_right <= 0.0 { continue; }

        let vl_y = base_y + vl.y_offset;

        cr.save().unwrap();
        cr.rectangle(
            layout_x,
            vl_y,
            clip_right + GRADIENT_EDGE_PX,
            vl.height,
        );
        let _ = cr.clip();

        let gx0 = layout_x + clip_right - GRADIENT_EDGE_PX;
        let gx1 = layout_x + clip_right + GRADIENT_EDGE_PX;
        let grad = cairo::LinearGradient::new(gx0, 0.0, gx1, 0.0);
        grad.add_color_stop_rgba(0.0, r, g, b, fa * ALPHA_ACTIVE);
        grad.add_color_stop_rgba(1.0, r, g, b, fa * ALPHA_DIM);

        cr.move_to(layout_x, base_y);
        pangocairo::functions::layout_path(cr, &cached.layout);
        cr.set_source(&grad).unwrap();
        cr.fill().unwrap();

        cr.restore().unwrap();
    }
}

fn draw_translation(
    cr: &cairo::Context,
    cached: &CachedLine,
    tl_y: f64,
    widget_w: f64,
    align: LyricAlign,
    r: f64, g: f64, b: f64, a: f64,
) {
    let Some(tl) = &cached.tl_layout else { return; };
    cr.save().unwrap();
    let x = x_for_layout(widget_w, tl, align);
    cr.move_to(x, tl_y);
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

    da.set_draw_func({
        let state = state.clone();
        move |widget, cr, w, h| {
            draw(&state.borrow(), widget, cr, w, h);
        }
    });

    da.add_tick_callback({
        let state = state.clone();
        move |widget, _frame_clock| {
            let mut st = state.borrow_mut();

            let now = Instant::now();
            let dt = st.last_frame_time
                .map(|t| now.duration_since(t).as_secs_f64())
                .unwrap_or(0.016)
                .min(0.1);
            st.last_frame_time = Some(now);

            // 如果用户正在手动滚动，不自动滚动
            if !st.user_scrolling {
                // 更新每行的活跃状态和距离
                st.update_line_states();

                let h = widget.height() as f64;
                if let Some(idx) = st.active_line_index() {
                    st.update_scroll_target(h, idx);
                }
            }

            // 推进所有弹簧动画
            st.tick_springs(dt);

            if st.active_line_index().is_some() || !st.cached_lines.is_empty() {
                widget.queue_draw();
            }

            gtk::glib::ControlFlow::Continue
        }
    });

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

    let drag_gesture = gtk::GestureDrag::new();
    drag_gesture.connect_drag_begin({
        let state = state.clone();
        move |_, _, _| {
            let mut st = state.borrow_mut();
            st.user_scrolling = true;
            st.drag_start_scroll = st.scroll_spring.current_position;
        }
    });
    drag_gesture.connect_drag_update({
        let state = state.clone();
        move |_, offset_x, offset_y| {
            let mut st = state.borrow_mut();
            let new_scroll = st.drag_start_scroll - offset_y;
            st.scroll_spring.snap_to(new_scroll);
            st.scroll_spring.set_target(new_scroll);
        }
    });
    drag_gesture.connect_drag_end({
        let state = state.clone();
        move |_, _, _| {
            let mut st = state.borrow_mut();
            st.user_scrolling = false;
            st.scroll_spring.current_velocity = 0.0;
        }
    });
    da.add_controller(drag_gesture);

    let scroll_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll_controller.connect_scroll({
        let state = state.clone();
        move |_, _, dy| {
            let mut st = state.borrow_mut();
            let current = st.scroll_spring.current_position;
            let delta = dy * 40.0;
            st.scroll_spring.snap_to(current + delta);
            st.scroll_spring.set_target(current + delta);
            st.user_scrolling = true;
            glib::timeout_add_local_once(std::time::Duration::from_millis(1500), {
                let state = state.clone();
                move || {
                    let mut st = state.borrow_mut();
                    st.user_scrolling = false;
                    st.scroll_spring.current_velocity = 0.0;
                }
            });
            gtk::glib::Propagation::Stop
        }
    });
    da.add_controller(scroll_controller);

    da
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

fn fg_color(widget: &DrawingArea) -> (f64, f64, f64, f64) {
    let c = widget.style_context().color();
    (c.red() as f64, c.green() as f64, c.blue() as f64, c.alpha() as f64)
}

/// 根据对齐方式计算 layout 在 widget 中的 x 起点
fn x_for_layout(widget_w: f64, layout: &pango::Layout, align: LyricAlign) -> f64 {
    let (_, logical) = layout.extents();
    let text_w = logical.width() as f64 / pango::SCALE as f64;
    match align {
        LyricAlign::Left   => PADDING_H,
        LyricAlign::Center => ((widget_w - text_w) / 2.0).max(PADDING_H),
        LyricAlign::Right  => (widget_w - text_w - PADDING_H).max(PADDING_H),
    }
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

/// 收集 layout 中每条视觉行的字节范围和 y 偏移
fn collect_visual_lines(layout: &pango::Layout) -> Vec<VisualLineInfo> {
    let mut result = Vec::new();
    let mut y_accum = 0.0f64;

    for pango_line in layout.lines_readonly() {
        let byte_start = pango_line.start_index() as usize;
        let byte_len   = pango_line.length() as usize;
        let byte_end   = byte_start + byte_len;

        let (_, logical) = pango_line.extents();
        let line_h = logical.height() as f64 / pango::SCALE as f64;

        result.push(VisualLineInfo {
            byte_start,
            byte_end,
            y_offset: y_accum,
            height: line_h,
        });

        y_accum += line_h;
    }

    result
}

/// 计算每个 LyricChar 的 x 偏移、宽度，以及所在视觉行索引
fn compute_char_metrics(
    layout: &pango::Layout,
    chars: &[LyricChar],
    visual_lines: &[VisualLineInfo],
) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let mut offsets    = Vec::with_capacity(chars.len());
    let mut widths     = Vec::with_capacity(chars.len());
    let mut vl_indices = Vec::with_capacity(chars.len());

    let mut byte_idx: i32 = 0;

    for ch in chars {
        let rect = layout.index_to_pos(byte_idx);
        offsets.push(rect.x() as f64 / pango::SCALE as f64);
        widths.push((rect.width() as f64 / pango::SCALE as f64).abs());

        let bidx = byte_idx as usize;
        let vl = visual_lines
            .iter()
            .position(|vl| bidx >= vl.byte_start && bidx < vl.byte_end)
            .unwrap_or(0);
        vl_indices.push(vl);

        byte_idx += ch.ch.len() as i32;
    }

    (offsets, widths, vl_indices)
}
