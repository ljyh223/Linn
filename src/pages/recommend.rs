//! 为我推荐页面
//!
//! 展示推荐歌单的页面组件。

use relm4::{gtk::{self, prelude::*}, ComponentParts, ComponentSender, SimpleComponent};

/// 为我推荐页面组件
pub struct Recommend;

#[relm4::component(pub)]
impl SimpleComponent for Recommend {
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
                set_label: "为我推荐",
                set_halign: gtk::Align::Start,
                add_css_class: "heading",
            },

            gtk::Label {
                set_label: "推荐歌单加载中...",
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
        let model = Recommend;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}
