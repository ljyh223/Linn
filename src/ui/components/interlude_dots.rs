//! 间奏动画点
//!
//! 当两行歌词间隔 > 2 秒时，显示三个呼吸缩放的圆点动画。

use relm4::gtk::cairo;

/// 间奏检测阈值（毫秒）
const INTERLUDE_THRESHOLD_MS: u64 = 2000;

/// 圆点半径
const DOT_RADIUS: f64 = 4.0;

/// 圆点间距
const DOT_SPACING: f64 = 16.0;

/// 动画周期（秒）
const BREATH_CYCLE: f64 = 1.5;

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

/// 间奏动画状态
#[derive(Debug, Clone)]
pub struct InterludeDots {
    /// 是否正在显示间奏动画
    pub visible: bool,
    /// 动画时间（秒）
    time: f64,
    /// 间奏开始时间（毫秒）
    interlude_start: u64,
    /// 间奏结束时间（毫秒）
    interlude_end: u64,
    /// 动画进度（0..1）
    progress: f64,
}

impl InterludeDots {
    pub fn new() -> Self {
        Self {
            visible: false,
            time: 0.0,
            interlude_start: 0,
            interlude_end: 0,
            progress: 0.0,
        }
    }

    /// 检测间奏区间并更新状态
    /// `lines` 是所有歌词行信息，必须按 start 排序
    /// 使用行结束时间（start + duration）而非 start 来计算间隙
    pub fn detect(&mut self, lines: &[LyricLineInfo], current_ms: u64) {
        self.visible = false;

        for i in 0..lines.len().saturating_sub(1) {
            let line_end = lines[i].start + lines[i].duration;
            let next_start = lines[i + 1].start;
            // 只有当前行结束后，下一行才开始，且间隙 > 阈值
            if next_start > line_end && next_start - line_end > INTERLUDE_THRESHOLD_MS {
                if current_ms >= line_end && current_ms < next_start {
                    self.visible = true;
                    self.interlude_start = line_end;
                    self.interlude_end = next_start;
                    self.progress = ((current_ms - line_end) as f64
                        / (next_start - line_end) as f64)
                        .clamp(0.0, 1.0);
                    return;
                }
            }
        }
    }

    /// 推进动画时间
    pub fn tick(&mut self, dt: f64) {
        if self.visible {
            self.time += dt;
            if self.time > BREATH_CYCLE * 3.0 {
                self.time -= BREATH_CYCLE * 3.0;
            }
        }
    }

    /// 绘制间奏圆点
    /// `center_y` 是间奏动画在 widget 中的 y 坐标
    /// `widget_w` 是 widget 宽度
    /// `(r, g, b)` 是前景颜色
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

            // 呼吸缩放
            let scale = ease_in_out_back(if cycle_pos < 0.5 {
                cycle_pos * 2.0
            } else {
                2.0 - cycle_pos * 2.0
            });

            // 入场/出场缓动
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
    }
}
