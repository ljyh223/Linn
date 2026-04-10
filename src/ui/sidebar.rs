//! 侧边栏子组件 — Player / Lyrics / Queue

use log::trace;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::gtk::Orientation;
use relm4::{adw, ComponentParts, ComponentSender, SimpleComponent, gtk};

use crate::icon_names;

pub struct Sidebar {
    stack: adw::ViewStack,
    buttons: Vec<gtk::Button>,
    current_page: String,
}

#[derive(Debug)]
pub enum SidebarMsg {
    SwitchPage(String),
}

#[relm4::component(pub)]
impl SimpleComponent for Sidebar {
    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                set_show_start_title_buttons: true,
                set_show_end_title_buttons: true,
            },

            #[wrap(Some)]
            set_content = &adw::ViewStack {
                set_name: Some("sidebar_stack"),
            },

            add_bottom_bar = &gtk::Box {
                set_orientation: Orientation::Horizontal,
                set_homogeneous: true,
                set_spacing: 0,
                set_margin_start: 7,
                set_margin_end: 7,
                set_margin_top: 6,
                set_margin_bottom: 6,
                add_css_class: "linked",
            },

            set_bottom_bar_style: adw::ToolbarStyle::Flat,
        }
    }

    type Init = ();
    type Input = SidebarMsg;
    type Output = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 获取 view! 创建的 ViewStack
        let stack: adw::ViewStack = root
            .content().unwrap()
            .downcast_ref::<adw::ViewStack>().unwrap()
            .clone();

        // 添加页面
        let pages = [
            ("player", icon_names::MUSIC_NOTE_OUTLINE, "Player", "No song playing", "Player controls will appear here"),
            ("lyrics", icon_names::CHAT_BUBBLE_TEXT, "Lyrics", "Lyrics", "Lyrics will appear here"),
            ("queue", icon_names::MUSIC_QUEUE, "Queue", "Queue", "Queue is empty"),
        ];

        for (name, icon, title, label, subtitle) in pages {
            let page = gtk::Box::builder()
                .orientation(Orientation::Vertical)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .vexpand(true)
                .spacing(12)
                .build();
            page.append(&gtk::Image::builder().icon_name(icon).pixel_size(64).opacity(0.4).build());
            page.append(&gtk::Label::builder().label(label).css_classes(["title-2"]).build());
            page.append(&gtk::Label::builder().label(subtitle).opacity(0.6).build());
            stack.add_titled(&page, Some(name), title);
        }
        stack.set_visible_child_name("player");

        // 获取 view! 创建的底部 Box，添加按钮
        let bottom_bars = root.bottom_bars();
        let footer: &gtk::Box = bottom_bars.first().unwrap().downcast_ref().unwrap();

        let button_defs = [
            ("player", icon_names::MUSIC_NOTE_OUTLINE, "Player"),
            ("lyrics", icon_names::CHAT_BUBBLE_TEXT, "Lyrics"),
            ("queue", icon_names::MUSIC_QUEUE, "Queue"),
        ];
        let mut buttons = Vec::new();

        for (tag, icon, label) in button_defs {
            let btn = gtk::Button::builder().hexpand(true).build();
            btn.set_child(Some(
                &adw::ButtonContent::builder().icon_name(icon).label(label).build(),
            ));
            if tag == "player" { btn.add_css_class("raised"); } else { btn.add_css_class("flat"); }
            let s = sender.clone();
            let t = tag.to_string();
            btn.connect_clicked(move |_| s.input(SidebarMsg::SwitchPage(t.clone())));
            footer.append(&btn);
            buttons.push(btn);
        }

        let model = Self { stack, buttons, current_page: "player".into() };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        trace!("Sidebar: {message:?}");
        if let SidebarMsg::SwitchPage(tag) = message {
            self.stack.set_visible_child_name(&tag);
            for btn in &self.buttons {
                btn.remove_css_class("raised");
                btn.add_css_class("flat");
            }
            let idx = match tag.as_str() {
                "player" => 0, "lyrics" => 1, "queue" => 2, _ => return,
            };
            if let Some(btn) = self.buttons.get(idx) {
                btn.remove_css_class("flat");
                btn.add_css_class("raised");
            }
            self.current_page = tag;
        }
    }
}
