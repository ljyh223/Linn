//! 主应用组件
//!
//! 应用的顶层组件，负责管理窗口、导航栏、播放栏和页面切换。

use gtk::prelude::*;
use relm4::{
    component::{Component, ComponentController, Controller},
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};

use crate::components::{Navigation, PlayerBar};
use crate::pages::{Collection, Discover, Favorites, Recommend};

pub struct App {
    navigation: Controller<Navigation>,
    player_bar: Controller<PlayerBar>,
    // 页面组件
    discover: Controller<Discover>,
}

#[derive(Debug)]
pub enum AppMsg {
    NavigationClicked(NavigationItem),
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    Recommend,
    Discover,
    MyCollection,
    MyFavorites,
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::ApplicationWindow {
            set_title: Some("Linn - 网易云音乐"),
            set_default_width: 1200,
            set_default_height: 800,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_hexpand: true,
                    set_vexpand: true,

                    model.navigation.widget().clone() -> gtk::Box {
                        set_width_request: 200,
                        set_hexpand: false,
                    },

                    model.discover.widget().clone() -> gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                    }
                },

                model.player_bar.widget().clone() -> gtk::Box {
                    set_height_request: 80,
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let navigation = Navigation::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                crate::components::NavigationOutput::Recommend => {
                    AppMsg::NavigationClicked(NavigationItem::Recommend)
                }
                crate::components::NavigationOutput::Discover => {
                    AppMsg::NavigationClicked(NavigationItem::Discover)
                }
                crate::components::NavigationOutput::MyCollection => {
                    AppMsg::NavigationClicked(NavigationItem::MyCollection)
                }
                crate::components::NavigationOutput::MyFavorites => {
                    AppMsg::NavigationClicked(NavigationItem::MyFavorites)
                }
            });

        let player_bar = PlayerBar::builder().launch(()).detach();

        // 创建 Discover 页面
        let discover = Discover::builder().launch(()).detach();
        let discover_widget = discover.widget().clone();

        let model = App {
            navigation,
            player_bar,
            discover,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::NavigationClicked(item) => {
                eprintln!("导航项被点击: {:?}", item);
                // TODO: 切换页面
            }
            AppMsg::Exit => {}
        }
    }
}
