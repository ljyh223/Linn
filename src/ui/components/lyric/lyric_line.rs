//! 逐行歌词动画状态
//!
//! 每行歌词使用独立的弹簧动画管理垂直位置、水平偏移、缩放、透明度。

use super::spring::{Spring, SpringParams};

/// 垂直位置：参照 AMLL 默认 ζ≈0.83（轻微欠阻尼，平滑有韧性）
const SPRING_SCROLL: SpringParams = SpringParams::new(1.0, 16.0, 100.0);
/// 缩放：ζ≈0.71 微妙过冲，活跃行弹入约0.6%
const SPRING_SCALE: SpringParams = SpringParams::new(2.0, 20.0, 100.0);
/// posY 动态阻尼乘数（AMLL=2.2），使弹簧过阻尼不弹跳
const POS_DAMPING_MULT: f64 = 1.5;

/// 透明度指数平滑参数（非对称 attack/release，AMLL 同款）
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
        }
    }

    /// 设置为活跃行
    pub fn set_active(&mut self, active: bool) {
        if self.is_active == active {
            return;
        }
        self.is_active = active;

        if active {
            self.scale.set_target(1.0);
            self.target_alpha = 1.0;
        } else {
            self.scale.set_target(0.85);
            self.target_alpha = 0.28;
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
        self.pos_y.set_target(y);
    }

    /// 设置垂直目标位置 + 自定义刚度（级联速度差）
    /// damping = sqrt(stiffness) * POS_DAMPING_MULT（AMLL 默认 2.2）
    /// 保持 ζ≈1.1 过阻尼，不弹跳，只有速度差
    pub fn set_target_y_with_stiffness(&mut self, y: f64, stiffness: f64) {
        let damping = stiffness.sqrt() * POS_DAMPING_MULT;
        self.pos_y.set_target_with_params(y, SpringParams::new(1.0, damping, stiffness));
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
