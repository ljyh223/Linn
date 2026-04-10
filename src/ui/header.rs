//! Header bar component — 主布局控制器
//!
//! 参考 Tonearm 的布局：
//! - 使用 adw::OverlaySplitView 作为根布局
//! - 侧边栏有自己的 ToolbarView + HeaderBar（窗口标题、菜单按钮）
//! - 内容区有自己的 ToolbarView + HeaderBar（Home/Explore/Collection 导航）
//! - 侧边栏底部有 ViewSwitcher 切换 Player/Lyrics/Queue

use log::trace;
use relm4::gtk::prelude::{
    BoxExt, ButtonExt, Cast, OrientableExt, ToggleButtonExt, WidgetExt,
};
use relm4::gtk::Orientation;
use relm4::{adw, ComponentParts, ComponentSender, SimpleComponent, gtk};

use crate::icon_names;

/// Header 组件 — 整个应用的布局控制器
pub struct Header {
    sidebar_stack: adw::ViewStack,
    main_stack: adw::ViewStack,
    split_view: adw::OverlaySplitView,
    nav_buttons: Vec<gtk::Button>,
    sidebar_buttons: Vec<gtk::Button>,
}

#[derive(Debug)]
pub enum HeaderMsg {
    SwitchSidebarPage(String),
    ToggleSidebar,
    SwitchMainPage(String),
}

fn build_sidebar_page(icon: &str, title: &str, subtitle: &str) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .spacing(12)
        .build();

    page.append(&gtk::Image::builder()
        .icon_name(icon)
        .pixel_size(64)
        .opacity(0.4)
        .build());
    page.append(&gtk::Label::builder()
        .label(title)
        .css_classes(["title-2"])
        .build());
    page.append(&gtk::Label::builder()
        .label(subtitle)
        .opacity(0.6)
        .build());

    page
}

fn build_main_page(label: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(label)
        .css_classes(["title-1"])
        .vexpand(true)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .build()
}

#[relm4::component(pub)]
impl SimpleComponent for Header {
    view! {
        #[root]
        gtk::Box {}
    }

    type Init = ();
    type Input = HeaderMsg;
    type Output = ();

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // === 侧边栏 ===
        let sidebar_stack = adw::ViewStack::builder().name("sidebar_stack").build();

        sidebar_stack.add_titled(
            &build_sidebar_page(icon_names::MUSIC_NOTE_OUTLINE, "No song playing", "Player controls will appear here"),
            Some("player"),
            "Player",
        );
        sidebar_stack.add_titled(
            &build_sidebar_page(icon_names::CHAT_BUBBLE_TEXT, "Lyrics", "Lyrics will appear here"),
            Some("lyrics"),
            "Lyrics",
        );
        sidebar_stack.add_titled(
            &build_sidebar_page(icon_names::MUSIC_QUEUE, "Queue", "Queue is empty"),
            Some("queue"),
            "Queue",
        );
        sidebar_stack.set_visible_child_name("player");

        // 侧边栏底部按钮
        let sidebar_footer = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .homogeneous(true)
            .spacing(0)
            .margin_start(7)
            .margin_end(7)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        sidebar_footer.add_css_class("linked");

        let sidebar_pages = [
            ("player", icon_names::MUSIC_NOTE_OUTLINE, "Player"),
            ("lyrics", icon_names::CHAT_BUBBLE_TEXT, "Lyrics"),
            ("queue", icon_names::MUSIC_QUEUE, "Queue"),
        ];
        let mut sidebar_buttons = Vec::new();

        for (tag, icon, label) in sidebar_pages {
            let btn = gtk::Button::builder().hexpand(true).build();
            btn.set_child(Some(
                &adw::ButtonContent::builder()
                    .icon_name(icon)
                    .label(label)
                    .build(),
            ));
            if tag == "player" {
                btn.add_css_class("raised");
            } else {
                btn.add_css_class("flat");
            }
            let s = sender.clone();
            let t = tag.to_string();
            btn.connect_clicked(move |_| {
                s.input(HeaderMsg::SwitchSidebarPage(t.clone()));
            });
            sidebar_footer.append(&btn);
            sidebar_buttons.push(btn);
        }

        // 侧边栏 ToolbarView
        let sidebar_header = adw::HeaderBar::builder()
            .show_start_title_buttons(true)
            .show_end_title_buttons(true)
            .build();

        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&sidebar_header);
        sidebar_toolbar.set_content(Some(&sidebar_stack));
        sidebar_toolbar.add_bottom_bar(&sidebar_footer);
        sidebar_toolbar.set_bottom_bar_style(adw::ToolbarStyle::Flat);

        // === 主内容区 ===
        let main_stack = adw::ViewStack::builder().name("main_stack").build();
        main_stack.add_titled(&build_main_page("Home"), Some("home"), "Home");
        main_stack.add_titled(&build_main_page("Explore"), Some("explore"), "Explore");
        main_stack.add_titled(&build_main_page("Collection"), Some("collection"), "Collection");
        main_stack.set_visible_child_name("home");

        // 导航按钮
        let nav_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(3)
            .build();

        let nav_pages = [
            ("home", icon_names::GO_HOME, "Home"),
            ("explore", icon_names::COMPASS2, "Explore"),
            ("collection", icon_names::LIBRARY_MUSIC, "Collection"),
        ];
        let mut nav_buttons = Vec::new();

        for (tag, icon, label) in nav_pages {
            let btn = gtk::Button::builder()
                .icon_name(icon)
                .label(label)
                .build();
            if tag == "home" {
                btn.add_css_class("raised");
            } else {
                btn.add_css_class("flat");
            }
            let s = sender.clone();
            let t = tag.to_string();
            btn.connect_clicked(move |_| {
                s.input(HeaderMsg::SwitchMainPage(t.clone()));
            });
            nav_box.append(&btn);
            nav_buttons.push(btn);
        }

        // 侧边栏切换按钮
        let toggle_btn = gtk::ToggleButton::builder()
            .icon_name(icon_names::DOCK_LEFT)
            .tooltip_text("Toggle Sidebar")
            .active(true)
            .css_classes(["flat"])
            .build();

        let s = sender.clone();
        toggle_btn.connect_toggled(move |_| {
            s.input(HeaderMsg::ToggleSidebar);
        });

        // 内容区 HeaderBar
        let content_headerbar = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .title_widget(&nav_box)
            .build();
        content_headerbar.pack_start(&toggle_btn);

        // 内容区 ToolbarView
        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&content_headerbar);
        content_toolbar.set_content(Some(&main_stack));

        // === 根布局 ===
        let split_view = adw::OverlaySplitView::builder()
            .sidebar_position(gtk::PackType::Start)
            .sidebar_width_fraction(0.4)
            .min_sidebar_width(320.0)
            .max_sidebar_width(420.0)
            .show_sidebar(true)
            .sidebar(&sidebar_toolbar)
            .content(&content_toolbar)
            .build();

        let model = Self {
            sidebar_stack,
            main_stack,
            split_view: split_view.clone(),
            nav_buttons,
            sidebar_buttons,
        };

        // 把 split_view 放入 view! 创建的 root (Box) 中
        _root.append(&split_view);

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        trace!("Header: {message:?}");

        match message {
            HeaderMsg::SwitchSidebarPage(tag) => {
                self.sidebar_stack.set_visible_child_name(&tag);
                for btn in &self.sidebar_buttons {
                    btn.remove_css_class("raised");
                    btn.remove_css_class("flat");
                    btn.add_css_class("flat");
                }
                let idx = match tag.as_str() {
                    "player" => 0,
                    "lyrics" => 1,
                    "queue" => 2,
                    _ => return,
                };
                if let Some(btn) = self.sidebar_buttons.get(idx) {
                    btn.remove_css_class("flat");
                    btn.add_css_class("raised");
                }
            },
            HeaderMsg::ToggleSidebar => {
                let current = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!current);
            },
            HeaderMsg::SwitchMainPage(tag) => {
                self.main_stack.set_visible_child_name(&tag);
                for btn in &self.nav_buttons {
                    btn.remove_css_class("raised");
                    btn.remove_css_class("flat");
                    btn.add_css_class("flat");
                }
                let idx = match tag.as_str() {
                    "home" => 0,
                    "explore" => 1,
                    "collection" => 2,
                    _ => return,
                };
                if let Some(btn) = self.nav_buttons.get(idx) {
                    btn.remove_css_class("flat");
                    btn.add_css_class("raised");
                }
            },
        }
    }
}
