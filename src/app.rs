use relm4::{
    component::{Component, ComponentController, Controller},
    gtk::{self, prelude::*}, ComponentParts, ComponentSender, SimpleComponent,
};

use crate::widgets::{navigation::Navigation, player_bar::PlayerBar};

pub struct App {
    navigation: Controller<Navigation>,
    player_bar: Controller<PlayerBar>,
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

                    #[local_ref]
                    navigation_widget -> gtk::Box {
                        set_width_request: 200,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_css_classes: &["content-area"],

                        gtk::Label {
                            set_label: "欢迎使用 Linn",
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_hexpand: true,
                            set_vexpand: true,
                        }
                    }
                },

                #[local_ref]
                player_widget -> gtk::Box {
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
                crate::widgets::navigation::NavigationOutput::Recommend => {
                    AppMsg::NavigationClicked(NavigationItem::Recommend)
                }
                crate::widgets::navigation::NavigationOutput::Discover => {
                    AppMsg::NavigationClicked(NavigationItem::Discover)
                }
                crate::widgets::navigation::NavigationOutput::MyCollection => {
                    AppMsg::NavigationClicked(NavigationItem::MyCollection)
                }
                crate::widgets::navigation::NavigationOutput::MyFavorites => {
                    AppMsg::NavigationClicked(NavigationItem::MyFavorites)
                }
            });

        let player_bar = PlayerBar::builder().launch(()).detach();

        let model = App {
            navigation,
            player_bar,
        };

        let navigation_widget = model.navigation.widget().clone();
        let player_widget = model.player_bar.widget().clone();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::NavigationClicked(item) => {
                eprintln!("导航项被点击: {:?}", item);
            }
            AppMsg::Exit => {}
        }
    }
}
