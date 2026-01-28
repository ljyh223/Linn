use relm4::prelude::*;
use relm4::gtk::prelude::*;
use crate::pages::Page;
use crate::app::{AppInput, AppModel};

pub fn create_sidebar(sender: &ComponentSender<AppModel>) -> gtk::Box {
    let sidebar = gtk::Box::builder()
        .width_request(250)
        .hexpand(false)
        .orientation(gtk::Orientation::Vertical)
        .build();

    // Logo
    let logo = gtk::Label::builder()
        .label("<big><b>Linn</b></big>")
        .use_markup(true)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .build();
    sidebar.append(&logo);

    let separator = gtk::Separator::builder()
        .margin_bottom(10)
        .build();
    sidebar.append(&separator);

    // 导航按钮容器
    let nav_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .vexpand(true)
        .build();
    sidebar.append(&nav_box);

    // 导航项数据
    let nav_items = [
        ("为我推荐", "starred-symbolic", Page::Discover),
        ("发现音乐", "emblem-system-symbolic", Page::Explore),
        ("我的收藏", "library-music-symbolic", Page::Library),
        ("我喜欢的歌曲", "heart-symbolic", Page::Favorites),
    ];

    for (label_text, icon_name, page) in nav_items {
        let button = create_nav_button(label_text, icon_name);

        let sender_clone = sender.clone();
        button.connect_clicked(move |_| {
            sender_clone.input(AppInput::Navigate(page));
        });

        nav_box.append(&button);
    }

    sidebar
}

fn create_nav_button(label_text: &str, icon_name: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .halign(gtk::Align::Start)
        .css_classes(["flat", "navigation-item"])
        .build();

    // 创建内容容器（水平排列）
    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    // 创建图标
    let icon = gtk::Image::builder()
        .icon_name(icon_name)
        .pixel_size(20)
        .build();

    // 创建文字标签
    let label = gtk::Label::builder()
        .label(label_text)
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();

    content_box.append(&icon);
    content_box.append(&label);
    button.set_child(Some(&content_box));

    button
}
