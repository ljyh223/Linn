use gtk::prelude::*;
use relm4::{
    gtk::{self, glib},
    loading_widgets::LoadingWidgets, ComponentParts, ComponentSender, SimpleComponent,
};
use std::sync::Arc;

use crate::widgets::{navigation::Navigation, player_bar::PlayerBar};

pub struct App {
    // 导航栏控制器
    navigation: Controller<Navigation>,
    // 播放栏控制器
    player_bar: Controller<PlayerBar>,
    // API 客户端（后续添加）
    // api: Arc<MusicApi>,
}

#[derive(Debug)]
pub enum AppMsg {
    // 导航项被点击
    NavigationClicked(NavigationItem),
    // 退出应用
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    // 为我推荐
    Recommend,
    // 发现音乐
    Discover,
    // 我的收藏
    MyCollection,
    // 我喜欢的音乐
    MyFavorites,
}

impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Widgets = AppWidgets;

    fn init_root() -> gtk::Application {
        gtk::Application::builder()
            .application_id("com.github.linn")
            .build()
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 创建导航栏组件
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

        // 创建播放栏组件
        let player_bar = PlayerBar::builder().launch(()).detach();

        let model = App {
            navigation,
            player_bar,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::NavigationClicked(item) => {
                eprintln!("导航项被点击: {:?}", item);
                // TODO: 根据导航项更新右侧内容区
            }
            AppMsg::Exit => {
                // TODO: 清理资源
            }
        }
    }

    fn pre_view() {
        // 可以在这里添加预处理逻辑
    }
}

relm4::view! {
    #[name(main_window)]
    gtk::ApplicationWindow {
        set_title: Some("Linn - 网易云音乐"),
        set_default_width: 1200,
        set_default_height: 800,

        // 使用 AdwApplicationWindow 或 GtkBox 作为主布局
        #[wrap(Some)]
        set_content = &gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 0,

            // 主内容区（包含左侧导航和右侧内容）
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_vexpand: true,

                // 左侧导航栏
                append = model.navigation.widget(), {
                    set_width_request: 200,
                }

                // 右侧内容区（占位）
                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_css_classes: &["content-area"],

                    append = &gtk::Label {
                        set_label: "欢迎使用 Linn",
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_hexpand: true,
                        set_vexpand: true,
                    }
                }
            }

            // 底部播放栏
            append = model.player_bar.widget(), {
                set_height_request: 80,
            }
        }
    }
}
