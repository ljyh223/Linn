use relm4::gtk::{
    cairo::LinearGradient,
    gdk::Display,
    glib::{
        ControlFlow,
        object::{Cast, ObjectExt},
        timeout_add_local,
    },
    prelude::{DrawingAreaExtManual, WidgetExt},
};

pub fn make_shimmer_widget() -> relm4::gtk::Widget {
    let area = relm4::gtk::DrawingArea::new();
    area.set_halign(relm4::gtk::Align::Fill);
    area.set_valign(relm4::gtk::Align::Fill);
    area.set_hexpand(true);
    area.set_vexpand(true);

    // phase: 0.0 ~ 1.0，驱动高光位置
    let phase = std::rc::Rc::new(std::cell::Cell::new(0.0f32));

    area.set_draw_func({
        let phase = phase.clone();
        move |_, cr, w, h| {
            let p = phase.get();
            let w = w as f64;
            let h = h as f64;

            // 底色：从 relm4::gtk 样式上下文取，自动适配深浅色
            // let style = StyleContext::new()
            // 用硬编码的中性灰，深色模式下也合适
            // 深色：~#3a3a3a，浅色：~#e0e0e0 — 用 cairo pattern 模拟
            let is_dark = {
                let display = Display::default().unwrap();
                let settings = relm4::gtk::Settings::for_display(&display);
                settings.is_gtk_application_prefer_dark_theme()
            };

            let (base_r, base_g, base_b) = if is_dark {
                (0.22, 0.22, 0.22)
            } else {
                (0.88, 0.88, 0.88)
            };
            let (hi_r, hi_g, hi_b) = if is_dark {
                (0.32, 0.32, 0.32)
            } else {
                (0.96, 0.96, 0.96)
            };

            // 底色矩形
            cr.set_source_rgb(base_r, base_g, base_b);
            cr.rectangle(0.0, 0.0, w, h);
            let _ = cr.fill();

            // 高光扫描条：用 LinearGradient
            let sweep_x = (p as f64) * (w + w * 0.6) - w * 0.3;
            let grad = LinearGradient::new(sweep_x - w * 0.3, 0.0, sweep_x + w * 0.3, 0.0);
            grad.add_color_stop_rgba(0.0, hi_r, hi_g, hi_b, 0.0);
            grad.add_color_stop_rgba(0.5, hi_r, hi_g, hi_b, 1.0);
            grad.add_color_stop_rgba(1.0, hi_r, hi_g, hi_b, 0.0);
            cr.set_source(&grad).unwrap();
            cr.rectangle(0.0, 0.0, w, h);
            let _ = cr.fill();
        }
    });

    // 每 16ms（~60fps）推进 phase，触发重绘
    let area_weak = area.downgrade();
    timeout_add_local(std::time::Duration::from_millis(16), move || {
        let Some(area) = area_weak.upgrade() else {
            return ControlFlow::Break; // widget 已销毁，停止
        };
        phase.set((phase.get() + 0.012).fract()); // 约 1.4s 一个周期
        area.queue_draw();
        ControlFlow::Continue
    });

    area.upcast()
}
