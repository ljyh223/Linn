//! 逐行歌词动画状态
//!
//! 每行歌词使用独立的弹簧动画管理垂直位置、水平偏移、缩放、透明度。

use super::spring::{Spring, SpringParams};

/// 弹簧参数预设（临界/过阻尼，无振荡）
/// 垂直滚动：临界阻尼
const SPRING_SCROLL: SpringParams = SpringParams::new(1.0, 20.0, 100.0);
/// 缩放：过阻尼
const SPRING_SCALE: SpringParams = SpringParams::new(2.0, 30.0, 100.0);
/// 水平偏移：mass=1, damping=10, stiffness=100
const SPRING_OFFSET: SpringParams = SpringParams::new(1.0, 10.0, 100.0);
/// 透明度：mass=1, damping=20, stiffness=120
const SPRING_OPACITY: SpringParams = SpringParams::new(1.0, 20.0, 120.0);

/// 每行歌词的弹簧动画状态
#[derive(Debug, Clone)]
pub struct LyricLineState {
    /// 垂直位置（像素）
    pub pos_y: Spring,
    /// 水平偏移（活跃行强调效果）
    pub pos_x: Spring,
    /// 缩放（活跃=1.0，非活跃=0.9）
    pub scale: Spring,
    /// 透明度（活跃=1.0，非活跃=0.28）
    pub opacity: Spring,
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
            pos_x: Spring::new(SPRING_OFFSET, 0.0),
            scale: Spring::new(SPRING_SCALE, 0.9),
            opacity: Spring::new(SPRING_OPACITY, 0.28),
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
            self.opacity.set_target(1.0);
            self.pos_x.set_target(16.0); // 活跃行向右偏移
        } else {
            self.scale.set_target(0.9);
            self.opacity.set_target(0.28);
            self.pos_x.set_target(0.0);
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
            0 => 1.0,
            1 => 0.5,
            2 => 0.35,
            _ => 0.28,
        };
        self.opacity.set_target(dim_factor);

        let scale_factor = match distance.abs() {
            0 => 1.0,
            1 => 0.95,
            _ => 0.9,
        };
        self.scale.set_target(scale_factor);
    }

    /// 推进所有弹簧动画
    pub fn tick(&mut self, dt: f64) {
        self.pos_y.tick(dt);
        self.pos_x.tick(dt);
        self.scale.tick(dt);
        self.opacity.tick(dt);
    }

    /// 强制设置垂直位置
    pub fn snap_y(&mut self, y: f64) {
        self.pos_y.snap_to(y);
    }

    /// 设置垂直目标位置
    pub fn set_target_y(&mut self, y: f64) {
        self.pos_y.set_target(y);
    }

    /// 获取当前垂直位置
    pub fn y(&self) -> f64 {
        self.pos_y.current_position
    }

    /// 获取当前水平偏移
    pub fn x_offset(&self) -> f64 {
        self.pos_x.current_position
    }

    /// 获取当前缩放
    pub fn scale(&self) -> f64 {
        self.scale.current_position
    }

    /// 获取当前透明度
    pub fn opacity(&self) -> f64 {
        self.opacity.current_position
    }

    /// 是否所有弹簧都已到达目标
    pub fn arrived(&self) -> bool {
        self.pos_y.arrived()
            && self.pos_x.arrived()
            && self.scale.arrived()
            && self.opacity.arrived()
    }
}
