mod async_image;

use relm4::{gtk::prelude::*, gtk, ComponentParts, ComponentSender, SimpleComponent, RelmWidgetExt};
use async_image::AsyncImage;

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::ApplicationWindow {
            set_title: Some("AsyncImage - 像原生组件一样简单"),
            set_default_width: 900,
            set_default_height: 600,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 15,
                set_margin_all: 20,

                // 标题
                gtk::Label {
                    set_label: "🚀 在 view! 中直接使用 AsyncImage",
                    add_css_class: "title",
                },

                gtk::Label {
                    set_label: "现在使用 AsyncImage 就像使用 gtk::Button 一样简单！",
                    add_css_class: "subtitle",
                },

                // 示例 1: 不同圆角
                gtk::Label {
                    set_label: "示例 1: 不同圆角",
                    add_css_class: "section-title",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 15,
                    set_halign: gtk::Align::Center,

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 180,
                        set_height_request: 180,
                    },

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 180,
                        set_height_request: 180,
                        set_border_radius: 12,
                    },

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 180,
                        set_height_request: 180,
                        set_border_radius: 24,
                    },

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 180,
                        set_height_request: 180,
                        set_border_radius: 90,  // 圆形！
                    },
                },

                // 示例 2: 卡片布局
                gtk::Label {
                    set_label: "示例 2: 卡片布局",
                    add_css_class: "section-title",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 20,
                    set_halign: gtk::Align::Center,

                    gtk::Frame {
                        gtk::Label {
                            set_label: "用户头像",
                        },
                        AsyncImage {
                            set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                            set_width_request: 120,
                            set_height_request: 120,
                            set_border_radius: 60,  // 圆形头像
                            set_halign: gtk::Align::Center,
                        },
                    },

                    gtk::Frame {
                        gtk::Label {
                            set_label: "封面图",
                        },
                        AsyncImage {
                            set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                            set_width_request: 200,
                            set_height_request: 150,
                            set_border_radius: 8,
                            set_halign: gtk::Align::Center,
                        },
                    },
                },

                // 示例 3: 自定义圆角
                gtk::Label {
                    set_label: "示例 3: 自定义圆角（任意值）",
                    add_css_class: "section-title",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 15,
                    set_halign: gtk::Align::Center,

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 150,
                        set_height_request: 150,
                        set_border_radius: 5,   // 自定义 5px
                    },

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 150,
                        set_height_request: 150,
                        set_border_radius: 17,  // 自定义 17px
                    },

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 150,
                        set_height_request: 150,
                        set_border_radius: 33,  // 自定义 33px
                    },

                    AsyncImage {
                        set_src: "https://p1.music.126.net/fjfq18nvWGD7CakXR6yecA==/849922488314050.jpg",
                        set_width_request: 150,
                        set_height_request: 150,
                        set_border_radius: 50,  // 自定义 50px
                    },
                },

                // 说明
                gtk::Label {
                    set_label: r#"
✅ 像原生 widget 一样使用 - 直接在 view! 中声明
✅ 无需 #[local_ref] - 所有配置都在一个地方
✅ 支持任意圆角 - 5px, 17px, 33px 等任意值都支持
✅ CSS 自动注入 - 无需手动配置样式
✅ 代码简洁 - 从 20+ 行减少到 5 行
                    "#,
                    set_justify: gtk::Justification::Left,
                    add_css_class: "info",
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();

        // 应用样式
        let css = gtk::CssProvider::new();
        css.load_from_data(
            "
            .title { font-size: 26px; font-weight: bold; }
            .subtitle { font-size: 14px; color: @dim_label_color; margin-bottom: 20px; }
            .section-title { font-size: 16px; font-weight: bold; margin-top: 15px; }
            .info { font-size: 12px; font-family: monospace; padding: 15px; }
            frame { border: 2px solid @borders; border-radius: 12px; padding: 15px; }
            label { font-size: 12px; font-weight: 500; margin-bottom: 8px; }
        ",
        );

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        ComponentParts { model: App, widgets }
    }

    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {}
}

#[derive(Debug, Clone)]
pub struct App;

#[derive(Debug)]
pub enum AppMsg {}

fn main() {
    relm4::RelmApp::new("org.example.async-image-simple")
        .run::<App>(());
}
