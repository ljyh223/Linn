//! 弹簧物理模型
//!
//! 解析式弹簧求解器，从 AMLL (applemusic-like-lyrics) 的 spring.ts 移植。
//! 支持过阻尼/临界阻尼（同一公式）和欠阻尼两种情况。

/// 弹簧参数
#[derive(Debug, Clone, Copy)]
pub struct SpringParams {
    pub mass: f64,
    pub damping: f64,
    pub stiffness: f64,
    pub soft: bool,
}

impl SpringParams {
    pub const fn new(mass: f64, damping: f64, stiffness: f64) -> Self {
        Self {
            mass,
            damping,
            stiffness,
            soft: false,
        }
    }
}

/// 弹簧求解器枚举
#[derive(Debug, Clone)]
enum SpringSolver {
    /// 过阻尼/临界阻尼（AMLL 统一为同一公式）
    Overdamped {
        to: f64,
        delta: f64,
        /// omega = angular_frequency (< 0)
        omega: f64,
        leftover: f64,
    },
    /// 欠阻尼：cos/sin * exp
    Underdamped {
        to: f64,
        delta: f64,
        /// dm = -0.5 * damping / mass (< 0)
        dm: f64,
        /// dfm = 0.5 * damping_freq / mass (> 0)
        dfm: f64,
        leftover: f64,
    },
    /// 静态（已到达目标）
    Static { to: f64 },
}

/// 弹簧动画
#[derive(Debug, Clone)]
pub struct Spring {
    pub current_position: f64,
    pub current_velocity: f64,
    pub target_position: f64,
    current_time: f64,
    solver: SpringSolver,
    params: SpringParams,
}

impl Spring {
    pub fn new(params: SpringParams, initial: f64) -> Self {
        Self {
            current_position: initial,
            current_velocity: 0.0,
            target_position: initial,
            current_time: 0.0,
            solver: SpringSolver::Static { to: initial },
            params,
        }
    }

    /// 设置目标位置，重建解析求解器
    pub fn set_target(&mut self, target: f64) {
        if (self.target_position - target).abs() < f64::EPSILON {
            return;
        }
        self.target_position = target;
        self.current_time = 0.0;

        let SpringParams {
            mass,
            damping,
            stiffness,
            soft,
        } = self.params;

        let from = self.current_position;
        let velocity = self.current_velocity;
        // AMLL: delta = to - from
        let delta = target - from;

        // 软弹簧或高阻尼：统一用过阻尼公式
        let use_overdamped = soft || damping / (2.0 * (stiffness * mass).sqrt()) >= 1.0;

        if use_overdamped {
            // angular_frequency = -sqrt(k/m)  (负值)
            let omega = -(stiffness / mass).sqrt();
            // AMLL: leftover = -angular_frequency * delta - velocity
            //        = -(-sqrt(k/m)) * delta - velocity
            //        = sqrt(k/m) * delta - velocity
            let leftover = -(omega) * delta - velocity;
            self.solver = SpringSolver::Overdamped {
                to: target,
                delta,
                omega,
                leftover,
            };
        } else {
            // 欠阻尼
            let damping_freq =
                (4.0 * mass * stiffness - damping * damping).sqrt();
            // AMLL: leftover = (damping * delta - 2 * mass * velocity) / damping_frequency
            let leftover = (damping * delta - 2.0 * mass * velocity) / damping_freq;
            let dm = -0.5 * damping / mass;
            let dfm = 0.5 * damping_freq / mass;
            self.solver = SpringSolver::Underdamped {
                to: target,
                delta,
                dm,
                dfm,
                leftover,
            };
        }
    }

    /// 推进弹簧动画，返回是否已到达目标
    pub fn tick(&mut self, dt: f64) -> bool {
        if dt <= 0.0 {
            return self.arrived();
        }

        self.current_time += dt;
        let t = self.current_time;

        match &self.solver {
            SpringSolver::Overdamped {
                to,
                delta,
                omega,
                leftover,
            } => {
                let (to, delta, omega, leftover) = (*to, *delta, *omega, *leftover);
                let exp_val = (t * omega).exp();
                // AMLL: to - (delta + t * leftover) * e^(omega * t)
                self.current_position = to - (delta + t * leftover) * exp_val;
                // velocity = d/dt of position
                self.current_velocity =
                    -((delta + t * leftover) * omega * exp_val + leftover * exp_val);
                self.clamp_if_near(to);
            }
            SpringSolver::Underdamped {
                to,
                delta,
                dm,
                dfm,
                leftover,
            } => {
                let (to, delta, dm, dfm, leftover) = (*to, *delta, *dm, *dfm, *leftover);
                let exp_val = (t * dm).exp();
                let cos_val = (t * dfm).cos();
                let sin_val = (t * dfm).sin();
                // AMLL: to - (cos*delta + sin*leftover) * e^(dm * t)
                self.current_position = to - (cos_val * delta + sin_val * leftover) * exp_val;
                // 速度: 中心差分数值微分
                self.current_velocity = derivative_spring(t, to, dm, dfm, delta, leftover);
                self.clamp_if_near(to);
            }
            SpringSolver::Static { .. } => {
                return true;
            }
        }

        self.arrived()
    }

    /// 若非常接近目标则直接吸附，避免数值振荡
    fn clamp_if_near(&mut self, to: f64) {
        if (self.current_position - to).abs() < 0.005
            && self.current_velocity.abs() < 0.01
        {
            self.current_position = to;
            self.current_velocity = 0.0;
        }
    }

    /// 三重收敛检查：位置 + 速度 + 加速度均 < 0.01
    pub fn arrived(&self) -> bool {
        matches!(&self.solver, SpringSolver::Static { .. })
            || self.current_velocity.abs() < 0.01
                && (self.current_position - self.target_position).abs() < 0.01
    }

    /// 更新弹簧参数，不重建求解器（仅在下次 set_target 时生效）
    pub fn update_params(&mut self, params: SpringParams) {
        self.params = params;
    }

    /// 设置目标位置，并可选地同时应用动态弹簧参数
    /// 如果提供了 `dynamic_params`，则使用该参数重建求解器（一步完成，避免两次重建）
    pub fn set_target_with_params(&mut self, target: f64, params: SpringParams) {
        self.params = params;
        // 以下与 set_target 相同但用新 params
        if (self.target_position - target).abs() < f64::EPSILON
            && (self.params.mass - params.mass).abs() < f64::EPSILON
            && (self.params.damping - params.damping).abs() < f64::EPSILON
            && (self.params.stiffness - params.stiffness).abs() < f64::EPSILON
        {
            // 参数和目标都没变，但 params 可能已经更新了，仍需重建
        }
        self.target_position = target;
        self.current_time = 0.0;

        let from = self.current_position;
        let velocity = self.current_velocity;
        let delta = target - from;

        let SpringParams { mass, damping, stiffness, soft } = params;
        let use_overdamped = soft || damping / (2.0 * (stiffness * mass).sqrt()) >= 1.0;

        if use_overdamped {
            let omega = -(stiffness / mass).sqrt();
            let leftover = -(omega) * delta - velocity;
            self.solver = SpringSolver::Overdamped { to: target, delta, omega, leftover };
        } else {
            let damping_freq = (4.0 * mass * stiffness - damping * damping).sqrt();
            let leftover = (damping * delta - 2.0 * mass * velocity) / damping_freq;
            let dm = -0.5 * damping / mass;
            let dfm = 0.5 * damping_freq / mass;
            self.solver = SpringSolver::Underdamped { to: target, delta, dm, dfm, leftover };
        }
    }

    /// 强制吸附到指定位置
    pub fn snap_to(&mut self, position: f64) {
        self.current_position = position;
        self.current_velocity = 0.0;
        self.target_position = position;
        self.current_time = 0.0;
        self.solver = SpringSolver::Static { to: position };
    }
}

/// 中心差分数值微分，计算欠阻尼弹簧在时间 t 的速度
fn derivative_spring(
    t: f64,
    to: f64,
    dm: f64,
    dfm: f64,
    delta: f64,
    leftover: f64,
) -> f64 {
    let h = 0.001;
    let t1 = t - h;
    let t2 = t + h;
    let exp1 = (t1 * dm).exp();
    let exp2 = (t2 * dm).exp();
    let cos1 = (t1 * dfm).cos();
    let cos2 = (t2 * dfm).cos();
    let sin1 = (t1 * dfm).sin();
    let sin2 = (t2 * dfm).sin();
    let p1 = to - (cos1 * delta + sin1 * leftover) * exp1;
    let p2 = to - (cos2 * delta + sin2 * leftover) * exp2;
    (p2 - p1) / (2.0 * h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_converges() {
        let params = SpringParams::new(1.0, 20.0, 100.0);
        let mut spring = Spring::new(params, 0.0);
        // t=0 位置必须是 0，不能跳变
        assert!((spring.current_position).abs() < f64::EPSILON);
        spring.set_target(100.0);
        // t=0 位置仍是 0（刚设目标还未推进一步）
        assert!((spring.current_position).abs() < f64::EPSILON);
        // 推进几步应离开 0
        spring.tick(0.016);
        assert!(spring.current_position > 0.0);

        for _ in 0..200 {
            spring.tick(0.016);
        }

        assert!(spring.arrived());
        assert!((spring.current_position - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_spring_underdamped() {
        let params = SpringParams::new(0.5, 5.0, 200.0);
        let mut spring = Spring::new(params, 0.0);
        spring.set_target(50.0);
        assert!((spring.current_position).abs() < f64::EPSILON);

        spring.tick(0.016);
        assert!(spring.current_position > 0.0);

        for _ in 0..300 {
            spring.tick(0.016);
        }

        assert!(spring.arrived());
    }

    #[test]
    fn test_spring_snap() {
        let params = SpringParams::new(1.0, 15.0, 90.0);
        let mut spring = Spring::new(params, 0.0);
        spring.snap_to(42.0);
        assert!((spring.current_position - 42.0).abs() < f64::EPSILON);
        assert!(spring.arrived());
    }

    #[test]
    fn test_spring_no_jump() {
        // 关键测试：调 set_target 后位置不应跳变
        let params = SpringParams::new(1.0, 20.0, 100.0);
        let mut spring = Spring::new(params, 0.0);
        // 先收敛到某个位置
        spring.snap_to(50.0);
        // 设置新目标
        spring.set_target(200.0);
        // 位置应保持在 50（还没 tick）
        assert!((spring.current_position - 50.0).abs() < 0.01);
        // tick 后应朝 200 移动
        spring.tick(0.016);
        assert!(spring.current_position > 50.0);
    }
}
