// lyrics_widget.rs
// 渲染核心：逐视觉行 clip + 对齐方式 + frame clock 缓动 + 翻译 + Seek

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

const ALPHA_ACTIVE: f64      = 1.0;
const ALPHA_DIM: f64         = 0.28;
const FONT_SIZE_PT: i32      = 17;
const FONT_SIZE_TL_PT: i32   = 12;
const GRADIENT_EDGE_PX: f64  = 6.0;
const LINE_SPACING: f64      = 20.0;  // 歌词句间距
const TL_GAP: f64            = 3.0;   // 主歌词与翻译间距
const PADDING_H: f64         = 24.0;  // 左右内边距
const ACTIVE_LINE_RATIO: f64 = 0.32;
const SCROLL_EASE: f64       = 0.12;

// ─── 对齐方式 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LyricAlign {
    Left,
    #[default]
    Center,
    Right,
}

// ─── 视觉行信息 ────────────────────────────────────────────────────────────────
//
// Pango 换行后，一个 LyricLine 可能产生多条 LayoutLine（视觉行）。
// 预先记录每条视觉行的字节范围，绘制时逐条独立 clip。

#[derive(Debug)]
struct VisualLineInfo {
    /// 该视觉行在 layout 文本中的起始字节索引
    byte_start: usize,
    /// 该视觉行在 layout 文本中的结束字节索引（exclusive）
    byte_end: usize,
    /// 该视觉行相对于 layout 顶部的 y 偏移（px）
    y_offset: f64,
    /// 该视觉行的高度（px）
    height: f64,
}

// ─── 缓存结构 ──────────────────────────────────────────────────────────────────

pub struct CachedLine {
    pub line: LyricLine,
    pub layout: pango::Layout,

    // 逐字模式：每个 LyricChar 对应的 x 偏移和宽度（Pango 坐标系，px）
    // char_x_offsets[i] 是第 i 个字符在 layout 坐标系内的 x（不含行偏移）
    pub char_x_offsets: Vec<f64>,
    pub char_widths: Vec<f64>,

    // 每个 LyricChar 所在的视觉行索引（用于跨行 clip 判断）
    char_visual_line: Vec<usize>,

    // 预计算的视觉行信息
    visual_lines: Vec<VisualLineInfo>,

    pub layout_height: f64,   // 主歌词整体高度
    pub tl_layout: Option<pango::Layout>,
    pub tl_height: f64,
    pub total_height: f64,
}

impl CachedLine {
    pub fn build(line: LyricLine, pango_ctx: &pango::Context, available_width: i32) -> Self {
        let layout = make_layout(pango_ctx, FONT_SIZE_PT, available_width, true);
        layout.set_text(&line.text);

        // 收集视觉行信息
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
    scroll_y_target: f64,
    scroll_y_current: f64,
    last_frame_time: Option<Instant>,
}

impl LyricsWidgetState {
    pub fn new() -> Self {
        Self {
            cached_lines: Vec::new(),
            current_ms: 0,
            align: LyricAlign::Left,
            scroll_y_target: 0.0,
            scroll_y_current: 0.0,
            last_frame_time: None,
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
            .into_iter()
            .map(|l| CachedLine::build(l, pango_ctx, available_width))
            .collect();
        self.scroll_y_current = 0.0;
        self.scroll_y_target = 0.0;
        self.current_ms = 0;
        self.last_frame_time = None;
    }

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
    let align = state.align;

    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.clip();

    for (i, cached) in state.cached_lines.iter().enumerate() {
        let line_y = positions[i] - scroll_y;
        if line_y + cached.total_height < 0.0 || line_y > h { continue; }

        if active_idx == Some(i) {
            draw_active_line(cr, cached, state.current_ms, line_y, w, align, (fr, fg, fb, fa));
        } else {
            draw_dim_line(cr, cached, line_y, w, align, (fr, fg, fb, fa));
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
) {
    cr.save().unwrap();
    // 非当前行：layout 整体绘制，用 x_for_layout 决定起点
    let x = x_for_layout(widget_w, &cached.layout, align);
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
) {
    match &cached.line.kind {
        LyricLineKind::Verbatim(_) => {
            draw_active_verbatim(cr, cached, current_ms, y, widget_w, align, r, g, b, fa);
        }
        LyricLineKind::Plain => {
            let x = x_for_layout(widget_w, &cached.layout, align);
            cr.save().unwrap();
            cr.move_to(x, y);
            cr.set_source_rgba(r, g, b, fa * ALPHA_ACTIVE);
            pangocairo::functions::show_layout(cr, &cached.layout);
            cr.restore().unwrap();
        }
    }
    draw_translation(cr, cached, y + cached.layout_height + TL_GAP, widget_w, align, r, g, b, fa * ALPHA_DIM);
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
) {
    let (fully_lit, char_progress) = cached.highlight_progress(current_ms);
    let n_chars = cached.char_x_offsets.len();

    // layout 在 widget 中的 x 起点（所有视觉行共用同一个对齐起点）
    let layout_x = x_for_layout(widget_w, &cached.layout, align);

    // ── 第一层：暗色全文（整体绘制即可）──
    cr.save().unwrap();
    cr.move_to(layout_x, base_y);
    cr.set_source_rgba(r, g, b, fa * ALPHA_DIM);
    pangocairo::functions::show_layout(cr, &cached.layout);
    cr.restore().unwrap();

    // ── 第二层：逐视觉行亮色 clip ──
    //
    // 对每条视觉行，独立计算它的 clip_right：
    //   - 如果该视觉行的所有字都已点亮 → clip 到该行末尾（全亮）
    //   - 如果该视觉行包含"当前正在渐变的字" → clip 到该字的进度位置
    //   - 如果该视觉行的字都还未点亮 → 跳过（不绘制亮色层）

    for (vl_idx, vl) in cached.visual_lines.iter().enumerate() {
        // 找出属于本视觉行的字符范围
        let chars_in_line: Vec<usize> = (0..n_chars)
            .filter(|&ci| cached.char_visual_line[ci] == vl_idx)
            .collect();

        if chars_in_line.is_empty() { continue; }

        let first_char = *chars_in_line.first().unwrap();
        let last_char  = *chars_in_line.last().unwrap();

        // 判断本视觉行的高亮状态
        // clip_right 是相对于 layout_x 的偏移（Pango x 坐标，layout 坐标系内）
        let clip_right: Option<f64> = if fully_lit > last_char {
            // 本行所有字已完全点亮
            let right = cached.char_x_offsets[last_char] + cached.char_widths[last_char];
            Some(right)
        } else if fully_lit >= first_char && fully_lit <= last_char {
            // 当前渐变字在本行内
            if fully_lit == first_char && char_progress == 0.0 {
                // 本行第一个字还没开始，跳过
                None
            } else {
                let clip = if fully_lit < n_chars && cached.char_visual_line[fully_lit] == vl_idx {
                    // 渐变字在本行
                    cached.char_x_offsets[fully_lit] + cached.char_widths[fully_lit] * char_progress
                } else {
                    // fully_lit 已超出本行（渐变字在下一行），本行全亮
                    cached.char_x_offsets[last_char] + cached.char_widths[last_char]
                };
                Some(clip)
            }
        } else {
            // 本行字符都在 fully_lit 之前：不应发生（因为上面已处理），或本行全未点亮
            None
        };

        let Some(clip_right) = clip_right else { continue; };
        if clip_right <= 0.0 { continue; }

        // 视觉行的 y 起点（相对于 layout 顶部的偏移 + base_y）
        let vl_y = base_y + vl.y_offset;

        // clip 矩形：layout_x 起点，宽度到 clip_right + 渐变延伸
        cr.save().unwrap();
        cr.rectangle(
            layout_x,
            vl_y,
            clip_right + GRADIENT_EDGE_PX,
            vl.height,
        );
        let _ = cr.clip();

        // 渐变：在 clip_right 处从亮到暗
        let gx0 = layout_x + clip_right - GRADIENT_EDGE_PX;
        let gx1 = layout_x + clip_right + GRADIENT_EDGE_PX;
        let grad = cairo::LinearGradient::new(gx0, 0.0, gx1, 0.0);
        grad.add_color_stop_rgba(0.0, r, g, b, fa * ALPHA_ACTIVE);
        grad.add_color_stop_rgba(1.0, r, g, b, fa * ALPHA_DIM);

        // 只绘制本视觉行的路径（layout_path 会绘制整个 layout，
        // 但 clip 已限制到本视觉行的矩形，所以其他行不受影响）
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

            let h = widget.height() as f64;
            if let Some(idx) = st.active_line_index() {
                st.update_scroll_target(h, idx);
            }
            st.tick_scroll(dt);

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

        // 获取该视觉行的像素高度
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
        // Pango x 在 layout 坐标系中（不含行对齐偏移，行对齐偏移在绘制时加）
        offsets.push(rect.x() as f64 / pango::SCALE as f64);
        widths.push((rect.width() as f64 / pango::SCALE as f64).abs());

        // 找到该字节索引所在的视觉行
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