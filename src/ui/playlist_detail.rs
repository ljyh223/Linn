//! 歌单详情页组件

use log::{info, trace};
use relm4::gtk::glib::{BoxedAnyObject, MainContext};
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, gtk};

use crate::api::model::AlbumDetail;
use crate::api::{PlaylistDetail as PlaylistDetailModel, Song, get_playlist_detail};


pub struct MusicCollection {
    pub id: u64,

    pub title: String,
    pub subtitle: String,     // 作者 / 艺术家
    pub description: String,
    pub cover_url: String,

    pub tracks: Vec<Song>,
    pub track_ids: Vec<i64>,
}

impl From<PlaylistDetailModel> for MusicCollection {
    fn from(detail: PlaylistDetailModel) -> Self {
        Self {
            id: detail.id as u64,
            title: detail.name,
            subtitle: format!("创建者：{}", detail.creator_name),
            description: detail.description,
            cover_url: detail.cover_url,
            tracks: detail.tracks.clone(),
            track_ids: detail.track_ids,
        }
    }
}

impl From<AlbumDetail> for MusicCollection {
    fn from(detail: AlbumDetail) -> Self {
        Self {
            id: detail.id as u64,
            title: detail.name,
            subtitle: format!("艺术家：{}", detail.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")),
            description: detail.description,
            cover_url: detail.cover_url,
            tracks: detail.tracks.clone(),
            track_ids: detail.tracks.iter().map(|s| s.id).collect(),
        }
    }
}



pub struct PlaylistDetail {
    playlist_id: u64,

    track_ids: Vec<i64>,
    tracks: Vec<Song>,

    // Header 信息
    title: String,
    creator: String,
    cover_url: String,
    description: String,

    // 虚拟列表的数据源
    list_store: gtk::gio::ListStore,

    // 用于动态装载顶部封面的容器
    header_cover_container: gtk::Box,
    main_stack: gtk::Stack,
}

#[derive(Debug)]
pub enum PlaylistDetailMsg {
    // 生命周期 & 数据加载
    LoadPlaylist(u64),
    PlaylistLoaded(PlaylistDetailModel),
    // Header 上的功能按钮点击
    PlayAllClicked,
    LikeClicked,

    // 列表 Item 上的功能按钮点击
    TrackPlayClicked(i64),
    TrackMoreClicked(i64),
}

#[derive(Debug)]
pub enum PlaylistDetailOutput {
    PlayQueue(Vec<Song>, Vec<i64>, usize),
}

#[relm4::component(pub)]
impl Component for PlaylistDetail {
    type Init = u64; // 传入 Playlist ID
    type Input = PlaylistDetailMsg;
    type CommandOutput = PlaylistDetailOutput;
    type Output = PlaylistDetailOutput;

    view! {
        #[root]
        #[name(main_stack)]
        gtk::Stack {
            // 设置淡入淡出动画，让加载完成的瞬间显得很高级
            set_transition_type: gtk::StackTransitionType::Crossfade,

            // ==========================================
            // 状态 1：加载中动画页面
            // ==========================================
            add_named[Some("loading")] = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center, // 绝对居中
                set_spacing: 16,

                // 原生的旋转菊花
                gtk::Spinner {
                    set_spinning: true,
                    set_width_request: 48,
                    set_height_request: 48,
                },

                gtk::Label {
                    set_label: "正在加载歌单...",
                    add_css_class: "dim-label",
                }
            },

            // ==========================================
            // 状态 2：真实内容页面 (你之前的代码放这里)
            // ==========================================
            add_named[Some("content")] = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                

                // --- 你的 Header 区域 ---
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Start,
                    set_spacing: 24,
                    set_margin_all: 32,

                    #[name(header_cover_container)]
                    gtk::Box {
                            set_width_request: 150,
                            set_height_request: 150,
                            set_halign: gtk::Align::Start,  // ← 不要 Fill，否则会横向拉伸
                            set_valign: gtk::Align::Start,
                            // set_vexpand: false,
                            // set_hexpand: false,
                            set_overflow: gtk::Overflow::Hidden,
                     },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_valign: gtk::Align::Center,

                        gtk::Label { #[watch] set_label: &model.title, add_css_class: "title-1", set_halign: gtk::Align::Start },
                        gtk::Label { #[watch] set_label: &model.creator, add_css_class: "dim-label", set_halign: gtk::Align::Start },
                        gtk::Label { #[watch] set_label: &model.description, set_wrap: true, set_max_width_chars: 60, set_halign: gtk::Align::Start },

                        // 功能按钮 Row
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            gtk::Button { set_label: "播放全部", set_icon_name: "media-playback-start-symbolic", add_css_class: "suggested-action", add_css_class: "pill", connect_clicked => PlaylistDetailMsg::PlayAllClicked },
                            gtk::Button { set_icon_name: "xsi-emblem-favorite-symbolic", add_css_class: "circular", connect_clicked => PlaylistDetailMsg::LikeClicked, set_margin_start: 8, set_margin_end: 8}
                        }
                    }
                },

                // --- 你的 ListView 区域 ---
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[name(list_view)]
                    gtk::ListView { add_css_class: "navigation-sidebar" }
                }
            }
        }
    }

    fn init(
        playlist_id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_store = gtk::gio::ListStore::new::<BoxedAnyObject>();

        let mut model = Self {
            playlist_id,
            track_ids: Vec::new(),
            tracks: Vec::new(),
            title: String::new(),
            creator: String::new(),
            cover_url: String::new(),
            description: String::new(),
            list_store: list_store.clone(),
            header_cover_container: gtk::Box::default(),
            main_stack: gtk::Stack::default(), // 初始化
        };

        let widgets = view_output!();
        model.header_cover_container = widgets.header_cover_container.clone();
        model.main_stack = widgets.main_stack.clone(); // 保存 Stack 引用

        // 默认显示加载动画
        model.main_stack.set_visible_child_name("loading");

        let selection_model = gtk::SingleSelection::new(Some(list_store.clone()));
        widgets.list_view.set_model(Some(&selection_model));

        let factory = setup_list_factory(sender.clone());
        widgets.list_view.set_factory(Some(&factory));

        sender.input(PlaylistDetailMsg::LoadPlaylist(playlist_id));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        trace!("PlaylistDetail Msg: {:?}", message);
        match message {
            PlaylistDetailMsg::LoadPlaylist(id) => {
                info!("开始加载歌单 ID: {}", id);

                // 确保此时显示的是 loading 页面 (从其他页面进来可能已经是，但保险起见再设一次)
                self.main_stack.set_visible_child_name("loading");

                let ctx = MainContext::default();
                ctx.spawn_local(async move {
                    match get_playlist_detail(id as i64).await {
                        Ok(detail) => {
                            sender.input(PlaylistDetailMsg::PlaylistLoaded(detail));
                        }
                        Err(e) => {
                            eprintln!("加载歌单失败: {}", e);
                            // 实际项目中可以在这里切到一个 "error" 的 Stack 页面，显示重试按钮
                        }
                    }
                });
            }
            PlaylistDetailMsg::PlaylistLoaded(detail) => {
                info!("歌单加载完成: {}", detail.name);
                self.track_ids = detail.track_ids;
                self.tracks = detail.tracks.clone();
                self.title = detail.name;
                self.creator = format!("创建者：{}", detail.creator_name);
                self.description = detail.description;
                self.cover_url = format!("{}?param=600y600", detail.cover_url);
                while let Some(child) = self.header_cover_container.first_child() {
                    self.header_cover_container.remove(&child);
                }
                let cover_img = async_image!(
                    &self.cover_url,
                    size: (150, 150),
                    radius: Lg,
                    // placeholder: icon("missing-album", 150)
                );
                cover_img.set_hexpand(false);
                
                cover_img.set_vexpand(false);
                cover_img.set_halign(gtk::Align::Start);
                cover_img.set_valign(gtk::Align::Start);
                self.header_cover_container.append(cover_img.widget());

                self.list_store.remove_all();
                for track in detail.tracks {
                    let obj = BoxedAnyObject::new(track);
                    self.list_store.append(&obj);
                }

                // ===================================================
                // 数据和 UI 都准备完毕，丝滑切换到 content 视图！
                // ===================================================
                self.main_stack.set_visible_child_name("content");
            }
            PlaylistDetailMsg::PlayAllClicked => {
                println!("点击了播放全部，track_ids: {:?}", self.track_ids);
                sender
                    .output(PlaylistDetailOutput::PlayQueue(
                        self.tracks.clone(),
                        self.track_ids.clone(),
                        0,
                    ))
                    .unwrap();
            }
            PlaylistDetailMsg::LikeClicked => {
                info!("点击了收藏");
            }
            PlaylistDetailMsg::TrackPlayClicked(track_id) => {
                let mut index = 0;
                for (i, id) in self.track_ids.iter().enumerate() {
                    if *id == track_id {
                        index = i;
                        break;
                    }
                }
                let song_ids = self
                    .tracks
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<Vec<i64>>();
                sender
                    .output(PlaylistDetailOutput::PlayQueue(
                        self.tracks.clone(),
                        self.track_ids.clone(),
                        index
                    ))
                    .unwrap();
            }
            PlaylistDetailMsg::TrackMoreClicked(track_id) => {
                info!("点击了列表更多选项，音轨 ID: {}", track_id);
            }
        }
    }
}

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
            if let Ok(id) = btn.widget_name().parse::<i64>() {
                s1.input(PlaylistDetailMsg::TrackPlayClicked(id));
            }
        });

        let s2 = sender_for_setup.clone();
        more_btn.connect_clicked(move |btn| {
            if let Ok(id) = btn.widget_name().parse::<i64>() {
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
        let artist_names = track
            .artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        artist_label.set_label(&artist_names);
        album_label.set_label(&track.album.name);

        // 利用 widget_name 巧妙保存 track_id，供 Setup 阶段绑定的闭包读取
        play_btn.set_widget_name(&track.id.to_string());
        more_btn.set_widget_name(&track.id.to_string());

        // 动态加载封面图片
        while let Some(child) = cover_container.first_child() {
            cover_container.remove(&child);
        }
        let cover_url = format!("{}?param=80y80", track.cover_url);
        let track_cover = async_image!(
            &cover_url,
            size: (40, 40),
            radius: Md, // 列表中等圆角
            placeholder: icon("missing-album", 40)
        );
        cover_container.append(track_cover.widget());
    });

    factory
}
