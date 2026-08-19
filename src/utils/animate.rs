//! 通用淡入/淡出动画工具
//!
//! 复用 fullscreen_lyric 封面动画的同款 tick-callback 范式：
//! 用 `Rc<Cell>` 保存当前值 / 目标值 / 起始帧时间，每帧驱动 widget 的 opacity。
//! 支持动画中途反向重定向，到达目标后触发一次可选回调。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use relm4::gtk;
use relm4::gtk::glib;
use relm4::gtk::prelude::*;

const EPSILON: f64 = 0.001;

/// 对一个 widget 反复执行的淡入淡出动画。
///
/// 同一实例可多次调用 `set_visible` / `set_visible_then`，
/// 目标被重定向时会从当前值向新目标平滑过渡。
pub struct Fade {
    target: Rc<Cell<f64>>,
    current: Rc<Cell<f64>>,
    start_value: Rc<Cell<f64>>,
    start_time: Rc<Cell<Option<u64>>>,
    on_finished: Rc<RefCell<Option<Box<dyn FnOnce()>>>>,
}

impl Fade {
    /// 创建一个 animation。`initial` 为 widget 的起始 opacity（0 或 1）。
    pub fn new(widget: &impl IsA<gtk::Widget>, initial: f64, duration_ms: u64) -> Self {
        widget.set_opacity(initial);

        let target = Rc::new(Cell::new(initial));
        let current = Rc::new(Cell::new(initial));
        let start_value = Rc::new(Cell::new(initial));
        let start_time = Rc::new(Cell::new(None));
        let on_finished: Rc<RefCell<Option<Box<dyn FnOnce()>>>> = Rc::new(RefCell::new(None));

        let cb_target = Rc::clone(&target);
        let cb_current = Rc::clone(&current);
        let cb_start_value = Rc::clone(&start_value);
        let cb_start_time = Rc::clone(&start_time);
        let cb_on_finished = Rc::clone(&on_finished);

        widget.add_tick_callback(move |widget, clock| {
            let t = cb_target.get();
            let c = cb_current.get();

            if (c - t).abs() < EPSILON {
                cb_current.set(t);
                widget.set_opacity(t);
                cb_start_time.set(None);
                let cb = cb_on_finished.borrow_mut().take();
                if let Some(cb) = cb {
                    cb();
                }
                return glib::ControlFlow::Continue;
            }

            let frame_ms = clock.frame_time() as u64 / 1000;
            if cb_start_time.get().is_none() {
                cb_start_time.set(Some(frame_ms));
            }
            let elapsed = (frame_ms - cb_start_time.get().unwrap()) as f64;
            let mut progress = (elapsed / duration_ms as f64).clamp(0.0, 1.0);
            // ease-out cubic
            progress = 1.0 - (1.0 - progress).powi(3);

            let sv = cb_start_value.get();
            let next = sv + (t - sv) * progress;
            cb_current.set(next);
            widget.set_opacity(next);

            glib::ControlFlow::Continue
        });

        Self {
            target,
            current,
            start_value,
            start_time,
            on_finished,
        }
    }

    /// 直接设目标，无完成回调。
    pub fn set_visible(&self, visible: bool) {
        self.set_visible_then(visible, None);
    }

    /// 设置目标并在动画到达目标（或本来就是目标值）时执行回调。
    pub fn set_visible_then(&self, visible: bool, on_finished: Option<Box<dyn FnOnce()>>) {
        let t: f64 = if visible { 1.0 } else { 0.0 };

        // 已处于目标值：直接复位并触发回调
        if (self.current.get() - t).abs() < EPSILON {
            self.current.set(t);
            if let Some(cb) = on_finished {
                cb();
            }
            return;
        }

        self.target.set(t);
        self.start_value.set(self.current.get());
        self.start_time.set(None);
        *self.on_finished.borrow_mut() = on_finished;
    }
}
