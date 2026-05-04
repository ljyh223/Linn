//! 间奏呼吸圆点
//!
//! 参考 accompanist-lyrics-ui 的 KaraokeBreathingDots 5 阶段设计：
//!   Intro(3s) → Breathing → PreExit(3s) → Still(0.2s) → Outro(0.2s)
//!
//! 三圆点平分间奏时间，逐点波次变亮；整体余弦呼吸缩放；
//! 进入期水平揭示；退场前一次完整余弦摆荡（"提前加速呼吸"）。

use relm4::gtk::cairo;
use std::f64::consts::TAU;

// ─── 时间参数（accompanist 默认）───────────────────────────────────────────────

/// 进入期（淡入 + 水平揭示）
const ENTER_MS: u64 = 3000;
/// 预退场余弦摆荡
const PRE_EXIT_DIP_MS: u64 = 3000;
/// 静止保持
const PRE_EXIT_STILL_MS: u64 = 200;
/// 退场淡出
const EXIT_MS: u64 = 200;

/// 余弦呼吸完整周期（秒）
const BREATH_PERIOD_S: f64 = 3.0;

/// 间奏检测阈值（毫秒）
const INTERLUDE_THRESHOLD_MS: u64 = 2000;

/// 推开下方歌词的高度（像素）
const PUSH_HEIGHT: f64 = 44.0;

/// 推挤动画平滑参数（非对称 attack/release）
const PUSH_ATTACK: f64 = 50.0;
const PUSH_RELEASE: f64 = 7.0;

// ─── 视觉参数 ──────────────────────────────────────────────────────────────────

const DOT_RADIUS: f64 = 4.0;
const DOT_SPACING: f64 = 16.0;
const DOT_LEFT_MARGIN: f64 = 24.0; // 与 lyric_widget PADDING_H 对齐

// ─── 缓动函数 ──────────────────────────────────────────────────────────────────

/// easeInOutCubic: 缓入缓出三次曲线
fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// ─── 歌词行信息 ────────────────────────────────────────────────────────────────

pub struct LyricLineInfo {
    pub start: u64,
    pub duration: u64,
}

// ─── 内部阶段枚举 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum DotStage {
    /// 进入期：淡入 + 水平揭示 + 缩放 0→0.8
    Intro,
    /// 呼吸期：余弦振荡 0.8↔1.0，逐点波次变亮
    Breathing,
    /// 预退场：一次完整余弦摆荡 ↓↑（"提前加速呼吸"）
    PreExit,
    /// 静止：保持满缩放满透明
    Still,
    /// 退场：淡出 + 缩放 →0
    Outro,
}

// ─── 间奏动画状态 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InterludeDots {
    // 外部接口
    pub visible: bool,
    /// 间隙前一行索引（间隙在 idx 和 idx+1 之间）
    pub interlude_idx: Option<usize>,
    /// 推挤量（指数平滑）
    pub push_amount: f64,

    // 推挤目标
    push_target: f64,

    // 时间线
    interlude_start: u64,
    interlude_end: u64,
    enter_end: u64,
    dip_start: u64,
    still_start: u64,
    exit_start: u64,

    // 呼吸震荡计时器
    breath_time: f64,
}

impl InterludeDots {
    pub fn new() -> Self {
        Self {
            visible: false,
            interlude_idx: None,
            push_amount: 0.0,
            push_target: 0.0,

            interlude_start: 0,
            interlude_end: 0,
            enter_end: 0,
            dip_start: 0,
            still_start: 0,
            exit_start: 0,

            breath_time: 0.0,
        }
    }

    // ── 检测 ──────────────────────────────────────────────────────────────────

    /// 检测间奏区间并更新时间线
    pub fn detect(&mut self, lines: &[LyricLineInfo], current_ms: u64) {
        let was_visible = self.visible;
        self.visible = false;
        self.interlude_idx = None;

        // 开头间奏（歌曲开始到第一句歌词，gap ≥ 阈值）
        if !lines.is_empty() {
            let first_start = lines[0].start;
            if current_ms < first_start && first_start >= INTERLUDE_THRESHOLD_MS {
                self.visible = true;
                self.interlude_idx = None; // None = 开头间奏
                self.interlude_start = 0;
                self.interlude_end = first_start;
            }
        }

        // 行间间奏
        if !self.visible {
            for i in 0..lines.len().saturating_sub(1) {
                let line_end = lines[i].start + lines[i].duration;
                let next_start = lines[i + 1].start;
                if next_start > line_end && next_start - line_end > INTERLUDE_THRESHOLD_MS {
                    if current_ms >= line_end && current_ms < next_start {
                        self.visible = true;
                        self.interlude_idx = Some(i);
                        self.interlude_start = line_end;
                        self.interlude_end = next_start;
                        break;
                    }
                }
            }
        }

        // 间奏切换
        if self.visible && !was_visible {
            self.compute_timeline();
            self.push_target = PUSH_HEIGHT;
            self.breath_time = 0.0;
        } else if !self.visible && was_visible {
            self.push_target = 0.0;
        }
    }

    /// 自适应时间线：间隙 < 6400ms 则等比压缩所有阶段
    fn compute_timeline(&mut self) {
        let gap = self.interlude_end.saturating_sub(self.interlude_start);
        let fixed_total = ENTER_MS + PRE_EXIT_DIP_MS + PRE_EXIT_STILL_MS + EXIT_MS;
        let factor = if (gap as f64) < (fixed_total as f64) {
            gap as f64 / fixed_total as f64
        } else {
            1.0
        };

        let enter = (ENTER_MS as f64 * factor) as u64;
        let dip = (PRE_EXIT_DIP_MS as f64 * factor) as u64;
        let still = (PRE_EXIT_STILL_MS as f64 * factor) as u64;
        let exit = (EXIT_MS as f64 * factor) as u64;

        self.enter_end = self.interlude_start + enter;
        self.exit_start = self.interlude_end.saturating_sub(exit);
        self.still_start = self.exit_start.saturating_sub(still);
        self.dip_start = self.still_start.saturating_sub(dip);
    }

    // ── 推进 ──────────────────────────────────────────────────────────────────

    /// 推进呼吸震荡 + 推挤平滑
    pub fn tick(&mut self, dt: f64) {
        if self.visible {
            self.breath_time += dt;
        }

        // 推挤动画：指数平滑
        let speed = if self.push_target > self.push_amount {
            PUSH_ATTACK
        } else {
            PUSH_RELEASE
        };
        let factor = 1.0 - (-speed * dt).exp();
        self.push_amount += (self.push_target - self.push_amount) * factor;
        if (self.push_amount - self.push_target).abs() < 0.1 {
            self.push_amount = self.push_target;
        }
    }

    // ── 绘制 ──────────────────────────────────────────────────────────────────

    pub fn draw(
        &self,
        cr: &cairo::Context,
        center_y: f64,
        _widget_w: f64,
        current_ms: u64,
        (r, g, b): (f64, f64, f64),
    ) {
        if !self.visible {
            return;
        }

        let stage = self.current_stage(current_ms);
        let (alpha, scale, reveal) = self.stage_params(current_ms, stage);

        if alpha < 0.001 {
            return;
        }

        // 三圆点总包围盒（左对齐，与歌词文本左边距一致）
        let total_w = 2.0 * DOT_RADIUS + 2.0 * DOT_SPACING; // ~40px
        let total_h = 2.0 * DOT_RADIUS;
        let base_x = DOT_LEFT_MARGIN;

        cr.save().unwrap();

        // 组缩放（以圆点行中心为 pivot）
        if (scale - 1.0).abs() > 0.001 {
            let cx = base_x + total_w / 2.0;
            cr.translate(cx, center_y);
            cr.scale(scale, scale);
            cr.translate(-cx, -center_y);
        }

        // 进入期水平揭示 clip
        if stage == DotStage::Intro {
            let reveal_w = reveal * (total_w + total_w * 0.5);
            let clip_left = base_x - DOT_RADIUS;
            cr.rectangle(clip_left, center_y - DOT_RADIUS, reveal_w, total_h);
            let _ = cr.clip();
        }

        // 逐点绘制（arc+fill 后路径自动清空，无需逐点 save/restore）
        for i in 0..3 {
            let dot_alpha = self.dot_alpha(i, current_ms, stage);
            let final_alpha = dot_alpha * alpha;

            if final_alpha < 0.005 {
                continue;
            }

            let cx = base_x + i as f64 * DOT_SPACING;
            cr.set_source_rgba(r, g, b, final_alpha * 0.6);
            cr.arc(cx, center_y, DOT_RADIUS, 0.0, TAU);
            cr.fill().unwrap();
        }

        cr.restore().unwrap();
    }

    // ── 阶段判定 ──────────────────────────────────────────────────────────────

    fn current_stage(&self, current_ms: u64) -> DotStage {
        if current_ms < self.enter_end {
            DotStage::Intro
        } else if current_ms < self.dip_start {
            DotStage::Breathing
        } else if current_ms < self.still_start {
            DotStage::PreExit
        } else if current_ms < self.exit_start {
            DotStage::Still
        } else {
            DotStage::Outro
        }
    }

    /// 返回 (总体透明度, 组缩放, 揭示进度)
    fn stage_params(&self, current_ms: u64, stage: DotStage) -> (f64, f64, f64) {
        match stage {
            DotStage::Intro => {
                let progress = if self.enter_end > self.interlude_start {
                    (current_ms - self.interlude_start) as f64
                        / (self.enter_end - self.interlude_start) as f64
                } else {
                    1.0
                }
                .clamp(0.0, 1.0);
                let eased = ease_in_out_cubic(progress);
                (eased, eased * 0.8, eased)
            }
            DotStage::Breathing => {
                let angle = (self.breath_time / BREATH_PERIOD_S) * TAU;
                (1.0, 0.9 - 0.1 * angle.cos(), 1.0)
            }
            DotStage::PreExit => {
                if self.still_start > self.dip_start {
                    let progress = (current_ms - self.dip_start) as f64
                        / (self.still_start - self.dip_start) as f64;
                    let scale = 0.8 + 0.2 * (progress * TAU).cos();
                    (1.0, scale, 1.0)
                } else {
                    (1.0, 1.0, 1.0)
                }
            }
            DotStage::Still => (1.0, 1.0, 1.0),
            DotStage::Outro => {
                let progress = if self.interlude_end > self.exit_start {
                    (self.interlude_end - current_ms) as f64
                        / (self.interlude_end - self.exit_start) as f64
                } else {
                    0.0
                }
                .clamp(0.0, 1.0);
                let eased = ease_in_out_cubic(progress);
                (eased, eased, 1.0)
            }
        }
    }

    /// 逐点波次 alpha：平分呼吸时间，每点独立线性变亮 (0.4 → 1.0)
    fn dot_alpha(&self, index: usize, current_ms: u64, stage: DotStage) -> f64 {
        if stage == DotStage::Intro {
            // 进入期基值 0.4，由 stage alpha 控制整体淡入
            return 0.4;
        }

        let breath_start = self.enter_end;
        let breath_end = self.dip_start;
        if breath_end <= breath_start {
            return 1.0;
        }
        let breath_dur = (breath_end - breath_start) as f64;

        // 每个点独占 breath_dur / 3 的窗口
        let per_dot_dur = breath_dur / 3.0;
        let dot_start = breath_start as f64 + index as f64 * per_dot_dur;

        if current_ms < self.exit_start {
            // 呼吸期 / 预退场 / 静止：当前时间已超过该点窗口则保持 1.0
            let t = (current_ms as f64 - dot_start) / per_dot_dur;
            (t.clamp(0.0, 1.0) * 0.6 + 0.4).min(1.0)
        } else {
            // 退场期：所有点一起淡出
            1.0
        }
    }

    // ── 推挤 ──────────────────────────────────────────────────────────────────

    pub fn reset(&mut self) {
        self.visible = false;
        self.interlude_idx = None;
        self.push_amount = 0.0;
        self.push_target = 0.0;
        self.interlude_start = 0;
        self.interlude_end = 0;
        self.enter_end = 0;
        self.dip_start = 0;
        self.still_start = 0;
        self.exit_start = 0;
        self.breath_time = 0.0;
    }

    pub fn snap_push(&mut self) {
        self.push_amount = self.push_target;
    }
}
