//! 歌单详情页组件

use log::{info, trace};
use relm4::gtk::glib::BoxedAnyObject;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, gtk};

use crate::api::{Album, Artist, Song};
use crate::async_image;

// --- 2. 定义组件状态与消息 ---

pub struct PlaylistDetail {
    playlist_id: u64,

    // Header 信息
    title: String,
    creator: String,
    cover_url: String,
    description: String,

    // 虚拟列表的数据源
    list_store: gtk::gio::ListStore,

    // 用于动态装载顶部封面的容器
    header_cover_container: gtk::Box,
}

#[derive(Debug)]
pub enum PlaylistDetailMsg {
    // 生命周期 & 数据加载
    LoadPlaylist(u64),
    PlaylistLoaded, // 占位：实际中应携带 API 返回的数据

    // Header 上的功能按钮点击
    PlayAllClicked,
    LikeClicked,

    // 列表 Item 上的功能按钮点击
    TrackPlayClicked(u64),
    TrackMoreClicked(u64),
}

#[derive(Debug)]
pub enum PlaylistDetailOutput {}

#[relm4::component(pub)]
impl Component for PlaylistDetail {
    type Init = u64; // 传入 Playlist ID
    type Input = PlaylistDetailMsg;
    type CommandOutput = ();
    type Output = PlaylistDetailOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            // ==========================================
            // 上方区域：Header (封面、名称、功能按钮)
            // ==========================================
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 24,
                set_margin_all: 32,

                // 1. 左侧：封面容器 (在 init 中动态插入 AsyncImage)
                #[name(header_cover_container)]
                gtk::Box {
                    set_size_request: (200, 200),
                },

                // 2. 右侧：文本信息与操作按钮
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_valign: gtk::Align::Center,

                    // 歌单标题
                    gtk::Label {
                        #[watch]
                        set_label: &model.title,
                        add_css_class: "title-1",
                        set_halign: gtk::Align::Start,
                    },

                    // 创建者
                    gtk::Label {
                        #[watch]
                        set_label: &model.creator,
                        add_css_class: "dim-label",
                        set_halign: gtk::Align::Start,
                    },

                    // 简介
                    gtk::Label {
                        #[watch]
                        set_label: &model.description,
                        set_wrap: true,
                        set_max_width_chars: 60,
                        set_halign: gtk::Align::Start,
                    },

                    // 占据剩余空间，把按钮往下推（可选）
                    gtk::Box { set_vexpand: true },

                    // 功能按钮 Row
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,

                        // 播放全部
                        gtk::Button {
                            set_label: "播放全部",
                            set_icon_name: "media-playback-start-symbolic",
                            add_css_class: "suggested-action", // 醒目强调色
                            add_css_class: "pill", // 胶囊形状
                            connect_clicked => PlaylistDetailMsg::PlayAllClicked,
                        },

                        // 收藏按钮
                        gtk::Button {
                            set_icon_name: "emblem-favorite-symbolic",
                            set_tooltip_text: Some("收藏歌单"),
                            add_css_class: "circular",
                            connect_clicked => PlaylistDetailMsg::LikeClicked,
                        }
                    }
                }
            },

            // ==========================================
            // 下方区域：虚拟列表 (GtkListView)
            // ==========================================
            gtk::ScrolledWindow {
                set_vexpand: true, // 占据下方所有剩余空间
                set_hscrollbar_policy: gtk::PolicyType::Never,

                // GtkListView 必须包裹在 ScrolledWindow 中
                #[name(list_view)]
                gtk::ListView {
                    add_css_class: "navigation-sidebar", // 使用自带的优美列表样式
                }
            }
        }

    }

    fn init(
        playlist_id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 创建用于 GtkListView 的数据源 (ListStore 存储 glib::BoxedAnyObject)
        let list_store = gtk::gio::ListStore::new::<BoxedAnyObject>();

        let mut model = Self {
            playlist_id,
            title: "加载中...".to_string(),
            creator: "".to_string(),
            cover_url: "".to_string(),
            description: "".to_string(),
            list_store: list_store.clone(),
            header_cover_container: gtk::Box::default(),
        };

        let widgets = view_output!();
        model.header_cover_container = widgets.header_cover_container.clone();

        // --- 设置虚拟列表的 Model 和 Factory ---

        // 1. 设置 Selection Model
        let selection_model = gtk::SingleSelection::new(Some(list_store.clone()));
        widgets.list_view.set_model(Some(&selection_model));

        // 2. 创建并设置 Factory（处理 UI 的生成与数据绑定）
        let factory = setup_list_factory(sender.clone());
        widgets.list_view.set_factory(Some(&factory));

        // 触发加载数据
        sender.input(PlaylistDetailMsg::LoadPlaylist(playlist_id));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        trace!("PlaylistDetail Msg: {:?}", message);
        match message {
            PlaylistDetailMsg::LoadPlaylist(id) => {
                info!("加载歌单 ID: {}", id);

                // TODO: 在这里发起异步 API 请求
                // 暂时使用 Mock 数据模拟加载完成
                self.title = "2024 年度最爱听的独立音乐".to_string();
                self.creator = "创建者：Linn".to_string();
                self.description = "这是一段关于该歌单的详细描述，包含了一些氛围介绍。".to_string();
                self.cover_url = "https://example.com/mock-cover.jpg".to_string(); // 替换为真实 URL

                // 生成顶部的 Cover AsyncImage
                while let Some(child) = self.header_cover_container.first_child() {
                    self.header_cover_container.remove(&child);
                }
                let cover_img = async_image!(
                    &self.cover_url,
                    size: (200, 200),
                    radius: Lg, // 圆角
                );
                // 假设你的 AsyncImage 结构体可以获取底层 widget
                // 如果你的 AsyncImage 就是 Widget，直接传 &cover_img
                // 如果它内部包裹了 Widget，调用如 cover_img.widget()
                self.header_cover_container.append(cover_img.widget());

                // Mock 列表数据插入 ListStore
                self.list_store.remove_all();
                for i in 1..=50 {
                    let track = Song {
                        id: i,
                        name: format!("独立音乐 Track {}", i),
                        artists: vec![Artist {
                            id: i * 5,
                            name: format!("Artist {}", i % 5),
                            cover_url: "https://example.com/mock-artist-cover.jpg".to_string(),
                        }],
                        album: Album {
                            id: i * 10,
                            name: format!("Album {}", i % 3),
                            cover_url: "https://example.com/mock-album-cover.jpg".to_string(),
                        },
                        cover_url: "https://example.com/mock-track-cover.jpg".to_string(),
                        duration: 1000,
                    };
                    // 用 BoxedAnyObject 包装 Rust 结构体存入 ListStore
                    let obj = BoxedAnyObject::new(track);
                    self.list_store.append(&obj);
                }
            }
            PlaylistDetailMsg::PlaylistLoaded => {}
            PlaylistDetailMsg::PlayAllClicked => {
                info!("点击了播放全部");
            }
            PlaylistDetailMsg::LikeClicked => {
                info!("点击了收藏");
            }
            PlaylistDetailMsg::TrackPlayClicked(track_id) => {
                info!("点击了列表播放，音轨 ID: {}", track_id);
            }
            PlaylistDetailMsg::TrackMoreClicked(track_id) => {
                info!("点击了列表更多选项，音轨 ID: {}", track_id);
            }
        }
    }
}

// =========================================================
// 核心逻辑：GtkListView 的 UI 生成与数据绑定 (虚拟滚动)
// =========================================================
fn setup_list_factory(sender: ComponentSender<PlaylistDetail>) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    // 1. Setup 阶段：生成每一行的空白 UI 骨架（只执行几次，用于回收复用）
    let sender_for_setup = sender.clone();
    factory.connect_setup(move |_factory, list_item| {
        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

        // 使用 CenterBox 实现完美的左中右三段式布局
        let center_box = gtk::CenterBox::new();
        center_box.set_margin_top(8);
        center_box.set_margin_bottom(8);
        center_box.set_margin_start(16);
        center_box.set_margin_end(16);

        // --- 左侧：封面 + 标题 + 歌手 ---
        let start_box = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        let cover_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let text_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        text_box.set_valign(gtk::Align::Center);

        let title_label = gtk::Label::new(None);
        title_label.set_halign(gtk::Align::Start);
        title_label.add_css_class("heading");

        let artist_label = gtk::Label::new(None);
        artist_label.set_halign(gtk::Align::Start);
        artist_label.add_css_class("dim-label");

        text_box.append(&title_label);
        text_box.append(&artist_label);
        start_box.append(&cover_container);
        start_box.append(&text_box);
        center_box.set_start_widget(Some(&start_box));

        // --- 中间：专辑名 ---
        let album_label = gtk::Label::new(None);
        album_label.add_css_class("dim-label");
        // 限制最大宽度并允许截断
        album_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        album_label.set_max_width_chars(20);
        center_box.set_center_widget(Some(&album_label));

        // --- 右侧：功能按钮 ---
        let end_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        let play_btn = gtk::Button::from_icon_name("media-playback-start-symbolic");
        play_btn.add_css_class("circular");
        play_btn.add_css_class("flat");

        let more_btn = gtk::Button::from_icon_name("view-more-symbolic");
        more_btn.add_css_class("circular");
        more_btn.add_css_class("flat");

        end_box.append(&play_btn);
        end_box.append(&more_btn);
        center_box.set_end_widget(Some(&end_box));

        // 【精髓】：按钮点击事件绑定
        // 由于 Setup 只运行一次，此时没有具体数据。我们通过读取按钮的 `widget_name` 来获取当前绑定的 track_id
        let s1 = sender_for_setup.clone();
        play_btn.connect_clicked(move |btn| {
            if let Ok(id) = btn.widget_name().parse::<u64>() {
                s1.input(PlaylistDetailMsg::TrackPlayClicked(id));
            }
        });

        let s2 = sender_for_setup.clone();
        more_btn.connect_clicked(move |btn| {
            if let Ok(id) = btn.widget_name().parse::<u64>() {
                s2.input(PlaylistDetailMsg::TrackMoreClicked(id));
            }
        });

        // 将根节点放入 ListItem
        list_item.set_child(Some(&center_box));
    });

    // 2. Bind 阶段：将真实数据填入上方构建的 UI 骨架中（滚动时高频执行）
    factory.connect_bind(move |_factory, list_item| {
        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

        // 提取数据
        let data_obj = list_item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let track = data_obj.borrow::<Song>();

        // 从嵌套中安全提取各个 UI 控件
        let center_box = list_item.child().and_downcast::<gtk::CenterBox>().unwrap();

        // 左侧
        let start_box = center_box
            .start_widget()
            .and_downcast::<gtk::Box>()
            .unwrap();
        let cover_container = start_box.first_child().and_downcast::<gtk::Box>().unwrap();
        let text_box = cover_container
            .next_sibling()
            .and_downcast::<gtk::Box>()
            .unwrap();
        let title_label = text_box.first_child().and_downcast::<gtk::Label>().unwrap();
        let artist_label = title_label
            .next_sibling()
            .and_downcast::<gtk::Label>()
            .unwrap();

        // 中间
        let album_label = center_box
            .center_widget()
            .and_downcast::<gtk::Label>()
            .unwrap();

        // 右侧
        let end_box = center_box.end_widget().and_downcast::<gtk::Box>().unwrap();
        let play_btn = end_box.first_child().and_downcast::<gtk::Button>().unwrap();
        let more_btn = play_btn
            .next_sibling()
            .and_downcast::<gtk::Button>()
            .unwrap();

        // 绑定文本数据
        title_label.set_label(&track.name);
        artist_label.set_label(
            &track
                .artists
                .first()
                .map(|a| &a.name)
                .unwrap_or(&String::new()),
        );
        album_label.set_label(&track.album.name);

        // 利用 widget_name 巧妙保存 track_id，供 Setup 阶段绑定的闭包读取
        play_btn.set_widget_name(&track.id.to_string());
        more_btn.set_widget_name(&track.id.to_string());

        // 动态加载封面图片
        while let Some(child) = cover_container.first_child() {
            cover_container.remove(&child);
        }
        let track_cover = async_image!(
            &track.cover_url,
            size: (40, 40),
            radius: Md, // 列表中等圆角
        );
        // 注意：假设你的 AsyncImage 能够解构为 GTK Widget
        cover_container.append(track_cover.widget());
    });

    factory
}
