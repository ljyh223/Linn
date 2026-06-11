// lyrics_widget.rs
// 渲染核心：弹簧动画 + 逐视觉行 clip + 对齐方式 + 翻译 + Seek + 间奏点

use pangocairo::pango;
use relm4::gtk;
use relm4::gtk::cairo;
use relm4::gtk::glib;
use relm4::gtk::prelude::*;
use relm4::gtk::DrawingArea;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

use super::interlude_dots::{InterludeDots, LyricLineInfo};
use super::lyric_line::LyricLineState;
use super::spring::SpringParams;

// ─── 样式常量 ──────────────────────────────────────────────────────────────────

pub const ALPHA_ACTIVE: f64 = 1.0;
pub const ALPHA_DIM: f64 = 0.4;
pub const FONT_SIZE_PT: i32 = 20;
pub const FONT_SIZE_TL_PT: i32 = 13;
pub const GRADIENT_EDGE_PX: f64 = 50.0;
pub const LINE_SPACING: f64 = 20.0;
pub const TL_GAP: f64 = 3.0;
pub const PADDING_H: f64 = 24.0;
pub const ACTIVE_LINE_RATIO: f64 = 0.32;
pub const LINE_SWITCH_DEBOUNCE_MS: u64 = 120;
pub const TOP_PADDING: f64 = 48.0;
pub const FADE_HEIGHT: f64 = 140.0;
/// 距离模糊：每行距离增加的高斯模糊半径（像素），参照 accompanist
pub const BLUR_DELTA: f64 = 3.0;
/// 距离模糊上限（性能保护）
pub const BLUR_MAX: f64 = 12.0;
/// 逐字浮起动画：最大偏移量（像素）
pub const MAX_FLOAT_OFFSET: f64 = 4.0;
/// 逐字浮起动画：持续时间（毫秒）
pub const FLOAT_DURATION_MS: f64 = 700.0;
/// 长字动画：单字最小时长阈值（毫秒）
pub const FAST_CHAR_ANIM_THRESHOLD_MS: f64 = 200.0;
/// 长字动画：单词最小时长阈值（毫秒）
pub const WORD_ANIM_THRESHOLD_MS: f64 = 1000.0;
/// 长字动画：最大下沉/上浮偏移（像素）
pub const MAX_DIP_OFFSET: f64 = 2.0;
/// 长字动画：最大膨胀缩放比例
pub const MAX_SWELL_SCALE: f64 = 0.1;
/// 长字动画：最大发光模糊半径（像素）
pub const MAX_BOUNCE_BLUR: f64 = 10.0;

pub const SCROLL_FRICTION: f64 = 0.95;

// ─── 对齐方式 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LyricAlign {
    #[default]
    Left,
    Center,
    Right,
}

// ─── 视觉行信息 ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct VisualLineInfo {
    pub byte_start: usize,
    pub byte_end: usize,
    pub y_offset: f64,
    pub height: f64,
}

// ─── 缓存结构 ──────────────────────────────────────────────────────────────────

pub struct CachedLine {
    pub line: LyricLine,
    pub layout: pango::Layout,

    pub char_x_offsets: Vec<f64>,
    pub char_widths: Vec<f64>,

    pub char_visual_line: Vec<usize>,
    pub visual_lines: Vec<VisualLineInfo>,
    /// 每个视觉行对应的字符索引列表（预计算，避免每帧 filter）
    pub chars_per_visual_line: Vec<Vec<usize>>,

    pub layout_height: f64,
    pub tl_layout: Option<pango::Layout>,
    pub tl_height: f64,
    pub total_height: f64,

    pub text_width: f64,
    pub tl_text_width: f64,
}

impl CachedLine {
    pub fn build(line: LyricLine, pango_ctx: &pango::Context, available_width: i32) -> Self {
        let layout = make_layout(pango_ctx, FONT_SIZE_PT, available_width, true);
        layout.set_text(&line.text);

        let visual_lines = collect_visual_lines(&layout);

        let (char_x_offsets, char_widths, char_visual_line) = match &line.kind {
            LyricLineKind::Verbatim(chars) => compute_char_metrics(&layout, chars, &visual_lines),
            LyricLineKind::Plain => (Vec::new(), Vec::new(), Vec::new()),
        };

        // 预计算每个视觉行对应的字符索引
        let chars_per_visual_line: Vec<Vec<usize>> = (0..visual_lines.len())
            .map(|vl_idx| {
                (0..char_visual_line.len())
                    .filter(|&ci| char_visual_line[ci] == vl_idx)
                    .collect()
            })
            .collect();

        let layout_height = layout_h(&layout);

        let (_, logical) = layout.extents();
        let text_width = logical.width() as f64 / pango::SCALE as f64;

        let (tl_layout, tl_height, tl_text_width) = if let Some(tl_text) = &line.translation {
            let tl = make_layout(pango_ctx, FONT_SIZE_TL_PT, available_width, false);
            tl.set_text(tl_text);
            let h = layout_h(&tl);

            let (_, tl_logical) = tl.extents();
            let tl_text_width = tl_logical.width() as f64 / pango::SCALE as f64;
            (Some(tl), h, tl_text_width)
        } else {
            (None, 0.0, 0.0)
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
            chars_per_visual_line,
            layout_height,
            tl_layout,
            tl_height,
            total_height,
            text_width,
            tl_text_width,
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
}

// ─── 组件状态 ──────────────────────────────────────────────────────────────────

pub struct LyricsWidgetState {
    pub cached_lines: Vec<CachedLine>,
    pub current_ms: u64,
    pub align: LyricAlign,
    /// 每行独立 pos_y 弹簧（屏幕空间位置，不再减去全局 scroll）
    pub line_states: Vec<LyricLineState>,
    /// 间奏动画
    pub interlude_dots: InterludeDots,
    pub last_frame_time: Option<Instant>,
    pub last_active_idx: Option<usize>,
    pub last_raw_active_idx: Option<usize>,
    /// 用户正在拖拽
    pub user_scrolling: bool,
    /// 首次加载后需触发一次滚动定位
    pub needs_initial_scroll: bool,
    pub text_color_override: Option<(f64, f64, f64, f64)>,
    pub enable_shadow: bool,
    pub cached_y_positions: Vec<f64>,
    pub line_infos: Vec<LyricLineInfo>,
    pub bg_color: (f64, f64, f64),
    /// 拖拽偏移量（像素，加到所有行绘制位置）
    pub drag_offset: f64,
    /// 拖拽惯性速度（像素/秒）
    pub drag_velocity: f64,
    /// 是否正在惯性滚动
    pub is_inertia: bool,
    /// 上一次拖拽偏移量，用于计算拖拽速度
    pub last_drag_offset: f64,
    /// 上一次拖拽时间，用于计算拖拽速度
    pub last_drag_time: Option<Instant>,
}

impl Default for LyricsWidgetState {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsWidgetState {
    pub fn new() -> Self {
        Self {
            cached_lines: Vec::new(),
            current_ms: 0,
            align: LyricAlign::Left,
            line_states: Vec::new(),
            interlude_dots: InterludeDots::new(),
            last_frame_time: None,
            last_active_idx: None,
            last_raw_active_idx: None,
            user_scrolling: false,
            needs_initial_scroll: false,
            text_color_override: None,
            enable_shadow: false,
            cached_y_positions: Vec::new(),
            line_infos: Vec::new(),
            bg_color: (0.0, 0.0, 0.0),
            drag_offset: 0.0,
            drag_velocity: 0.0,
            is_inertia: false,
            last_drag_offset: 0.0,
            last_drag_time: None,
        }
    }

    pub fn set_align(&mut self, align: LyricAlign) {
        self.align = align;
    }
    pub fn set_bg_color(&mut self, r: f64, g: f64, b: f64) {
        self.bg_color = (r, g, b);
    }

    pub fn set_text_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.text_color_override = Some((r, g, b, a));
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
        let mut y = TOP_PADDING;
        self.line_states = self
            .cached_lines
            .iter()
            .map(|l| {
                let state = LyricLineState::new(y);
                y += l.total_height + LINE_SPACING;
                state
            })
            .collect();

        self.current_ms = 0;
        self.last_frame_time = None;
        self.last_active_idx = None;
        self.last_raw_active_idx = None;
        self.needs_initial_scroll = true;
        self.drag_offset = 0.0;
        self.drag_velocity = 0.0;
        self.is_inertia = false;
        self.interlude_dots.reset();

        // 检测间奏区间
        self.line_infos = self
            .cached_lines
            .iter()
            .map(|l| LyricLineInfo {
                start: l.line.start,
                duration: l.line.duration,
            })
            .collect();
        self.cached_y_positions = self.static_y_positions();

        self.interlude_dots
            .detect(&self.line_infos, self.current_ms);
        self.interlude_dots.snap_push();
    }

    pub fn update_time(&mut self, ms: u64) {
        self.current_ms = ms;
        self.interlude_dots.detect(&self.line_infos, ms);
    }

    pub fn active_line_index(&self) -> Option<usize> {
        let ms = self.current_ms;
        let idx = self.cached_lines.partition_point(|l| l.line.start <= ms);
        if idx == 0 {
            None
        } else {
            Some(idx - 1)
        }
    }
    /// 计算每行的静态 y 位置（用于滚动计算）
    fn static_y_positions(&self) -> Vec<f64> {
        let mut y = TOP_PADDING;
        self.cached_lines
            .iter()
            .map(|l| {
                let pos = y;
                y += l.total_height + LINE_SPACING;
                pos
            })
            .collect()
    }

    /// 计算 scroll_center：让活跃行落在视口 ACTIVE_LINE_RATIO 处所需的偏移
    fn scroll_center(&self, active_idx: usize) -> f64 {
        let positions = &self.cached_y_positions;
        positions[active_idx] + self.cached_lines[active_idx].layout_height / 2.0
    }

    /// 间奏中点（无活跃行时的焦点位置）
    fn interlude_center(&self) -> f64 {
        let positions = &self.cached_y_positions;
        let push = self.interlude_dots.push_amount;
        match self.interlude_dots.interlude_idx {
            Some(pi) if pi + 1 < positions.len() => {
                (positions[pi] + self.cached_lines[pi].total_height + positions[pi + 1] + push)
                    / 2.0
            }
            _ => TOP_PADDING + push / 2.0,
        }
    }

    /// 统一更新：活跃行检测 + 逐行屏幕位置目标 + 距离/缩放/透明度
    /// 每帧调用（非拖拽状态），确保间奏推挤平滑
    pub fn update_line_positions(&mut self, widget_h: f64) {
        let raw_active = self.active_line_index();

        // 防抖
        let active_idx = match (self.last_active_idx, raw_active) {
            (Some(confirmed), Some(candidate)) if candidate > confirmed => {
                let elapsed = self
                    .current_ms
                    .saturating_sub(self.cached_lines[candidate].line.start);
                if elapsed < LINE_SWITCH_DEBOUNCE_MS {
                    Some(confirmed)
                } else {
                    raw_active
                }
            }
            _ => raw_active,
        };

        // 活跃行切换
        let line_switched = active_idx != self.last_active_idx;
        if line_switched {
            if let Some(old_idx) = self.last_active_idx {
                if old_idx < self.line_states.len() {
                    self.line_states[old_idx].set_active(false);
                }
            }
            if let Some(new_idx) = active_idx {
                if new_idx < self.line_states.len() {
                    self.line_states[new_idx].set_active(true);
                }
            }
            self.last_active_idx = active_idx;
        }

        // 焦点中心（活跃行中心 或 间奏中点）
        let center = active_idx
            .map(|ai| self.scroll_center(ai))
            .or_else(|| {
                if self.interlude_dots.visible {
                    Some(self.interlude_center())
                } else {
                    None
                }
            })
            .unwrap_or(0.0);

        // 视口偏移：让焦点落在 32% 处
        let focus_offset = widget_h * ACTIVE_LINE_RATIO;

        let positions = &self.cached_y_positions;
        let push = self.interlude_dots.push_amount;
        let push_idx = self.interlude_dots.interlude_idx;
        let visible = self.interlude_dots.visible;

        for (i, state) in self.line_states.iter_mut().enumerate() {
            let mut abs_y = positions[i];
            if visible {
                match push_idx {
                    Some(pi) if i > pi => abs_y += push,
                    None => abs_y += push,
                    _ => {}
                }
            }

            // 屏幕空间位置：绝对位置 - 焦点中心 + 视口偏移
            // drag_offset 不纳入弹簧目标，在绘制时叠加（避免每帧重建求解器）
            let screen_y = abs_y - center + focus_offset;

            // 行切换时用级联刚度，否则用默认弹簧（间奏推挤连续更新）
            if let Some(ai) = active_idx {
                if line_switched {
                    let dist = (i as i32 - ai as i32).unsigned_abs() as u32;
                    let stiffness = match dist {
                        0 => 220.0,
                        1 => 180.0,
                        2 => 130.0,
                        _ => 70.0,
                    };
                    state.set_target_y_with_stiffness(screen_y, stiffness);
                } else {
                    state.set_target_y(screen_y);
                }
                state.set_distance(i as i32 - ai as i32);
            } else {
                state.set_target_y(screen_y);
            }
        }
    }

    pub fn line_at_y(&self, click_y: f64) -> Option<usize> {
        for (i, cached) in self.cached_lines.iter().enumerate() {
            let top = self.line_states[i].y() + self.drag_offset;
            let bottom = top + cached.total_height;
            if click_y >= top && click_y < bottom {
                return Some(i);
            }
        }
        None
    }

    pub fn tick_springs(&mut self, dt: f64) {
        for state in &mut self.line_states {
            state.tick(dt);
        }
        self.interlude_dots.tick(dt);
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
    let drag_offset = state.drag_offset;
    let active_idx = state.last_active_idx;
    let align = state.align;

    // 文字颜色：优先覆写（全屏白色），否则取系统前景色
    let (fr, fg, fb, fa) = state.text_color_override
        .unwrap_or_else(|| fg_color(widget));
    let shadow = state.enable_shadow;

    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.clip();

    for (i, cached) in state.cached_lines.iter().enumerate() {
        let line_state = &state.line_states[i];
        let line_y = line_state.y() + drag_offset;

        // 跳过不在可见区域的行
        if line_y + cached.total_height < 0.0 || line_y > h { continue; }

        // 垂直渐隐：接近顶部/底部时降低透明度
        let fade_alpha = {
            let top_fade = if line_y < FADE_HEIGHT {
                (line_y / FADE_HEIGHT).max(0.0)
            } else {
                1.0
            };
            let bottom_in = line_y + cached.total_height - (h - FADE_HEIGHT);
            let bottom_fade = if bottom_in > 0.0 {
                1.0 - (bottom_in / FADE_HEIGHT).min(1.0)
            } else {
                1.0
            };
            top_fade * bottom_fade
        };

        let alpha = line_state.current_alpha * fade_alpha;
        let scale = line_state.scale();

        if active_idx == Some(i) {
            draw_active_line(
                cr, cached, state.current_ms, line_y, w, align,
                (fr, fg, fb, fa * alpha), scale, shadow, state.bg_color
            );
        } else {
            draw_dim_line(
                cr, cached, line_y, w, align,
                (fr, fg, fb, fa * alpha), scale, state.bg_color
            );
        }
    }

    // 绘制间奏点（同样应用渐隐）
    if state.interlude_dots.visible {
        if let Some(pi) = state.interlude_dots.interlude_idx {
            if pi + 1 < state.line_states.len() {
                let bottom = state.line_states[pi].y() + drag_offset
                    + state.cached_lines[pi].total_height;
                let top_next = state.line_states[pi + 1].y() + drag_offset;
                let dot_y = (bottom + top_next) / 2.0;
                let dot_fade = fade_alpha_for_y(dot_y, h, FADE_HEIGHT);
                state.interlude_dots.draw(cr, dot_y, w, state.current_ms, (fr * dot_fade, fg * dot_fade, fb * dot_fade));
            }
        } else if !state.line_states.is_empty() {
            let push = state.interlude_dots.push_amount;
            let dot_y = TOP_PADDING + push / 2.0 + drag_offset;
            let dot_fade = fade_alpha_for_y(dot_y, h, FADE_HEIGHT);
            state.interlude_dots.draw(cr, dot_y, w, state.current_ms, (fr * dot_fade, fg * dot_fade, fb * dot_fade));
        }
    }
}

pub fn draw_dim_line(
    cr: &cairo::Context,
    cached: &CachedLine,
    y: f64,
    widget_w: f64,
    align: LyricAlign,
    (r, g, b, fa): (f64, f64, f64, f64),
    scale: f64,
    bg_color: (f64, f64, f64),
) {
    cr.save().unwrap();

    let x = x_for_layout(widget_w, cached.text_width, align);

    // 应用缩放变换（左边缘锚定，所有行左对齐）
    if (scale - 1.0).abs() > 0.001 {
        cr.translate(x, y);
        cr.scale(scale, scale);
        cr.translate(-x, -y);
    }

    cr.move_to(x, y);
    let (r, g, b) = dim_color((r, g, b), bg_color);

    cr.set_source_rgba(r, g, b, fa);
    pangocairo::functions::show_layout(cr, &cached.layout);
    draw_translation(
        cr,
        cached,
        y + cached.layout_height + TL_GAP,
        widget_w,
        align,
        r,
        g,
        b,
        fa * ALPHA_DIM,
    );
    cr.restore().unwrap();
}

pub fn draw_active_line(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    y: f64,
    widget_w: f64,
    align: LyricAlign,
    (r, g, b, fa): (f64, f64, f64, f64),
    scale: f64,
    shadow: bool,
    bg_color: (f64, f64, f64),
) {
    cr.save().unwrap();

    let layout_x = x_for_layout(widget_w, cached.text_width, align);

    // 应用缩放变换（左边缘锚定，所有行左对齐）
    if (scale - 1.0).abs() > 0.001 {
        cr.translate(layout_x, y);
        cr.scale(scale, scale);
        cr.translate(-layout_x, -y);
    }

    // 文字阴影（仅活跃行，增强背景对比度）
    if shadow {
        let shadow_alpha = (fa * 0.35).min(0.35);
        cr.save().unwrap();
        cr.move_to(layout_x + 1.0, y + 1.0);
        cr.set_source_rgba(0.0, 0.0, 0.0, shadow_alpha);
        pangocairo::functions::show_layout(cr, &cached.layout);
        cr.restore().unwrap();
    }

    // P1-1: 叠加发光模式（参照 accompanist-lyrics-ui BlendMode.Plus）
    cr.save().unwrap();
    cr.set_operator(cairo::Operator::Add);

    match &cached.line.kind {
        LyricLineKind::Verbatim(_) => {
            draw_active_verbatim(
                cr,
                cached,
                current_ms,
                y,
                widget_w,
                align,
                bg_color,
                (r, g, b),
                fa,
            );
        }
        LyricLineKind::Plain => {
            cr.move_to(layout_x, y);
            cr.set_source_rgba(r, g, b, fa * ALPHA_ACTIVE);
            pangocairo::functions::show_layout(cr, &cached.layout);
        }
    }

    // Translation（纳入叠加发光范围）
    draw_translation(
        cr,
        cached,
        y + cached.layout_height + TL_GAP,
        widget_w,
        align,
        r,
        g,
        b,
        fa * ALPHA_DIM,
    );

    // 叠加发光结束
    cr.restore().unwrap();

    cr.restore().unwrap();
}

/// 逐字渐变绘制：逐视觉行独立 clip，修复多行高亮 bug
pub fn draw_active_verbatim(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    base_y: f64,
    widget_w: f64,
    align: LyricAlign,
    bg_color: (f64, f64, f64),
    (r, g, b): (f64, f64, f64),
    fa: f64,
) {
    let (fully_lit, char_progress) = cached.highlight_progress(current_ms);
    let n_chars = cached.char_x_offsets.len();

    let layout_x = x_for_layout(widget_w, cached.text_width, align);

    // ── 第一层：暗色全文 ──
    cr.save().unwrap();
    cr.move_to(layout_x, base_y);
    let (dim_r, dim_g, dim_b) = dim_color((r, g, b), bg_color);

    cr.set_source_rgba(dim_r, dim_g, dim_b, fa);
    pangocairo::functions::show_layout(cr, &cached.layout);
    cr.restore().unwrap();

    // ── 第二层：逐视觉行亮色 clip ──
    for (vl_idx, vl) in cached.visual_lines.iter().enumerate() {
        let chars_in_line: Vec<usize> = (0..n_chars)
            .filter(|&ci| cached.char_visual_line[ci] == vl_idx)
            .collect();

        if chars_in_line.is_empty() {
            continue;
        }

        let first_char = *chars_in_line.first().unwrap();
        let last_char = *chars_in_line.last().unwrap();

        let clip_right: Option<f64> = if fully_lit > last_char {
            // 整行已唱完，全行点亮
            Some(cached.char_x_offsets[last_char] + cached.char_widths[last_char])
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

        let Some(clip_right) = clip_right else {
            continue;
        };
        if clip_right <= 0.0 {
            continue;
        }

        let vl_y = base_y + vl.y_offset;

        cr.save().unwrap();
        cr.rectangle(layout_x, vl_y, clip_right + GRADIENT_EDGE_PX, vl.height);
        let _ = cr.clip();

        let gx0 = layout_x + clip_right - GRADIENT_EDGE_PX;
        let gx1 = layout_x + clip_right + GRADIENT_EDGE_PX;
        let (dim_r, dim_g, dim_b) = dim_color((r, g, b), bg_color);

        let grad = cairo::LinearGradient::new(gx0, 0.0, gx1, 0.0);
        // P1-3: 缓动渐变（参照 accompanist EaseInQuart）
        let steps = 10;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let eased = ease_in_quart(t);
            let stop_r = r + (dim_r - r) * eased;
            let stop_g = g + (dim_g - g) * eased;
            let stop_b = b + (dim_b - b) * eased;
            let stop_a = fa * ALPHA_ACTIVE + (fa - fa * ALPHA_ACTIVE) * eased;
            grad.add_color_stop_rgba(t, stop_r, stop_g, stop_b, stop_a);
        }

        cr.move_to(layout_x, base_y);
        pangocairo::functions::layout_path(cr, &cached.layout);
        cr.set_source(&grad).unwrap();
        cr.fill().unwrap();

        cr.restore().unwrap();
    }

    // ── 第三层：长字强调发光 ──
    // 当前字时长 ≥ 1000ms 时，在当前字位置叠加发光脉冲
    let chars = match &cached.line.kind {
        LyricLineKind::Verbatim(c) => c,
        _ => return,
    };
    if fully_lit < n_chars {
        let ch = &chars[fully_lit];
        let dur = ch.duration;
        if dur >= 1000 {
            let progress = ((current_ms - ch.start) as f64 / dur as f64).clamp(0.0, 1.0);
            let pulse = ease_in_out_cubic(progress);
            // 发光强度随字长递增，上限 0.35
            let glow_alpha = ((dur as f64 - 1000.0) / 2000.0).min(1.0) * 0.35 * pulse;

            let char_x = layout_x + cached.char_x_offsets[fully_lit];
            let char_w = cached.char_widths[fully_lit];
            let vl_idx = cached.char_visual_line[fully_lit];
            let vl_y = base_y + cached.visual_lines[vl_idx].y_offset;
            let vl_h = cached.visual_lines[vl_idx].height;

            // 在字符区域绘制发光叠加
            cr.save().unwrap();
            cr.rectangle(
                char_x - GRADIENT_EDGE_PX,
                vl_y,
                char_w + 2.0 * GRADIENT_EDGE_PX,
                vl_h,
            );
            let _ = cr.clip();
            cr.move_to(layout_x, base_y);
            cr.set_source_rgba(r, g, b, fa * glow_alpha);
            pangocairo::functions::show_layout(cr, &cached.layout);
            cr.restore().unwrap();
        }
    }

    // ── 第四层：逐字浮起动画（参照 accompanist-lyrics-ui） ──
    draw_floating_characters(
        cr, cached, current_ms, layout_x, base_y,
        (r, g, b), (dim_r, dim_g, dim_b),
        fa * ALPHA_ACTIVE, fa,
    );

    // ── 第五层：长字动画（下沉-上浮 + 膨胀 + 弹跳发光） ──
    draw_long_word_animations(
        cr, cached, current_ms, layout_x, base_y,
        (r, g, b), fa * ALPHA_ACTIVE,
    );
}

/// easeInOutCubic: 缓入缓出三次曲线
pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// easeInQuart: 缓入四次曲线（参照 accompanist EaseInQuart）
pub fn ease_in_quart(t: f64) -> f64 {
    t * t * t * t
}

/// 逐字浮起绘制（参照 accompanist-lyrics-ui Simple Float）
/// 已唱字符向上微浮 MAX_FLOAT_OFFSET px，持续 FLOAT_DURATION_MS
pub fn draw_floating_characters(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    layout_x: f64,
    base_y: f64,
    active_color: (f64, f64, f64),
    inactive_color: (f64, f64, f64),
    active_alpha: f64,
    inactive_alpha: f64,
) {
    let n_chars = cached.char_x_offsets.len();
    if n_chars == 0 { return; }

    let chars = match &cached.line.kind {
        LyricLineKind::Verbatim(c) => c,
        _ => return,
    };

    let mut byte_idx: usize = 0;
    for ci in 0..n_chars {
        let ch = &chars[ci];
        let ch_start = ch.start;
        let ch_end = ch_start + ch.duration;
        let ch_len = ch.ch.len();
        let char_x = layout_x + cached.char_x_offsets[ci];
        let char_w = cached.char_widths[ci];
        let vl_idx = cached.char_visual_line[ci];
        let vl_y = base_y + cached.visual_lines[vl_idx].y_offset;
        let vl_h = cached.visual_lines[vl_idx].height;

        // 确定颜色和浮起状态
        let (color, alpha, is_floating) = if current_ms >= ch_end {
            // 已唱完 - 活跃色
            (active_color, active_alpha, false)
        } else if current_ms >= ch_start {
            // 正在唱 - 浮起 + 活跃色
            (active_color, active_alpha, true)
        } else {
            // 未唱 - 非活跃色
            (inactive_color, inactive_alpha, false)
        };

        // 计算浮起偏移
        let float_offset = if is_floating {
            let progress = ((current_ms - ch_start) as f64 / FLOAT_DURATION_MS).clamp(0.0, 1.0);
            // CubicBezier(0, 0, 0.2, 1) 的简单近似：快起慢落
            let eased = 1.0 - (1.0 - progress).powi(3);
            MAX_FLOAT_OFFSET * (1.0 - eased)
        } else {
            0.0
        };

        if float_offset.abs() < 0.01 && !is_floating {
            // 无浮起且非活跃色，跳过（已由第一层 dim 全文覆盖）
            byte_idx += ch_len;
            continue;
        }

        // 裁剪到字符区域
        cr.save().unwrap();
        let clip_x = char_x - 2.0;
        let clip_y = vl_y - MAX_FLOAT_OFFSET - 2.0;
        let clip_w = char_w + 4.0;
        let clip_h = vl_h + MAX_FLOAT_OFFSET + 4.0;
        cr.rectangle(clip_x, clip_y, clip_w, clip_h);
        let _ = cr.clip();

        // 绘制字符（带浮起偏移）
        let draw_y = base_y + (vl_h * 0.8) - float_offset; // 基线在视觉行底部 80% 处
        cr.move_to(layout_x, draw_y);
        cr.set_source_rgba(color.0, color.1, color.2, alpha);
        pangocairo::functions::show_layout(cr, &cached.layout);
        cr.restore().unwrap();

        byte_idx += ch_len;
    }
}

/// 长字动画绘制（参照 accompanist-lyrics-ui "Awesome Animation"）
/// 对 duration ≥ 1000ms 的单词，每个字符有三重叠加效果：
/// 1. 下沉-上浮（DipAndRise）：字符先下沉再上浮
/// 2. 膨胀（Swell）：缩放到 1+MAX_SWELL_SCALE 再回来
/// 3. 弹跳发光（Bounce Glow）：blur 从 0→MAX_BOUNCE_BLUR→0
pub fn draw_long_word_animations(
    cr: &cairo::Context,
    cached: &CachedLine,
    current_ms: u64,
    layout_x: f64,
    base_y: f64,
    fg_color: (f64, f64, f64),
    alpha: f64,
) {
    let chars = match &cached.line.kind {
        LyricLineKind::Verbatim(c) => c,
        _ => return,
    };
    let n_chars = chars.len();
    if n_chars == 0 { return; }

    // 查找当前正在唱的字符索引
    let mut active_char: Option<usize> = None;
    for i in 0..n_chars {
        if current_ms >= chars[i].start && current_ms < chars[i].start + chars[i].duration {
            active_char = Some(i);
            break;
        }
    }
    let Some(active_idx) = active_char else { return };

    // 计算单词边界（连续正在唱的字符）
    let word_start = chars[active_idx].start;
    let mut word_end = chars[active_idx].start + chars[active_idx].duration;
    let mut word_start_idx = active_idx;
    let mut word_end_idx = active_idx;

    // 向前扩展
    while word_start_idx > 0 {
        let prev = word_start_idx - 1;
        if chars[prev].start + chars[prev].duration >= word_start {
            word_start_idx = prev;
        } else {
            break;
        }
    }
    // 向后扩展
    while word_end_idx + 1 < n_chars {
        let next = word_end_idx + 1;
        if chars[next].start <= word_end {
            word_end_idx = next;
            word_end = chars[next].start + chars[next].duration;
        } else {
            break;
        }
    }

    let word_duration = word_end - word_start;
    if word_duration < WORD_ANIM_THRESHOLD_MS as u64 { return; }

    let num_chars_in_word = word_end_idx - word_start_idx + 1;
    let earliest_start = chars[word_start_idx].start;
    let latest_start = chars[word_end_idx].start;

    // 计算动画强度（参照 accompanist）
    let animation_intensity = ((word_duration as f64 - FAST_CHAR_ANIM_THRESHOLD_MS * num_chars_in_word as f64) / 1000.0).max(0.0);
    let dip = (0.5 * animation_intensity).clamp(0.0, 0.5);
    let _swell = (0.1 * animation_intensity).clamp(0.0, 0.1);

    // 动画时长 = 60% 的单词时长
    let awesome_duration = (word_duration as f64 * 0.6).max(100.0);

    for ci in word_start_idx..=word_end_idx {
        let ch = &chars[ci];
        let char_start = ch.start;

        // 字符在单词中的比例（用于交错启动）
        let char_ratio = if latest_start > earliest_start {
            ((char_start - earliest_start) as f64 / (latest_start - earliest_start) as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let awesome_start = earliest_start + ((latest_start - earliest_start) as f64 * char_ratio) as u64;
        if current_ms < awesome_start { continue; }

        let progress = ((current_ms - awesome_start) as f64 / awesome_duration).clamp(0.0, 1.0);

        // 1. 下沉-上浮偏移（Newton 多项式：(0,0), (0.5,-dip), (1.0, rise)）
        let rise = 1.0;
        let dip_rise_offset = MAX_DIP_OFFSET * newton_dip_and_rise(progress, dip, rise);

        let char_x = layout_x + cached.char_x_offsets[ci];
        let char_w = cached.char_widths[ci];
        let vl_idx = cached.char_visual_line[ci];
        let vl_y = base_y + cached.visual_lines[vl_idx].y_offset;
        let vl_h = cached.visual_lines[vl_idx].height;

        // 裁剪到字符区域 + 动画空间
        let clip_x = char_x - MAX_BOUNCE_BLUR - 2.0;
        let clip_y = vl_y - MAX_DIP_OFFSET - MAX_FLOAT_OFFSET - 2.0;
        let clip_w = char_w + MAX_BOUNCE_BLUR * 2.0 + 4.0;
        let clip_h = vl_h + MAX_DIP_OFFSET + MAX_FLOAT_OFFSET + MAX_BOUNCE_BLUR + 4.0;

        cr.save().unwrap();
        cr.rectangle(clip_x, clip_y, clip_w, clip_h);
        let _ = cr.clip();

        // 2. 膨胀缩放（在字符中心缩放）
        let swell_progress = newton_swell(progress);
        let scale = 1.0 + MAX_SWELL_SCALE * swell_progress;
        let cx = char_x + char_w / 2.0;
        let cy = vl_y + vl_h * 0.8;
        cr.translate(cx, cy);
        cr.scale(scale, scale);
        cr.translate(-cx, -cy);

        // 绘制字符
        let draw_y = base_y + vl_h * 0.8 - dip_rise_offset;
        cr.move_to(layout_x, draw_y);
        cr.set_source_rgba(fg_color.0, fg_color.1, fg_color.2, alpha);
        pangocairo::functions::show_layout(cr, &cached.layout);

        cr.restore().unwrap();

        // 3. 弹跳发光（在字符上方绘制模糊光晕）
        let bounce_progress = newton_bounce(progress);
        let blur_alpha = bounce_progress * 0.4;
        if blur_alpha > 0.01 {
            cr.save().unwrap();
            cr.rectangle(clip_x, clip_y, clip_w, clip_h);
            let _ = cr.clip();
            let draw_y = base_y + vl_h * 0.8 - dip_rise_offset;
            cr.move_to(layout_x, draw_y);
            cr.set_source_rgba(fg_color.0, fg_color.1, fg_color.2, alpha * blur_alpha);
            pangocairo::functions::show_layout(cr, &cached.layout);
            cr.restore().unwrap();
        }
    }
}

/// Newton 多项式插值：下沉-上浮曲线 (0,0), (0.5,-dip), (1.0,rise)
fn newton_dip_and_rise(t: f64, dip: f64, rise: f64) -> f64 {
    let f01 = (-dip - 0.0) / 0.5; // -2*dip
    let f12 = (rise - (-dip)) / 0.5; // 2*(rise+dip)
    let f012 = (f12 - f01) / 1.0; // 2*rise+4*dip
    f01 * t + f012 * t * (t - 0.5)
}

/// Newton 多项式插值：膨胀曲线 (0,0), (0.5,swell), (1.0,0)
fn newton_swell(t: f64) -> f64 {
    let swell = 1.0; // 归一化，实际缩放由调用方乘 MAX_SWELL_SCALE
    let f01 = (swell - 0.0) / 0.5; // 2*swell
    let f12 = (0.0 - swell) / 0.5; // -2*swell
    let f012 = (f12 - f01) / 1.0; // -4*swell
    f01 * t + f012 * t * (t - 0.5)
}

/// Newton 多项式插值：弹跳曲线 (0,0), (0.7,1.0), (1.0,0)
fn newton_bounce(t: f64) -> f64 {
    let f01 = (1.0 - 0.0) / 0.7; // 1/0.7
    let f12 = (0.0 - 1.0) / 0.3; // -1/0.3
    let f012 = (f12 - f01) / 1.0;
    (f01 * t + f012 * t * (t - 0.7)).max(0.0)
}

pub fn draw_translation(
    cr: &cairo::Context,
    cached: &CachedLine,
    tl_y: f64,
    widget_w: f64,
    align: LyricAlign,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
) {
    let Some(tl) = &cached.tl_layout else {
        return;
    };
    cr.save().unwrap();
    let x = x_for_layout(widget_w, cached.tl_text_width, align);
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
            let dt = st
                .last_frame_time
                .map(|t| now.duration_since(t).as_secs_f64())
                .unwrap_or(0.016)
                .min(0.1);
            st.last_frame_time = Some(now);

            // 惯性滚动
            if st.is_inertia && !st.user_scrolling {
                st.drag_offset += st.drag_velocity * dt;
                let friction = SCROLL_FRICTION.powf(dt / 0.016);
                st.drag_velocity *= friction;
                if st.drag_velocity.abs() < 5.0 {
                    st.is_inertia = false;
                    st.drag_velocity = 0.0;
                    st.user_scrolling = false;
                }
            }

            // drag_offset 回弹
            if !st.user_scrolling && !st.is_inertia && st.drag_offset.abs() > 0.5 {
                st.drag_offset *= 0.85f64.powf(dt / 0.016);
                if st.drag_offset.abs() < 0.5 {
                    st.drag_offset = 0.0;
                }
            }

            // 如果用户正在手动滚动，不自动滚动
            if !st.user_scrolling && !st.is_inertia {
                let h = widget.height() as f64;
                st.update_line_positions(h);
                let raw = st.active_line_index();
                st.needs_initial_scroll = false;
                st.last_raw_active_idx = raw;
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
            let mut st = state.borrow_mut();
            if let Some(idx) = st.line_at_y(click_y) {
                let target_ms = st.cached_lines[idx].line.start;
                on_seek(target_ms);

                // 手动切换 line_states，绕过防抖
                if let Some(old_idx) = st.last_active_idx {
                    if old_idx < st.line_states.len() {
                        st.line_states[old_idx].set_active(false);
                    }
                }
                if idx < st.line_states.len() {
                    st.line_states[idx].set_active(true);
                }

                st.current_ms = target_ms;
                st.last_active_idx = Some(idx);
                st.last_raw_active_idx = Some(idx);
                st.user_scrolling = false;
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
            st.is_inertia = false;
            st.drag_velocity = 0.0;
            st.last_drag_offset = 0.0;
            st.last_drag_time = None;
        }
    });
    drag_gesture.connect_drag_update({
        let state = state.clone();
        move |_, _offset_x, offset_y| {
            let mut st = state.borrow_mut();
            let delta = -(offset_y - st.last_drag_offset);
            st.drag_offset += delta;
            for s in &mut st.line_states {
                s.snap_y(s.y() + delta);
            }
            let now = Instant::now();
            if let Some(last_time) = st.last_drag_time {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.001 {
                    let vel = -(offset_y - st.last_drag_offset) / dt;
                    st.drag_velocity = vel.clamp(-8000.0, 8000.0);
                }
            }
            st.last_drag_offset = offset_y;
            st.last_drag_time = Some(now);
        }
    });
    drag_gesture.connect_drag_end({
        let state = state.clone();
        move |_, _offset_x, _offset_y| {
            let mut st = state.borrow_mut();
            st.user_scrolling = false;
            if st.drag_velocity.abs() > 80.0 {
                st.is_inertia = true;
            } else {
                st.is_inertia = false;
                st.drag_velocity = 0.0;
            }
        }
    });
    da.add_controller(drag_gesture);

    let scroll_controller =
        gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll_controller.connect_scroll({
        let state = state.clone();
        move |_, _, dy| {
            let mut st = state.borrow_mut();
            let delta = dy * 40.0;
            st.drag_offset += delta;
            for s in &mut st.line_states {
                s.snap_y(s.y() + delta);
            }
            st.user_scrolling = true;
            st.is_inertia = false;
            st.drag_velocity = 0.0;
            // 短暂禁用自动滚动
            let state_clone = state.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                let mut st = state_clone.borrow_mut();
                if !st.is_inertia {
                    st.user_scrolling = false;
                }
            });
            gtk::glib::Propagation::Stop
        }
    });
    da.add_controller(scroll_controller);

    da
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

pub fn fg_color(widget: &impl IsA<gtk::Widget>) -> (f64, f64, f64, f64) {
    let c = widget.style_context().color();
    (
        c.red() as f64,
        c.green() as f64,
        c.blue() as f64,
        c.alpha() as f64,
    )
}

/// 计算 y 位置处的垂直渐隐系数（0..1）
pub fn fade_alpha_for_y(y: f64, h: f64, fade_h: f64) -> f64 {
    let top = if y < fade_h { (y / fade_h).max(0.0) } else { 1.0 };
    let bottom_in = y - (h - fade_h);
    let bottom = if bottom_in > 0.0 { (1.0 - bottom_in / fade_h).max(0.0) } else { 1.0 };
    top * bottom
}

/// 根据对齐方式计算 layout 在 widget 中的 x 起点
pub fn x_for_layout(widget_w: f64, text_w: f64, align: LyricAlign) -> f64 {
    match align {
        LyricAlign::Left => PADDING_H,
        LyricAlign::Center => ((widget_w - text_w) / 2.0).max(PADDING_H),
        LyricAlign::Right => (widget_w - text_w - PADDING_H).max(PADDING_H),
    }
}

pub fn make_layout(
    ctx: &pango::Context,
    size_pt: i32,
    available_width: i32,
    bold: bool,
) -> pango::Layout {
    let layout = pango::Layout::new(ctx);
    let mut desc = pango::FontDescription::new();
    desc.set_family("Sans");
    desc.set_weight(if bold {
        pango::Weight::Bold
    } else {
        pango::Weight::Normal
    });
    desc.set_size(size_pt * pango::SCALE);
    layout.set_font_description(Some(&desc));
    layout.set_width(available_width * pango::SCALE);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout
}

pub fn layout_h(layout: &pango::Layout) -> f64 {
    layout.pixel_size().1 as f64
}

/// 收集 layout 中每条视觉行的字节范围和 y 偏移
fn collect_visual_lines(layout: &pango::Layout) -> Vec<VisualLineInfo> {
    let mut result = Vec::new();
    let mut y_accum = 0.0f64;

    for pango_line in layout.lines_readonly() {
        let byte_start = pango_line.start_index() as usize;
        let byte_len = pango_line.length() as usize;
        let byte_end = byte_start + byte_len;

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
    let mut offsets = Vec::with_capacity(chars.len());
    let mut widths = Vec::with_capacity(chars.len());
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

fn luminance(r: f64, g: f64, b: f64) -> f64 {
    0.299 * r + 0.587 * g + 0.114 * b
}

pub fn dim_color((r, g, b): (f64, f64, f64), bg: (f64, f64, f64)) -> (f64, f64, f64) {
    let (br, bg_c, bb) = bg;
    let t = 0.55;
    let mut dr = r * t + br * (1.0 - t);
    let mut dg = g * t + bg_c * (1.0 - t);
    let mut db = b * t + bb * (1.0 - t);

    let bg_lum = 0.299 * br + 0.587 * bg_c + 0.114 * bb;
    let dim_lum = 0.299 * dr + 0.587 * dg + 0.114 * db;
    let min_diff = 0.18;
    if (dim_lum - bg_lum).abs() < min_diff {
        let offset = if bg_lum > 0.5 { -min_diff } else { min_diff };
        let adjust = offset - (dim_lum - bg_lum);
        dr = (dr + adjust).clamp(0.0, 1.0);
        dg = (dg + adjust).clamp(0.0, 1.0);
        db = (db + adjust).clamp(0.0, 1.0);
    }
    (dr, dg, db)
}
