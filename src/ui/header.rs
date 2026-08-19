//! Header component — 纯粹的顶部导航栏

use std::sync::Arc;

use crate::api::UserInfo;
use crate::ui::components::image::AsyncImage;
use crate::ui::route::AppRoute;
use relm4::adw::{self};
use relm4::gtk::prelude::*;
use relm4::{Component, ComponentParts, ComponentSender, gtk};

pub struct Header {
    can_go_back: bool,
    current_tab: AppRoute,
    user_info: Arc<UserInfo>,
    /// 导航/搜索 切换 Stack
    nav_stack: gtk::Stack,
    /// 搜索输入框
    search_entry: gtk::SearchEntry,
}

#[derive(Debug)]
pub enum HeaderMsg {
    GoBackClicked,
    TabClicked(AppRoute),
    FullscreenClicked,
    OpenSettingsClicked,
    UpdateState {
        can_go_back: bool,
        active_tab: AppRoute,
    },
    UpdateUserInfo(Arc<UserInfo>),
    /// 搜索输入框回车
    SearchAccepted,
    /// 搜索输入框内容变化
    SearchChanged,
}

// 向上层抛出的路由事件 (【修改】增加了 OpenSettings)
#[derive(Debug)]
pub enum HeaderOutput {
    GoBack,
    NavigateTo(AppRoute),
    /// 进入/退出全屏歌词页
    ToggleFullscreen,
    OpenSettings,
    /// 在搜索页提交了搜索词
    SearchSubmit(String),
    /// 搜索输入框内容变化（用于实时建议）
    SearchChanged(String),
}

#[relm4::component(pub)]
impl Component for Header {
    type Init = Arc<UserInfo>;
    type Input = HeaderMsg;
    type Output = HeaderOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 16,
            set_margin_top: 8,
            set_margin_bottom: 8,
            set_margin_start: 16,
            set_margin_end: 16,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                gtk::Button {
                    set_icon_name: "sidebar-show-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("全屏歌词"),
                    connect_clicked => HeaderMsg::FullscreenClicked,
                },
                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    add_css_class: "circular",
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: model.can_go_back,
                    connect_clicked => HeaderMsg::GoBackClicked,
                },
            },

            gtk::Box { set_hexpand: true },

            // 搜索页时显示搜索输入框，否则显示导航按钮
            #[name(nav_stack)]
            gtk::Stack {
                set_transition_type: gtk::StackTransitionType::Crossfade,
                set_hexpand: true,
                set_halign: gtk::Align::Center,

                add_named[Some("nav")] = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,

                    gtk::ToggleButton {
                        add_css_class: "flat",
                        #[wrap(Some)]
                        set_child = &adw::ButtonContent {
                            set_icon_name: "go-home-symbolic",
                            set_label: "Home",
                        },
                        #[watch]
                        set_active: model.current_tab == AppRoute::Home,
                        connect_clicked => HeaderMsg::TabClicked(AppRoute::Home),
                    },

                    gtk::ToggleButton {
                        add_css_class: "flat",
                        #[wrap(Some)]
                        set_child = &adw::ButtonContent {
                            set_icon_name: "compass2",
                            set_label: "Explore",
                        },
                        #[watch]
                        set_active: model.current_tab == AppRoute::Explore,
                        connect_clicked => HeaderMsg::TabClicked(AppRoute::Explore),
                    },

                    gtk::ToggleButton {
                        add_css_class: "flat",
                        #[wrap(Some)]
                        set_child = &adw::ButtonContent {
                            // 建议改成 library-music-symbolic
                            set_icon_name: "library-music-symbolic",
                            set_label: "Collection",
                        },
                        #[watch]
                        set_active: model.current_tab == AppRoute::Collection,
                        connect_clicked => HeaderMsg::TabClicked(AppRoute::Collection),
                    },
                },

                add_named[Some("search")] = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    #[name(search_entry)]
                    gtk::SearchEntry {
                        set_width_request: 320,
                        set_hexpand: true,
                        set_placeholder_text: Some("搜索歌曲 / 歌单 / 歌手..."),
                        connect_activate => HeaderMsg::SearchAccepted,
                        connect_search_changed => HeaderMsg::SearchChanged,
                    },
                },
            },

            gtk::Box { set_hexpand: true },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                AsyncImage{
                    set_width_request: 32,
                    set_height_request: 32,
                    set_margin_end: 8,
                    set_corner_radius: 16.0,
                    #[watch]
                    set_url: format!("{}?param=100y100",model.user_info.avatar_url.clone()),

                },

                gtk::Button {
                    set_icon_name: "settings-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Settings"),
                    connect_clicked => HeaderMsg::OpenSettingsClicked,
                },
            }
        }
    }

    fn init(
        user_info: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            can_go_back: false,
            current_tab: AppRoute::Home,
            user_info: user_info,
            nav_stack: gtk::Stack::default(),
            search_entry: gtk::SearchEntry::default(),
        };
        let widgets = view_output!();

        let mut model = model;
        model.nav_stack = widgets.nav_stack.clone();
        model.search_entry = widgets.search_entry.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            HeaderMsg::GoBackClicked => {
                sender.output(HeaderOutput::GoBack).unwrap();
            }
            HeaderMsg::TabClicked(tab) => {
                self.current_tab = tab.clone();
                sender.output(HeaderOutput::NavigateTo(tab)).unwrap();
            }
            HeaderMsg::UpdateState {
                can_go_back,
                active_tab,
            } => {
                self.can_go_back = can_go_back;
                self.current_tab = active_tab;
                // 根据路由切换导航/搜索输入框
                match self.current_tab {
                    AppRoute::Search => {
                        self.nav_stack.set_visible_child_name("search");
                        self.search_entry.grab_focus();
                    }
                    _ => {
                        self.nav_stack.set_visible_child_name("nav");
                    }
                }
            }
            HeaderMsg::FullscreenClicked => {
                sender.output(HeaderOutput::ToggleFullscreen).unwrap();
            }
            HeaderMsg::OpenSettingsClicked => {
                // 【修改】将事件向上抛出给 Window
                sender.output(HeaderOutput::OpenSettings).unwrap();
            }
            HeaderMsg::UpdateUserInfo(user_info) => {
                self.user_info = user_info;
            }
            HeaderMsg::SearchAccepted => {
                let query = self.search_entry.text().to_string();
                if !query.is_empty() {
                    sender.output(HeaderOutput::SearchSubmit(query)).unwrap();
                }
            }
            HeaderMsg::SearchChanged => {
                let query = self.search_entry.text().to_string();
                sender.output(HeaderOutput::SearchChanged(query)).unwrap();
            }
        }
    }
}
