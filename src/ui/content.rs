//! 内容区子组件 — Home / Explore / Collection

use log::trace;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::gtk::Orientation;
use relm4::prelude::*;
use relm4::{adw, ComponentParts, ComponentSender, gtk};

use crate::icon_names;
use super::home::Home;

pub struct Content {
    stack: adw::ViewStack,
    buttons: Vec<gtk::Button>,
    current_page: String,
    home: Controller<Home>,
}

#[derive(Debug)]
pub enum ContentMsg {
    SwitchPage(String),
}

#[derive(Debug)]
pub enum ContentOutput {
    ToggleSidebar,
}

#[relm4::component(pub)]
impl SimpleComponent for Content {
    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                set_show_start_title_buttons: false,
                set_show_end_title_buttons: false,

                pack_start = &gtk::ToggleButton {
                    set_icon_name: icon_names::DOCK_LEFT,
                    set_tooltip_text: Some("Toggle Sidebar"),
                    set_active: true,
                    add_css_class: "flat",

                    connect_toggled[sender] => move |_| {
                        sender.output(ContentOutput::ToggleSidebar).unwrap();
                    },
                },

                #[name(title_box)]
                #[wrap(Some)]
                set_title_widget = &gtk::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 3,
                },
            },

            #[name(stack)]
            #[wrap(Some)]
            set_content = &adw::ViewStack {},
        }
    }

    type Init = ();
    type Input = ContentMsg;
    type Output = ContentOutput;

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let home = Home::builder().launch(()).detach();

        let mut model = Self {
            stack: adw::ViewStack::default(),
            buttons: Vec::new(),
            current_page: "home".into(),
            home,
        };
        let mut widgets = view_output!();

        // 用 widgets 中的 stack 替换 model 中的 stack
        model.stack = widgets.stack.clone();

        // 添加主内容页面
        // Home 页面使用 Home 组件
        widgets.stack.add_titled(model.home.widget(), Some("home"), "Home");

        // Explore 和 Collection 暂时用 Label 占位
        let explore_label = gtk::Label::builder()
            .label("Explore")
            .css_classes(["title-1"])
            .vexpand(true)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();
        widgets.stack.add_titled(&explore_label, Some("explore"), "Explore");

        let collection_label = gtk::Label::builder()
            .label("Collection")
            .css_classes(["title-1"])
            .vexpand(true)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();
        widgets.stack.add_titled(&collection_label, Some("collection"), "Collection");

        widgets.stack.set_visible_child_name("home");

        // 创建导航按钮
        let nav_defs = [
            ("home", icon_names::GO_HOME, "Home"),
            ("explore", icon_names::COMPASS2, "Explore"),
            ("collection", icon_names::LIBRARY_MUSIC, "Collection"),
        ];

        for (tag, icon, label) in nav_defs {
            let btn = gtk::Button::builder().icon_name(icon).label(label).build();
            if tag == "home" { btn.add_css_class("raised"); } else { btn.add_css_class("flat"); }
            let s = sender.clone();
            let t = tag.to_string();
            btn.connect_clicked(move |_| s.input(ContentMsg::SwitchPage(t.clone())));
            widgets.title_box.append(&btn);
            model.buttons.push(btn);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        trace!("Content: {message:?}");
        if let ContentMsg::SwitchPage(tag) = message {
            self.stack.set_visible_child_name(&tag);
            for btn in &self.buttons {
                btn.remove_css_class("raised");
                btn.add_css_class("flat");
            }
            let idx = match tag.as_str() {
                "home" => 0, "explore" => 1, "collection" => 2, _ => return,
            };
            if let Some(btn) = self.buttons.get(idx) {
                btn.remove_css_class("flat");
                btn.add_css_class("raised");
            }
            self.current_page = tag;
        }
    }
}
