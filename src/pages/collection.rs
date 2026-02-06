//! 我的收藏页面
//!
//! 展示用户收藏内容的页面组件。

use relm4::{gtk::{self, prelude::*}, ComponentParts, ComponentSender, SimpleComponent};

/// 我的收藏页面组件
pub struct Collection;

#[relm4::component(pub)]
impl SimpleComponent for Collection {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,
            set_spacing: 12,
            set_margin_start: 24,
            set_margin_end: 24,
            set_margin_top: 24,
            set_margin_bottom: 24,

            gtk::Label {
                set_label: "我的收藏",
                set_halign: gtk::Align::Start,
                add_css_class: "heading",
            },

            gtk::Label {
                set_label: "收藏内容开发中...",
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                set_vexpand: true,
                add_css_class: "dim-label",
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Collection;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}
