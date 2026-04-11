//! Header component — 纯粹的顶部导航栏

use log::trace;
use relm4::gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, gtk};

pub struct Header {
    // 纯 Header 不需要持有 Sidebar 和 Content，它只负责自己
}

#[derive(Debug)]
pub enum HeaderMsg {
    // 暂时留空，或者放搜索输入、按钮点击的消息
    Search(String),
}

#[relm4::component(pub)]
impl SimpleComponent for Header {
    type Init = ();
    type Input = HeaderMsg;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 16,
            set_margin_top: 8,
            set_margin_bottom: 8,
            set_margin_start: 16,
            set_margin_end: 16,
            set_halign: gtk::Align::Center, // 居中显示

            // 这里放你想要的 Home、Explore、Collection 按钮
            gtk::ToggleButton {
                set_label: "Home",
                set_active: true,
            },
            gtk::ToggleButton {
                set_label: "Explore",
            },
            gtk::ToggleButton {
                set_label: "Collection",
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {
        // 处理头部按钮事件
    }
}