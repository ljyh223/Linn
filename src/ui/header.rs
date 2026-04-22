//! Header component — 纯粹的顶部导航栏

use std::sync::Arc;

use relm4::adw::{self, ButtonContent};
use relm4::gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, gtk, Component}; 
use crate::api::{UserInfo, get_user_info};
use crate::ui::components::image::AsyncImage;
use crate::ui::route::{self, AppRoute};

pub struct Header {
    can_go_back: bool,
    current_tab: AppRoute,
    user_info: Arc<UserInfo>,
}

#[derive(Debug)]
pub enum HeaderMsg {
    GoBackClicked,
    TabClicked(AppRoute),
    SidebarToggleClicked,
    OpenSettingsClicked,
    UpdateState { can_go_back: bool, active_tab: AppRoute },
    UpdateUserInfo(Arc<UserInfo>),
}

// 向上层抛出的路由事件 (【修改】增加了 OpenSettings)
#[derive(Debug)]
pub enum HeaderOutput {
    GoBack,
    NavigateTo(AppRoute),
    ToggleSidebar,
    OpenSettings,
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
                    set_tooltip_text: Some("Toggle Sidebar"),
                    connect_clicked => HeaderMsg::SidebarToggleClicked,
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

            gtk::Box {
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
        };
        let widgets = view_output!();
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
            HeaderMsg::UpdateState { can_go_back, active_tab } => {
                self.can_go_back = can_go_back;
                self.current_tab = active_tab;
            }
            HeaderMsg::SidebarToggleClicked => {
                sender.output(HeaderOutput::ToggleSidebar).unwrap();
            },
            HeaderMsg::OpenSettingsClicked => {
                // 【修改】将事件向上抛出给 Window
                sender.output(HeaderOutput::OpenSettings).unwrap();
            }
            HeaderMsg::UpdateUserInfo(user_info) => {
                self.user_info = user_info;
            }
        }
    }
}
