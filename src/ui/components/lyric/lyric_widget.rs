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
use super::spring::{Spring, SpringParams};

// ─── 样式常量 ──────────────────────────────────────────────────────────────────

const ALPHA_ACTIVE: f64 = 1.0;
const ALPHA_DIM: f64 = 0.24;
const FONT_SIZE_PT: i32 = 20;
const FONT_SIZE_TL_PT: i32 = 13;
const GRADIENT_EDGE_PX: f64 = 50.0; // ~fontSize * 0.6, 2 * 此值 = 过渡区总宽
const LINE_SPACING: f64 = 20.0; // 歌词句间距
const TL_GAP: f64 = 3.0; // 主歌词与翻译间距
const PADDING_H: f64 = 24.0; // 左右内边距
const ACTIVE_LINE_RATIO: f64 = 0.32;
const LINE_SWITCH_DEBOUNCE_MS: u64 = 120;
const TOP_PADDING: f64 = 48.0; // 顶部留白，避免第一行贴边
const FADE_HEIGHT: f64 = 60.0; // 顶部/底部渐隐高度

// 动态弹簧刚度参数，参照 AMLL 的自适应弹簧
const MIN_INTERVAL: f64 = 100.0;
const MAX_INTERVAL: f64 = 800.0;
const MIN_STIFFNESS: f64 = 100.0;
const MAX_STIFFNESS: f64 = 180.0;
const DAMPING_RATIO: f64 = 1.0;

// 滚动惯性参数
const SCROLL_FRICTION: f64 = 0.95; // 每帧摩擦系数（基于 16ms）

// 垂直滚动弹簧参数（临界阻尼，无振荡）
const SCROLL_SPRING: SpringParams = SpringParams::new(1.0, 20.0, 100.0);

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
    /// 垂直滚动弹簧（替代原来的指数平滑）
    scroll_spring: Spring,
    /// 每行的动画状态
    line_states: Vec<LyricLineState>,
    /// 间奏动画
    interlude_dots: InterludeDots,
    last_frame_time: Option<Instant>,
    /// 上一次的活跃行索引（防抖后的，用于渲染切换）
    last_active_idx: Option<usize>,
    /// 上一帧的原始活跃行（时间线，用于滚动更新检测）
    last_raw_active_idx: Option<usize>,
    /// 用户正在手动拖拽滚动
    user_scrolling: bool,
    /// 拖拽开始时的滚动位置
    drag_start_scroll: f64,
    /// 首次加载后需触发一次滚动定位
    needs_initial_scroll: bool,
    /// 文字颜色覆写（全屏模式下强制白色）
    text_color_override: Option<(f64, f64, f64, f64)>,
    /// 为活跃行绘制文字阴影以增强对比度
    pub enable_shadow: bool,
    /// 缓存每行歌词的垂直位置
    cached_y_positions: Vec<f64>,
    line_infos: Vec<LyricLineInfo>,
    bg_color: (f64, f64, f64),
    /// 滚动惯性速度（像素/秒）
    scroll_velocity: f64,
    /// 是否正在惯性滚动
    is_decelerating: bool,
    /// 上一次拖拽偏移量，用于计算拖拽速度
    last_drag_offset: f64,
    /// 上一次拖拽时间，用于计算拖拽速度
    last_drag_time: Option<Instant>,
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
            last_raw_active_idx: None,
            user_scrolling: false,
            drag_start_scroll: 0.0,
            needs_initial_scroll: false,
            text_color_override: None,
            enable_shadow: false,
            cached_y_positions: Vec::new(),
            line_infos: Vec::new(),
            bg_color: (0.0, 0.0, 0.0),
            scroll_velocity: 0.0,
            is_decelerating: false,
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

        self.scroll_spring.snap_to(0.0);
        self.current_ms = 0;
        self.last_frame_time = None;
        self.last_active_idx = None;
        self.last_raw_active_idx = None;
        self.needs_initial_scroll = true;
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

    fn update_scroll_target(&mut self, widget_h: f64, active_idx: usize) {
        let positions = &self.cached_y_positions;
        if let Some(&line_y) = positions.get(active_idx) {
            let lh = self.cached_lines[active_idx].layout_height;
            let target = line_y + lh / 2.0 - widget_h * ACTIVE_LINE_RATIO;

            // 动态弹簧刚度：根据当前行与上一行的时间间隔调整
            if active_idx > 0 {
                let interval = (self.cached_lines[active_idx].line.start
                    - self.cached_lines[active_idx - 1].line.start) as f64;
                let clamped = interval.clamp(MIN_INTERVAL, MAX_INTERVAL);
                let ratio = 1.0 - (clamped - MIN_INTERVAL) / (MAX_INTERVAL - MIN_INTERVAL);
                let stiffness = MIN_STIFFNESS + ratio.powf(0.2) * (MAX_STIFFNESS - MIN_STIFFNESS);
                let damping = stiffness.sqrt() * DAMPING_RATIO;
                self.scroll_spring.set_target_with_params(target, SpringParams::new(1.0, damping, stiffness));
            } else {
                // 第一行，使用默认弹簧参数
                self.scroll_spring.set_target_with_params(target, SCROLL_SPRING);
            }
        }
    }

    /// Seek 进间奏区间时，滚动到间奏点位置
    fn update_scroll_for_interlude(&mut self, widget_h: f64) {
        let positions = &self.cached_y_positions;
        let push = self.interlude_dots.push_amount;
        let target = match self.interlude_dots.interlude_idx {
            Some(pi) if pi + 1 < positions.len() => {
                let bottom = positions[pi] + self.cached_lines[pi].total_height;
                let top_next = positions[pi + 1] + push;
                (bottom + top_next) / 2.0 - widget_h * ACTIVE_LINE_RATIO
            }
            _ => TOP_PADDING + push / 2.0 - widget_h * ACTIVE_LINE_RATIO,
        };
        self.scroll_spring.set_target(target);
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
        let raw_active = self.active_line_index();

        // 防抖：正向播放时延迟行切换，避免行边界处抖动
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

        // 检测活跃行切换
        if active_idx != self.last_active_idx {
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

        // 计算每行的目标 y 位置和距离（间奏推挤）
        let positions = &self.cached_y_positions;
        let push = self.interlude_dots.push_amount;
        let push_idx = self.interlude_dots.interlude_idx;
        let visible = self.interlude_dots.visible;
        for (i, state) in self.line_states.iter_mut().enumerate() {
            let mut y = positions[i];
            if visible {
                match push_idx {
                    Some(pi) if i > pi => y += push,
                    None => y += push, // 开头间奏：所有行都推
                    _ => {}
                }
            }
            state.set_target_y(y);
            if let Some(ai) = active_idx {
                state.set_distance(i as i32 - ai as i32);
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
    // 绘制使用防抖后的行索引，避免边界闪烁
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
        let line_y = line_state.y() - scroll_y;

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
                let bottom = state.line_states[pi].y() - scroll_y
                    + state.cached_lines[pi].total_height;
                let top_next = state.line_states[pi + 1].y() - scroll_y;
                let dot_y = (bottom + top_next) / 2.0;
                let dot_fade = fade_alpha_for_y(dot_y, h, FADE_HEIGHT);
                state.interlude_dots.draw(cr, dot_y, w, state.current_ms, (fr * dot_fade, fg * dot_fade, fb * dot_fade));
            }
        } else if !state.line_states.is_empty() {
            let push = state.interlude_dots.push_amount;
            let dot_y = TOP_PADDING + push / 2.0 - scroll_y;
            let dot_fade = fade_alpha_for_y(dot_y, h, FADE_HEIGHT);
            state.interlude_dots.draw(cr, dot_y, w, state.current_ms, (fr * dot_fade, fg * dot_fade, fb * dot_fade));
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

fn draw_active_line(
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

/// 逐字渐变绘制：逐视觉行独立 clip，修复多行高亮 bug
fn draw_active_verbatim(
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
            // let right = cached.char_x_offsets[last_char] + cached.char_widths[last_char];
            // Some(right)
            None
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
        grad.add_color_stop_rgba(0.0, r, g, b, fa * ALPHA_ACTIVE);
        grad.add_color_stop_rgba(0.6, r, g, b, fa * ALPHA_ACTIVE);
        grad.add_color_stop_rgba(1.0, dim_r, dim_g, dim_b, fa);

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
}

/// easeInOutCubic: 缓入缓出三次曲线
fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn draw_translation(
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
            if st.is_decelerating {
                let new_pos = st.scroll_spring.current_position + st.scroll_velocity * dt;
                st.scroll_spring.snap_to(new_pos);
                st.scroll_spring.set_target(new_pos);
                let friction = SCROLL_FRICTION.powf(dt / 0.016);
                st.scroll_velocity *= friction;
                if st.scroll_velocity.abs() < 5.0 {
                    st.is_decelerating = false;
                    st.scroll_velocity = 0.0;
                    // 惯性结束后恢复自动滚动
                    st.user_scrolling = false;
                }
            }

            // 如果用户正在手动滚动，不自动滚动
            if !st.user_scrolling && !st.is_decelerating {
                st.update_line_states();
                let raw = st.active_line_index();
                if st.needs_initial_scroll || raw != st.last_raw_active_idx {
                    st.needs_initial_scroll = false;
                    st.last_raw_active_idx = raw;
                    let h = widget.height() as f64;
                    if let Some(idx) = raw {
                        st.update_scroll_target(h, idx);
                    } else if st.interlude_dots.visible {
                        st.update_scroll_for_interlude(h);
                    }
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
            st.is_decelerating = false;
            st.scroll_velocity = 0.0;
            st.last_drag_offset = 0.0;
            st.last_drag_time = None;
            st.drag_start_scroll = st.scroll_spring.current_position;
        }
    });
    drag_gesture.connect_drag_update({
        let state = state.clone();
        move |_, _offset_x, offset_y| {
            let mut st = state.borrow_mut();
            let new_scroll = st.drag_start_scroll - offset_y;
            st.scroll_spring.snap_to(new_scroll);
            st.scroll_spring.set_target(new_scroll);
            let now = Instant::now();
            if let Some(last_time) = st.last_drag_time {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.001 {
                    let delta = -(offset_y - st.last_drag_offset);
                    st.scroll_velocity = (delta / dt).clamp(-8000.0, 8000.0);
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
            if st.scroll_velocity.abs() > 80.0 {
                st.is_decelerating = true;
            } else {
                st.user_scrolling = false;
                st.scroll_velocity = 0.0;
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
            let current = st.scroll_spring.current_position;
            let delta = dy * 40.0;
            st.scroll_spring.snap_to(current + delta);
            st.scroll_spring.set_target(current + delta);
            st.user_scrolling = true;
            st.is_decelerating = false;
            st.scroll_velocity = 0.0;
            // 短暂禁用自动滚动
            let state_clone = state.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                let mut st = state_clone.borrow_mut();
                if !st.is_decelerating {
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

fn fg_color(widget: &DrawingArea) -> (f64, f64, f64, f64) {
    let c = widget.style_context().color();
    (
        c.red() as f64,
        c.green() as f64,
        c.blue() as f64,
        c.alpha() as f64,
    )
}

/// 计算 y 位置处的垂直渐隐系数（0..1）
fn fade_alpha_for_y(y: f64, h: f64, fade_h: f64) -> f64 {
    let top = if y < fade_h { (y / fade_h).max(0.0) } else { 1.0 };
    let bottom_in = y - (h - fade_h);
    let bottom = if bottom_in > 0.0 { (1.0 - bottom_in / fade_h).max(0.0) } else { 1.0 };
    top * bottom
}

/// 根据对齐方式计算 layout 在 widget 中的 x 起点
fn x_for_layout(widget_w: f64, text_w: f64, align: LyricAlign) -> f64 {
    match align {
        LyricAlign::Left => PADDING_H,
        LyricAlign::Center => ((widget_w - text_w) / 2.0).max(PADDING_H),
        LyricAlign::Right => (widget_w - text_w - PADDING_H).max(PADDING_H),
    }
}

fn make_layout(
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

fn layout_h(layout: &pango::Layout) -> f64 {
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

fn dim_color((r, g, b): (f64, f64, f64), bg: (f64, f64, f64)) -> (f64, f64, f64) {
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
