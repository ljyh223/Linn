//! 间奏动画点
//!
//! 当两行歌词间隔 > 2 秒时，用指数平滑推开下方歌词，
//! 在间隙中显示三个呼吸缩放的圆点（冒泡效果）。

use relm4::gtk::cairo;

/// 间奏检测阈值（毫秒）
const INTERLUDE_THRESHOLD_MS: u64 = 2000;

/// 推开高度（间奏点占据的空间）
const PUSH_HEIGHT: f64 = 44.0;

/// 圆点半径
const DOT_RADIUS: f64 = 4.0;

/// 圆点间距
const DOT_SPACING: f64 = 16.0;

/// 动画周期（秒）
const BREATH_CYCLE: f64 = 1.5;

/// 推挤动画平滑参数（非对称 attack/release）
const PUSH_ATTACK: f64 = 50.0;
const PUSH_RELEASE: f64 = 7.0;

/// 缓动函数：easeOutExpo
fn ease_out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * t)
    }
}

/// 缓动函数：easeInOutBack
fn ease_in_out_back(t: f64) -> f64 {
    let c1 = 1.70158;
    let c2 = c1 * 1.525;
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
    }
}

/// 歌词行信息（用于间奏检测）
pub struct LyricLineInfo {
    pub start: u64,
    pub duration: u64,
}

/// 间奏动画状态（呼吸点 + 推挤下方行）
#[derive(Debug, Clone)]
pub struct InterludeDots {
    /// 是否正在间奏中（可见）
    pub visible: bool,
    /// 呼吸动画时间（秒）
    time: f64,
    /// 间奏开始时间（毫秒，= 上行 line_end）
    interlude_start: u64,
    /// 间奏结束时间（毫秒，= 下行 start）
    interlude_end: u64,
    /// 呼吸动画进度 (0..1)
    progress: f64,

    // ── 推挤动画 ──
    /// 前一行索引（间隙在 idx 和 idx+1 之间）
    pub interlude_idx: Option<usize>,
    /// 当前推挤偏移量（像素）
    pub push_amount: f64,
    push_target: f64,
}

impl InterludeDots {
    pub fn new() -> Self {
        Self {
            visible: false,
            time: 0.0,
            interlude_start: 0,
            interlude_end: 0,
            progress: 0.0,
            interlude_idx: None,
            push_amount: 0.0,
            push_target: 0.0,
        }
    }

    /// 检测间奏区间并更新状态
    pub fn detect(&mut self, lines: &[LyricLineInfo], current_ms: u64) {
        let was_visible = self.visible;
        self.visible = false;
        self.interlude_idx = None;

        for i in 0..lines.len().saturating_sub(1) {
            let line_end = lines[i].start + lines[i].duration;
            let next_start = lines[i + 1].start;
            if next_start > line_end && next_start - line_end > INTERLUDE_THRESHOLD_MS {
                if current_ms >= line_end && current_ms < next_start {
                    self.visible = true;
                    self.interlude_idx = Some(i);
                    self.interlude_start = line_end;
                    self.interlude_end = next_start;
                    self.progress = ((current_ms - line_end) as f64
                        / (next_start - line_end) as f64)
                        .clamp(0.0, 1.0);
                    break;
                }
            }
        }

        // 间奏状态切换：设置推挤目标
        if self.visible && !was_visible {
            self.push_target = PUSH_HEIGHT;
        } else if !self.visible && was_visible {
            self.push_target = 0.0;
        }
    }

    /// 推进呼吸动画 + 推挤平滑
    pub fn tick(&mut self, dt: f64) {
        // 呼吸动画
        if self.visible {
            self.time += dt;
            if self.time > BREATH_CYCLE * 3.0 {
                self.time -= BREATH_CYCLE * 3.0;
            }
        }

        // 推挤动画：指数平滑
        let speed = if self.push_target > self.push_amount {
            PUSH_ATTACK
        } else {
            PUSH_RELEASE
        };
        let factor = 1.0 - (-speed * dt).exp();
        self.push_amount += (self.push_target - self.push_amount) * factor;

        // 收敛后直接吸附
        if (self.push_amount - self.push_target).abs() < 0.1 {
            self.push_amount = self.push_target;
        }
    }

    /// 绘制间奏圆点
    pub fn draw(
        &self,
        cr: &cairo::Context,
        center_y: f64,
        widget_w: f64,
        (r, g, b): (f64, f64, f64),
    ) {
        if !self.visible {
            return;
        }

        let base_x = widget_w / 2.0 - DOT_SPACING;

        for i in 0..3 {
            let dot_time = self.time + i as f64 * BREATH_CYCLE * 0.33;
            let cycle_pos = (dot_time % BREATH_CYCLE) / BREATH_CYCLE;

            let scale = ease_in_out_back(if cycle_pos < 0.5 {
                cycle_pos * 2.0
            } else {
                2.0 - cycle_pos * 2.0
            });

            let fade_in = ease_out_expo((self.progress * 3.0 - i as f64 * 0.5).clamp(0.0, 1.0));
            let fade_out = ease_out_expo(((1.0 - self.progress) * 3.0 - i as f64 * 0.5).clamp(0.0, 1.0));
            let alpha = (fade_in * fade_out).clamp(0.0, 1.0);

            let x = base_x + i as f64 * DOT_SPACING;
            let radius = DOT_RADIUS * (0.5 + scale * 0.5);

            cr.save().unwrap();
            cr.set_source_rgba(r, g, b, alpha * 0.6);
            cr.arc(x, center_y, radius, 0.0, std::f64::consts::TAU);
            cr.fill().unwrap();
            cr.restore().unwrap();
        }
    }

    /// 重置动画状态
    pub fn reset(&mut self) {
        self.visible = false;
        self.time = 0.0;
        self.progress = 0.0;
        self.interlude_idx = None;
        self.push_amount = 0.0;
        self.push_target = 0.0;
    }

    /// 吸附推挤量到目标（首次加载时避免动画延迟）
    pub fn snap_push(&mut self) {
        self.push_amount = self.push_target;
    }
}
