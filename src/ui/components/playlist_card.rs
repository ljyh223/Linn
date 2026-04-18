use relm4::{gtk::{self, prelude::{BoxExt, GestureSingleExt, OrientableExt, WidgetExt}}, prelude::*};

use crate::ui::components::image::AsyncImage;

// 1. 初始化用的结构体 (传递给 init 的参数)
#[derive(Debug)]
pub struct PlaylistCardInit {
    pub id: u64,
    pub cover_url: String,
    pub title: String,
}

// 2. 组件 Model 本身
#[derive(Debug, Clone)]
pub struct PlaylistCard {
    pub id: u64,
    pub cover_url: String,
    pub title: String,
}

// 3. 内部输入事件 (UI 触发)
#[derive(Debug)]
pub enum PlaylistCardInput {
    Clicked,
}

// 4. 外部输出事件 (发给父组件的)
#[derive(Debug)]
pub enum PlaylistCardOutput {
    // 传递被点击的歌单 ID 给外部
    Clicked(u64),
}

// ==========================================
// 核心：使用 relm4 宏来自动处理大部分模版代码
// ==========================================
#[relm4::component(pub)]
impl SimpleComponent for PlaylistCard {
    type Init = PlaylistCardInit;
    type Input = PlaylistCardInput;
    type Output = PlaylistCardOutput;

    // View 宏直接写在这里
    view! {
        // Root widget: Box
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,              
            set_valign: gtk::Align::Center,
            add_css_class: "playlist-card", 

            // 1. Cover 
            // （前提：AsyncImage 已经注册为 GObject 原生 Widget，或者通过其他方式引入）
            AsyncImage {
                set_width_request: 160,
                set_height_request: 160,
                set_corner_radius: 8.0,
                // 这里直接读取 model 里的属性
                set_url: model.cover_url.clone(),
                set_placeholder_icon: "folder-music-symbolic", 
                set_fallback_icon: "image-missing-symbolic",
                add_css_class: "rounded-cover", 
            },

            // 2. Title
            gtk::Label {
                // 如果标题未来会改变，加上 #[watch]；如果不改变，去掉也可
                #[watch]
                set_label: &model.title,
                set_halign: gtk::Align::Start, 
                set_max_width_chars: 10,       
                set_ellipsize: gtk::pango::EllipsizeMode::End, 
                add_css_class: "playlist-title",
            },

            // 3. 点击手势
            add_controller = gtk::GestureClick {
                set_button: 1, 
                // sender 捕获组件的发送器
                connect_released[sender] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        // 发送内部 Input 事件
                        sender.input(PlaylistCardInput::Clicked);
                    }
                }
            }
        }
    }

    // 初始化函数
    fn init(
        init: Self::Init,
        root: Self::Root, // 宏会自动推断出 Root 是 gtk::Box
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        
        // 创建 Model
        let model = Self {
            id: init.id,
            cover_url: init.cover_url,
            title: init.title,
        };

        // 宏生成的代码，用于实例化 UI 组件
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    // 事件更新处理
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlaylistCardInput::Clicked => {
                // 收到内部点击事件后，触发 Output，告诉上层组件（父组件）被点击的 ID
                if let Err(err) = sender.output(PlaylistCardOutput::Clicked(self.id)) {
                    eprintln!("Failed to send output: {:?}", err);
                }
            }
        }
    }
}