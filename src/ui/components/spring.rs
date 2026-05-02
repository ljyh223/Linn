//! 弹簧物理模型
//!
//! 解析式弹簧求解器，从 AMLL (applemusic-like-lyrics) 的 spring.ts 移植。
//! 支持过阻尼、临界阻尼、欠阻尼三种情况。

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

/// 弹簧求解器枚举（避免堆分配）
#[derive(Debug, Clone)]
enum SpringSolver {
    /// 过阻尼/临界阻尼：delta + t * leftover 乘以 exp
    Overdamped {
        to: f64,
        omega: f64,
        delta: f64,
        leftover: f64,
    },
    /// 欠阻尼：cos/sin 乘以 exp
    Underdamped {
        to: f64,
        dm: f64,
        dfm: f64,
        delta: f64,
        leftover: f64,
    },
    /// 静态（已到达目标）
    Static {
        to: f64,
    },
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
    /// 创建新的弹簧
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

    /// 设置目标位置，重置求解器
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
            ..
        } = self.params;

        let from = self.current_position;
        let velocity = self.current_velocity;

        // 角频率
        let angular_freq = (stiffness / mass).sqrt();

        // 判别式
        let discriminant = damping * damping - 4.0 * mass * stiffness;

        if discriminant.abs() < f64::EPSILON {
            // 临界阻尼
            let omega = -angular_freq;
            let delta = from - target;
            let leftover = -velocity + delta * omega;
            self.solver = SpringSolver::Overdamped {
                to: target,
                omega,
                delta,
                leftover,
            };
        } else if discriminant > 0.0 {
            // 过阻尼
            let omega_a = (-damping + discriminant.sqrt()) / (2.0 * mass);
            let omega_b = (-damping - discriminant.sqrt()) / (2.0 * mass);
            // 使用较慢的那个根
            let omega = if omega_a.abs() < omega_b.abs() {
                omega_a
            } else {
                omega_b
            };
            let delta = from - target;
            let leftover = (-velocity + delta * omega).abs();
            self.solver = SpringSolver::Overdamped {
                to: target,
                omega,
                delta,
                leftover,
            };
        } else {
            // 欠阻尼
            let dm = -damping / (2.0 * mass);
            let dfm = (4.0 * mass * stiffness - damping * damping).sqrt() / (2.0 * mass);
            let delta = from - target;
            let leftover = (-velocity + delta * dm.abs()) / dfm;
            self.solver = SpringSolver::Underdamped {
                to: target,
                dm,
                dfm,
                delta,
                leftover,
            };
        }
    }

    /// 推进弹簧动画，返回是否已到达目标
    pub fn tick(&mut self, dt: f64) -> bool {
        if self.arrived() {
            return true;
        }

        self.current_time += dt;
        let t = self.current_time;

        match &self.solver {
            SpringSolver::Overdamped {
                to,
                omega,
                delta,
                leftover,
            } => {
                let exp_val = (t * omega).exp();
                self.current_position = to - (delta + t * leftover) * exp_val;
                self.current_velocity =
                    -((delta + t * leftover) * omega * exp_val + leftover * exp_val);
            }
            SpringSolver::Underdamped {
                to,
                dm,
                dfm,
                delta,
                leftover,
            } => {
                let exp_val = (t * dm).exp();
                let cos_val = (t * dfm).cos();
                let sin_val = (t * dfm).sin();
                self.current_position = to - (cos_val * delta + sin_val * leftover) * exp_val;
                // 速度：数值微分
                self.current_velocity = self.derivative(t, *to, *dm, *dfm, *delta, *leftover);
            }
            SpringSolver::Static { .. } => {
                return true;
            }
        }

        self.arrived()
    }

    /// 三重收敛检查：位置 + 速度 + 加速度均 < 0.01
    pub fn arrived(&self) -> bool {
        let pos_diff = (self.current_position - self.target_position).abs();
        let vel = self.current_velocity.abs();
        // 使用中心差分计算加速度
        let acc = self.derivative_central().abs();
        pos_diff < 0.01 && vel < 0.01 && acc < 0.01
    }

    /// 欠阻尼模式的速度计算
    fn derivative(
        &self,
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

    /// 中心差分数值微分计算加速度
    fn derivative_central(&self) -> f64 {
        let h = 0.001;
        match &self.solver {
            SpringSolver::Overdamped {
                to,
                omega,
                delta,
                leftover,
            } => {
                let t = self.current_time;
                let t1 = t - h;
                let t2 = t + h;
                let exp1 = (t1 * omega).exp();
                let exp2 = (t2 * omega).exp();
                let p1 = to - (delta + t1 * leftover) * exp1;
                let p2 = to - (delta + t2 * leftover) * exp2;
                let v1 = -((delta + t1 * leftover) * omega * exp1 + leftover * exp1);
                let v2 = -((delta + t2 * leftover) * omega * exp2 + leftover * exp2);
                (v2 - v1) / (2.0 * h)
            }
            SpringSolver::Underdamped {
                to,
                dm,
                dfm,
                delta,
                leftover,
            } => {
                let t = self.current_time;
                let t1 = t - h;
                let t2 = t + h;
                let exp1 = (t1 * dm).exp();
                let exp2 = (t2 * dm).exp();
                let cos1 = (t1 * dfm).cos();
                let cos2 = (t2 * dfm).cos();
                let sin1 = (t1 * dfm).sin();
                let sin2 = (t2 * dfm).sin();
                let v1_h = 0.0005;
                let t1h = t1 - v1_h;
                let t2h = t1 + v1_h;
                let exp1h = (t1h * dm).exp();
                let exp2h = (t2h * dm).exp();
                let cos1h = (t1h * dfm).cos();
                let cos2h = (t2h * dfm).cos();
                let sin1h = (t1h * dfm).sin();
                let sin2h = (t2h * dfm).sin();
                let p1h = to - (cos1h * delta + sin1h * leftover) * exp1h;
                let p2h = to - (cos2h * delta + sin2h * leftover) * exp2h;
                let v1 = (p2h - p1h) / (2.0 * v1_h);

                let t1h2 = t2 - v1_h;
                let t2h2 = t2 + v1_h;
                let exp1h2 = (t1h2 * dm).exp();
                let exp2h2 = (t2h2 * dm).exp();
                let cos1h2 = (t1h2 * dfm).cos();
                let cos2h2 = (t2h2 * dfm).cos();
                let sin1h2 = (t1h2 * dfm).sin();
                let sin2h2 = (t2h2 * dfm).sin();
                let p1h2 = to - (cos1h2 * delta + sin1h2 * leftover) * exp1h2;
                let p2h2 = to - (cos2h2 * delta + sin2h2 * leftover) * exp2h2;
                let v2 = (p2h2 - p1h2) / (2.0 * v1_h);

                (v2 - v1) / (2.0 * h)
            }
            SpringSolver::Static { .. } => 0.0,
        }
    }

    /// 强制设置位置和速度（用于初始化或跳转）
    pub fn snap_to(&mut self, position: f64) {
        self.current_position = position;
        self.current_velocity = 0.0;
        self.target_position = position;
        self.current_time = 0.0;
        self.solver = SpringSolver::Static { to: position };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_converges() {
        let params = SpringParams::new(1.0, 20.0, 100.0);
        let mut spring = Spring::new(params, 0.0);
        spring.set_target(100.0);

        for _ in 0..200 {
            spring.tick(0.016);
        }

        assert!(spring.arrived());
        assert!((spring.current_position - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_spring_underdamped() {
        let params = SpringParams::new(0.5, 5.0, 200.0);
        let mut spring = Spring::new(params, 0.0);
        spring.set_target(50.0);

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
}
