use relm4::gtk::{
    CssProvider, Overflow, STYLE_PROVIDER_PRIORITY_APPLICATION, Widget,
    gdk::Display,
    glib::object::{IsA, ObjectExt, ObjectType},
    prelude::WidgetExt,
    style_context_add_provider_for_display,
};

/// 给任意 widget 设置 border-radius，同时设置 overflow: hidden 实现裁剪。
/// CSS provider 绑定到 widget 的 data 上，随 widget 销毁自动释放。
pub fn set_border_radius(widget: &impl IsA<Widget>, px: i32) {
    widget.as_ref().set_overflow(Overflow::Hidden);

    // 生成一个唯一的 CSS class 名（用指针地址，避免跨 widget 污染）
    let addr = widget.as_ref().as_ptr() as usize;
    let class_name = format!("r{:x}", addr);

    let css = format!(".{} {{ border-radius: {}px; }}", class_name, px);
    let provider = CssProvider::new();
    provider.load_from_data(&css);

    style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // 清理旧 class、加新 class
    let w = widget.as_ref();
    // 移除以 'r' 开头的旧地址 class（我们自己生成的）
    let old_classes: Vec<_> = w
        .css_classes()
        .into_iter()
        .filter(|c| {
            let s = c.as_str();
            s.len() > 10 && s.starts_with('r') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
        })
        .collect();
    for c in old_classes {
        w.remove_css_class(&c);
    }
    w.add_css_class(&class_name);

    // 将 provider 存入 widget data，随 widget 生命周期自动释放
    unsafe {
        w.set_data("__border_radius_provider", provider)
    };
}
