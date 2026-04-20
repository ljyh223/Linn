use relm4::factory::FactoryComponent;
use relm4::gtk::prelude::BoxExt;
use relm4::{gtk::{self, prelude::{GestureSingleExt, OrientableExt, WidgetExt}}, prelude::*};
use relm4::factory::DynamicIndex;

use crate::ui::components::image::AsyncImage;

// 1. 初始化用的结构体
#[derive(Debug)]
pub struct PlaylistCardInit {
    pub id: u64,
    pub cover_url: String,
    pub title: String,
}

// 2. 组件 Model 本身
#[derive(Debug)]
pub struct PlaylistCard {
    id: u64,
    cover_url: String,
    title: String,
}

#[derive(Debug)]
pub enum PlaylistCardOutput {
    Clicked(u64),
}
#[relm4::factory(pub)]
impl FactoryComponent for PlaylistCard {
    type Init = PlaylistCardInit;
    type Input = (); // 不再需要 Input
    type Output = PlaylistCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox; 

    // View 宏
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,              
            set_valign: gtk::Align::Center,
            add_css_class: "playlist-card", 

            AsyncImage {
                set_width_request: 160,
                set_height_request: 160,
                set_corner_radius: 8.0,
                // 注意：在工厂组件的 view 宏中，通常直接写 &self.xxx
                set_url: self.cover_url.clone(),
                set_placeholder_icon: "folder-music-symbolic", 
                set_fallback_icon: "image-missing-symbolic",
                add_css_class: "rounded-cover", 
            },

            gtk::Label {
                // 标题是固定的，不需要 #[watch]
                set_label: &self.title,
                set_halign: gtk::Align::Start, 
                set_max_width_chars: 25,       
                set_ellipsize: gtk::pango::EllipsizeMode::End, 
                add_css_class: "playlist-title",
            },

            // 点击手势
            add_controller = gtk::GestureClick {
                set_button: 1, 
                // ✅ 优化：直接在这里发送 Output，不需要绕道 Input
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(PlaylistCardOutput::Clicked(id)).unwrap();
                    }
                }
            }
        }
    }

    // ✅ 核心改变：SimpleComponent 的 init 变成了工厂的 init_model
    // 注意：没有了 root 和 view_output!()，这些都由工厂底层自动接管
    fn init_model(
        init: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            id: init.id,
            cover_url: init.cover_url,
            title: init.title,
        }
    }

    // ✅ 因为 Input 是 ()，所以 update 函数体空着就行，不需要删除这个方法
    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {
    }
}
