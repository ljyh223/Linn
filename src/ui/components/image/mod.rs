pub mod image_manager;
pub mod imp;
pub mod widget;

// 导出公开的 Wrapper 以供外部 UI 使用
pub use widget::AsyncImage;

/* =========================================================
下面是 Relm4 的使用示例 (仅供参考，不参与模块编译)
========================================================= */


#[cfg(test)]
mod relm4_usage_example {
    use super::AsyncImage;
    use relm4::{
        gtk::{
            self,
            prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
        },
        prelude::*,
    }; // 引入刚才写的自定义控件

    struct AppModel {
        avatar_url: String,
    }

    #[derive(Debug)]
    enum AppInput {
        FetchNextAvatar,
    }

    #[relm4::component]
    impl SimpleComponent for AppModel {
        type Init = ();
        type Input = AppInput;
        type Output = ();

        // 重点关注 view! 宏内的书写方式
        // 我们不需要在 AppModel 里面维护 GdkTexture 或者任何 Option<Bytes> 的状态
        // 我们只需要像绑定原生 gtk::Label 一样绑定 URL！
        view! {
            gtk::Window {
                set_title: Some("Async Image Demo"),
                set_default_width: 400,
                set_default_height: 400,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,

                    // 直接优雅地调用自定义异步图片组件
                    #[local]
                    async_avatar = &AsyncImage {
                        set_width_request: 200,
                        set_height_request: 200,
                        // 动态绑定 Model 的属性，触发 GObject set_property -> 触发下载任务
                        set_url: &model.avatar_url,
                        set_placeholder_icon: "avatar-default-symbolic",
                        set_fallback_icon: "image-missing-symbolic",

                        // 甚至可以绑定原生的 CSS 类进行裁切等样式处理
                        add_css_class: "circular",
                    },

                    gtk::Button {
                        set_label: "Load Another Picture",
                        connect_clicked => AppInput::FetchNextAvatar,
                    }
                }
            }
        }

        fn init(
            _: Self::Init,
            root: Self::Root,
            sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> {
            let model = AppModel {
                avatar_url: "https://example.com/mock-avatar-1.png".to_string(),
            };
            let widgets = view_output!();
            ComponentParts { model, widgets }
        }

        fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
            match msg {
                AppInput::FetchNextAvatar => {
                    // 当 Model 中的 url 发生变化时，Relm4 自动同步视图
                    // 底层的 AsyncImage::set_url 会触发旧网络请求中断并启动新请求
                    self.avatar_url = "https://example.com/mock-avatar-2.png".to_string();
                }
            }
        }
    }
}
