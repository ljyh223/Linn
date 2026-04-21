use std::mem;
use std::sync::Arc;

use log::{info, trace};
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, factory::FactoryVecDeque, gtk, prelude::*};

use crate::api::{Playlist, PlaylistDetail as PlaylistDetailModel, Song, get_playlist_detail};
use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub enum PlaylistDetailMsg {
    LoadPlaylist(u64),
    PlaylistLoaded(PlaylistDetailModel),
    PlayAllClicked,
    LikeClicked,
    TrackPlayClicked(u64),
    TrackMoreClicked(u64),
}

#[derive(Debug)]
pub struct TrackRowInit {
    pub track: Song,
    pub index: usize,
}

#[derive(Debug)]
pub struct TrackRow {
    track: Song,
    index: usize,
}

#[relm4::factory(pub)]
impl FactoryComponent for TrackRow {
    type Init = TrackRowInit;
    type Input = ();
    type Output = PlaylistDetailMsg; // 修复1：明确声明 Output 为父组件的 Msg
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        // 使用水平 Box 布局
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 16,
            set_margin_all: 8,
            set_valign: gtk::Align::Center,

            // --- 1. 左侧：封面 ---
            // 假设你在上一轮问题中提到 AsyncImage 是可以直接在宏里调用的 Widget
            AsyncImage {
                set_width_request: 48,
                set_height_request: 48,
                set_corner_radius: 4.0,
                set_url: format!("{}?param=100y100", self.track.cover_url),
                set_placeholder_icon: "missing-album-symbolic",
            },

            // --- 2. 左中侧：歌名与歌手 ---
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_valign: gtk::Align::Center,
                set_spacing: 4,
                set_width_request: 200,

                gtk::Label {
                    set_label: &self.track.name,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "heading", // GTK 自带样式：加粗标题
                },
                gtk::Label {
                    set_label: &self.track.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "dim-label", // GTK 自带样式：灰色次要文本
                    add_css_class: "caption",
                }
            },

            // --- 3. 中间：专辑名 (占据剩余空间) ---
            gtk::Label {
                set_label: &self.track.album.name,
                set_halign: gtk::Align::Start,
                set_hexpand: true, // 撑开中间，把右侧按钮挤到最右边
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "dim-label",
            },

            // --- 4. 右侧：功能按钮 ---
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_valign: gtk::Align::Center,

                gtk::Button {
                    set_icon_name: "media-playback-start-symbolic",
                    add_css_class: "circular", // GTK 自带：正圆形按钮
                    add_css_class: "flat",     // GTK 自带：扁平无边框，悬浮变色
                    set_tooltip_text: Some("播放"),
                    // 完美绑定：直接将当前音轨 ID 发给父组件
                    connect_clicked[sender, track_id = self.track.id] => move |_| {
                        trace!("点击了播放按钮，播放 ID: {}", track_id);
                        sender.output(PlaylistDetailMsg::TrackPlayClicked(track_id)).unwrap();
                    }
                },
                gtk::Button {
                    set_icon_name: "view-more-symbolic",
                    add_css_class: "circular",
                    add_css_class: "flat",
                    set_tooltip_text: Some("更多选项"),
                    connect_clicked[sender, track_id = self.track.id] => move |_| {
                        trace!("点击了更多按钮，ID: {}", track_id);
                        sender.output(PlaylistDetailMsg::TrackMoreClicked(track_id)).unwrap();
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            track: init.track,
            index: init.index,
        }
    }
}

#[derive(Debug)]
pub enum PlaylistDetailOutput {
    PlayQueue(Arc<Vec<Song>>, Arc<Vec<u64>>, usize, Playlist),
}

pub struct PlaylistDetail {
    id: u64,
    detail_detail: Option<PlaylistDetailModel>,
    tracks_arc: Option<Arc<Vec<Song>>>,
    ids_arc: Option<Arc<Vec<u64>>>,
    is_loading: bool,

    tracks_list: FactoryVecDeque<TrackRow>,
}

#[relm4::component(pub)]
impl Component for PlaylistDetail {
    type Init = u64;
    type Input = PlaylistDetailMsg;
    type Output = PlaylistDetailOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Stack {
            set_transition_type: gtk::StackTransitionType::Crossfade,
            // 响应式切换页面：根本不需要在 update 里手动操作可见性
            #[watch]
            set_visible_child_name: if model.is_loading { "loading" } else { "content" },

            // ====== 页面 1：加载中 ======
            add_named[Some("loading")] = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_spacing: 16,

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

            // ====== 页面 2：真实内容 ======
            add_named[Some("content")] = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                // --- Header 区域 ---
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Start,

                    set_spacing: 32,
                    // 利用 margin 留白，显得更有呼吸感
                    set_margin_top: 48,
                    set_margin_bottom: 32,
                    set_margin_start: 48,
                    set_margin_end: 48,

                        AsyncImage {
                            set_width_request: 200,
                            set_height_request: 200,
                            set_corner_radius: 16.0,

                            #[watch]
                            set_url: model.detail_detail.as_ref()
                                .map(|d| format!("{}?param=600y600", d.cover_url)) // 现在你可以放心用 600 高清图了！
                                .unwrap_or_default(),
                            set_placeholder_icon: "folder-music-symbolic",
                            add_css_class: "card",
                        },
                    // 2. 右侧信息区
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_valign: gtk::Align::Center,

                        gtk::Label {
                            #[watch] set_label: model.detail_detail.as_ref().map(|d| d.name.as_str()).unwrap_or_default(),
                            add_css_class: "title-1", // 超大主标题
                            set_halign: gtk::Align::Start
                        },
                        gtk::Label {
                            #[watch] set_label: &model.detail_detail.as_ref().map(|d| format!("创建者：{}", d.creator_name)).unwrap_or_default(),
                            add_css_class: "dim-label",
                            set_halign: gtk::Align::Start
                        },
                        gtk::Label {
                            #[watch] set_label: model.detail_detail.as_ref().map(|d| d.description.as_str()).unwrap_or_default(),
                            set_wrap: true,
                            set_max_width_chars: 80,
                            set_halign: gtk::Align::Start
                        },

                        // 3. 按钮 Row
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 16,
                            set_margin_top: 16,

                            gtk::Button {
                                set_label: "播放全部",
                                set_icon_name: "media-playback-start-symbolic",
                                // GTK 样式：suggested-action (主题色背景), pill (药丸形大按钮)
                                add_css_class: "suggested-action",
                                add_css_class: "pill",
                                connect_clicked => PlaylistDetailMsg::PlayAllClicked
                            },
                            gtk::Button {
                                set_icon_name: "plus-large",
                                set_size_request: (46, 46),
                                add_css_class: "circular", // 圆形
                                set_tooltip_text: Some("收藏"),
                                connect_clicked => PlaylistDetailMsg::LikeClicked
                            }
                        }
                    }
                },

                // --- 列表区域 ---
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_margin_start: 24,
                    set_margin_end: 24,

                    // 使用 ListBox 配合 FactoryVecDeque
                    #[local_ref]
                    track_list_box -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        add_css_class: "rich-list",
                        set_selection_mode: gtk::SelectionMode::None,
                    }
                }
            }
        }
    }

    fn init(
        id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            id,
            detail_detail: None,
            tracks_arc: None,
            ids_arc: None,
            is_loading: true, // 初始为加载状态
            // 初始化工厂构建器，绑定到父组件 Sender
            tracks_list: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |msg| msg),
        };

        let track_list_box = model.tracks_list.widget();
        let widgets = view_output!();

        // 触发加载
        sender.input(PlaylistDetailMsg::LoadPlaylist(id));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        trace!("PlaylistDetail Msg: {:?}", message);
        match message {
            PlaylistDetailMsg::LoadPlaylist(id) => {
                eprintln!("开始加载歌单 ID: {}", id);
                self.is_loading = true; // 触发 UI 回到加载中

                let sender_clone = sender.clone();
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(detail) = get_playlist_detail(id).await {
                        sender_clone.input(PlaylistDetailMsg::PlaylistLoaded(detail));
                    }
                });
            }
            PlaylistDetailMsg::PlaylistLoaded(mut detail) => {
                self.tracks_list.guard().clear();

                // 1. 零成本转移 tracks
                let tracks = mem::take(&mut detail.tracks);
                let tracks_arc = Arc::new(tracks);
                let ids = mem::take(&mut detail.track_ids);
                let ids_arc = Arc::new(ids);

                let mut guard = self.tracks_list.guard();
                for (index, track) in tracks_arc.iter().enumerate() {
                    guard.push_back(TrackRowInit {
                        track: track.clone(),
                        index,
                    });
                }
                drop(guard);

                // 4. 保存状态
                self.tracks_arc = Some(tracks_arc);
                self.ids_arc = Some(ids_arc);
                self.detail_detail = Some(detail); // 存入被掏空 tracks 和 ids 的 detail
                self.is_loading = false;
            }
            PlaylistDetailMsg::PlayAllClicked => {
                // 同时取用两个 Arc
                if let (Some(_detail), Some(tracks_arc), Some(ids_arc)) =
                    (&self.detail_detail, &self.tracks_arc, &self.ids_arc)
                {
                    sender
                        .output(PlaylistDetailOutput::PlayQueue(
                            tracks_arc.clone(),
                            ids_arc.clone(),
                            0,
                            Playlist::from(self.detail_detail.as_ref().unwrap()),
                        ))
                        .unwrap();
                }
            }
            PlaylistDetailMsg::LikeClicked => {
                eprintln!("点击了收藏");
            }
            PlaylistDetailMsg::TrackPlayClicked(track_id) => {
                if let (Some(_detail), Some(tracks_arc), Some(ids_arc)) =
                    (&self.detail_detail, &self.tracks_arc, &self.ids_arc)
                {
                    // 注意：这里在 ids_arc 上查找位置
                    let index = ids_arc.iter().position(|id| *id == track_id).unwrap_or(0);

                    sender
                        .output(PlaylistDetailOutput::PlayQueue(
                            tracks_arc.clone(),
                            ids_arc.clone(),
                            index,
                            Playlist::from(self.detail_detail.as_ref().unwrap()),
                        ))
                        .unwrap();
                }
            }
            PlaylistDetailMsg::TrackMoreClicked(track_id) => {
                eprintln!("点击了列表更多选项，音轨 ID: {}", track_id);
            }
        }
    }
}
