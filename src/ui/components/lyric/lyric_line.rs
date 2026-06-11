//! 逐行歌词动画状态
//!
//! 每行歌词使用独立的弹簧动画管理垂直位置、水平偏移、缩放、透明度。

use super::spring::{Spring, SpringParams};

/// 垂直滚动：欠阻尼（ζ≈0.7），间奏推开时的轻微弹跳
const SPRING_SCROLL: SpringParams = SpringParams::new(1.0, 14.0, 100.0);
/// 缩放：低阻尼（ζ≈0.35），配合主动过冲产生明显弹入
const SPRING_SCALE: SpringParams = SpringParams::new(1.0, 7.0, 100.0);
/// 主动过冲缩放目标（弹簧先冲向此值再切回 1.0）
const SCALE_OVERSHOOT_TARGET: f64 = 1.20;
/// 过冲持续时间（帧数），约 100ms
const BOUNCE_FRAMES: i32 = 6;

/// 透明度指数平滑参数（非对称 attack/release）
const ATTACK_SPEED: f64 = 50.0;
const RELEASE_SPEED: f64 = 7.0;

/// 每行歌词的弹簧动画状态
#[derive(Debug, Clone)]
pub struct LyricLineState {
    /// 垂直位置（像素）
    pub pos_y: Spring,
    /// 缩放（活跃=1.0，非活跃=0.85）
    pub scale: Spring,
    /// 当前透明度（指数平滑，非对称 attack/release）
    pub current_alpha: f64,
    target_alpha: f64,
    /// 是否是当前活跃行
    is_active: bool,
    /// 与活跃行的距离（用于 dim 效果）
    distance: i32,
    /// 缩放弹跳倒计时（帧数），>0 表示正在主动过冲阶段
    scale_bounce_remaining: i32,
    /// 位置弹跳倒计时（帧数），>0 表示位置目标还有偏移
    pos_bounce_remaining: i32,
    /// 位置弹跳偏移量（像素）
    pos_bounce_offset: f64,
}

impl LyricLineState {
    /// 创建新的行状态
    pub fn new(initial_y: f64) -> Self {
        Self {
            pos_y: Spring::new(SPRING_SCROLL, initial_y),
            scale: Spring::new(SPRING_SCALE, 0.85),
            current_alpha: 0.28,
            target_alpha: 0.28,
            is_active: false,
            distance: 0,
            scale_bounce_remaining: 0,
            pos_bounce_remaining: 0,
            pos_bounce_offset: 0.0,
        }
    }

    /// 设置为活跃行
    pub fn set_active(&mut self, active: bool) {
        if self.is_active == active {
            return;
        }
        self.is_active = active;

        if active {
            // 主动过冲：先冲向 1.20，tick 里若干帧后切回 1.0
            self.scale.set_target(SCALE_OVERSHOOT_TARGET);
            self.scale_bounce_remaining = BOUNCE_FRAMES;
            // 位置过冲：下次 set_target_y 会加偏移
            self.pos_bounce_remaining = 1;
            self.pos_bounce_offset = -15.0;
            self.target_alpha = 1.0;
        } else {
            self.scale.set_target(0.85);
            self.target_alpha = 0.28;
            self.scale_bounce_remaining = 0;
            self.pos_bounce_remaining = 0;
        }
    }

    /// 设置与活跃行的距离（用于渐进 dim 效果）
    pub fn set_distance(&mut self, distance: i32) {
        if self.distance == distance {
            return;
        }
        self.distance = distance;

        if self.is_active {
            return;
        }

        // 距离越远，越透明、越小
        let dim_factor = match distance.abs() {
            0 => 0.85,
            1 => 0.45,
            2 => 0.32,
            _ => 0.28,
        };
        self.target_alpha = dim_factor;

        let scale_factor = match distance.abs() {
            0 => 0.97,
            1 => 0.92,
            _ => 0.85,
        };
        self.scale.set_target(scale_factor);
    }

    /// 推进所有弹簧动画和透明度平滑
    pub fn tick(&mut self, dt: f64) {
        self.pos_y.tick(dt);
        self.scale.tick(dt);

        // 缩放弹跳倒计时：过冲阶段结束后切回目标 1.0
        if self.scale_bounce_remaining > 0 {
            self.scale_bounce_remaining -= 1;
            if self.scale_bounce_remaining == 0 {
                self.scale.set_target(1.0);
            }
        }

        // 指数平滑透明度（非对称 attack/release）
        let speed = if self.target_alpha > self.current_alpha {
            ATTACK_SPEED
        } else {
            RELEASE_SPEED
        };
        let factor = 1.0 - (-speed * dt).exp();
        self.current_alpha += (self.target_alpha - self.current_alpha) * factor;
    }

    /// 强制设置垂直位置
    pub fn snap_y(&mut self, y: f64) {
        self.pos_y.snap_to(y);
    }

    /// 设置垂直目标位置
    pub fn set_target_y(&mut self, y: f64) {
        if self.pos_bounce_remaining > 0 {
            self.pos_bounce_remaining -= 1;
            self.pos_y.set_target(y + self.pos_bounce_offset);
        } else {
            self.pos_y.set_target(y);
        }
    }

    /// 获取当前垂直位置
    pub fn y(&self) -> f64 {
        self.pos_y.current_position
    }

    /// 获取当前缩放
    pub fn scale(&self) -> f64 {
        self.scale.current_position
    }

    /// 是否所有弹簧都已到达目标
    pub fn arrived(&self) -> bool {
        self.pos_y.arrived() && self.scale.arrived()
    }
}
