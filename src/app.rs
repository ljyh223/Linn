//! 主应用组件
//!
//! 应用的顶层组件，负责管理窗口、导航栏、播放栏和页面切换。

use gtk::prelude::*;
use relm4::{
    component::{Component, ComponentController, Controller},
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};

use crate::components::{Navigation, PlayerBar};
use crate::pages::{Collection, Discover, DiscoverOutput, Favorites, PlaylistDetail, Recommend};

pub struct App {
    navigation: Controller<Navigation>,
    player_bar: Controller<PlayerBar>,
    // 页面组件
    discover: Controller<Discover>,
    recommend: Controller<Recommend>,
    collection: Controller<Collection>,
    favorites: Controller<Favorites>,
    playlist_detail: Option<Controller<PlaylistDetail>>,
    // 页面切换的 Stack
    page_stack: gtk::Stack,
    // 用于跟踪是否已添加歌单详情页面
    playlist_detail_added: bool,
    // 页面 widget 引用（用于切换）- 使用 Widget trait object
    discover_widget: gtk::Widget,
    recommend_widget: gtk::Widget,
    collection_widget: gtk::Widget,
    favorites_widget: gtk::Widget,
    playlist_detail_widget: Option<gtk::Widget>,
}

#[derive(Debug)]
pub enum AppMsg {
    NavigationClicked(NavigationItem),
    OpenPlaylistDetail(u64),
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

                    #[name = "navigation_box"]
                    gtk::Box {
                        set_width_request: 200,
                        set_hexpand: false,
                    },

                    #[name = "page_stack"]
                    gtk::Stack {
                        set_hexpand: true,
                        set_vexpand: true,
                    }
                },

                #[name = "player_bar_box"]
                gtk::Box {
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

        // 创建所有页面组件
        let discover = Discover::builder()
            .launch(())
            .forward(sender.input_sender(), |output| {
                match output {
                    DiscoverOutput::PlaylistClicked(id) => AppMsg::OpenPlaylistDetail(id),
                }
            });
        let recommend = Recommend::builder().launch(()).detach();
        let collection = Collection::builder().launch(()).detach();
        let favorites = Favorites::builder().launch(()).detach();

        let widgets = view_output!();

        // 手动添加子组件到容器
        widgets.navigation_box.append(navigation.widget());
        widgets.player_bar_box.append(player_bar.widget());

        // 添加所有页面到 Stack，使用 add_named 指定名称
        widgets.page_stack.add_child(discover.widget());
        widgets.page_stack.add_child(recommend.widget());
        widgets.page_stack.add_child(collection.widget());
        widgets.page_stack.add_child(favorites.widget());

        // playlist_detail 页面将在需要时动态添加

        // 设置默认显示 discover 页面（第一个添加的子 widget）
        widgets.page_stack.set_visible_child(discover.widget());

        // 保存 Stack 和 widget 引用以便在 update 中使用
        let page_stack = widgets.page_stack.clone();
        let discover_widget = discover.widget().clone().upcast::<gtk::Widget>();
        let recommend_widget = recommend.widget().clone().upcast::<gtk::Widget>();
        let collection_widget = collection.widget().clone().upcast::<gtk::Widget>();
        let favorites_widget = favorites.widget().clone().upcast::<gtk::Widget>();

        let model = App {
            navigation,
            player_bar,
            discover,
            recommend,
            collection,
            favorites,
            playlist_detail: None,
            page_stack,
            playlist_detail_added: false,
            discover_widget,
            recommend_widget,
            collection_widget,
            favorites_widget,
            playlist_detail_widget: None,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::NavigationClicked(item) => {
                eprintln!("导航项被点击: {:?}", item);
                let widget = match item {
                    NavigationItem::Recommend => &self.recommend_widget,
                    NavigationItem::Discover => &self.discover_widget,
                    NavigationItem::MyCollection => &self.collection_widget,
                    NavigationItem::MyFavorites => &self.favorites_widget,
                };
                self.page_stack.set_visible_child(widget);
            }
            AppMsg::OpenPlaylistDetail(id) => {
                eprintln!("打开歌单详情: {}", id);

                // 如果还没有添加 playlist_detail 页面，创建并添加它
                if !self.playlist_detail_added {
                    let playlist_detail = PlaylistDetail::builder()
                        .launch(id)
                        .detach();
                    let widget = playlist_detail.widget().clone().upcast::<gtk::Widget>();

                    self.page_stack.add_child(&widget);

                    self.playlist_detail = Some(playlist_detail);
                    self.playlist_detail_widget = Some(widget);
                    self.playlist_detail_added = true;
                } else {
                    // 歌单详情页面已经存在，直接切换即可
                    // TODO: 未来可以发送消息更新歌单 ID
                }

                // 切换到歌单详情页面
                if let Some(ref widget) = self.playlist_detail_widget {
                    self.page_stack.set_visible_child(widget);
                }
            }
            AppMsg::Exit => {}
        }
    }
}
