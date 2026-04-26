use std::sync::Arc;
use std::sync::Mutex;

use log::trace;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, factory::FactoryVecDeque, gtk, prelude::*};

use crate::api::{
    PlaylistDetail as PlaylistDetailModel, Song, album_subscribe, get_album_detail,
    get_playlist_detail, get_recommend_song, playlist_subscribe,
};
use crate::db::{CollectType, Db};
use crate::ui::components::image::AsyncImage;
use crate::ui::components::track_row::{TrackRow, TrackRowInit, TrackRowOutput};
use crate::ui::model::{DetailView, PlaylistType};

#[derive(Debug)]
pub enum PlaylistDetailMsg {
    LoadPlaylist(u64),
    LoadAlbum(u64),
    LoadDailyRecommend,
    PlayAllClicked,
    LikeClicked,
    TrackPlayClicked(u64),
    TrackMoreClicked(u64),
}

#[derive(Debug)]
pub enum PlaylistDetailOutput {
    PlayQueue {
        tracks: Arc<Vec<Song>>,
        track_ids: Arc<Vec<u64>>,
        start_index: usize,
        playlist: crate::api::Playlist,
    },
    ShowToast(String),
}

#[derive(Debug)]
pub enum PlaylistDetailCmdMsg {
    PlaylistLoaded(PlaylistDetailModel),
    AlbumLoaded(crate::api::AlbumDetail),
    DailyRecommendLoaded(Vec<Song>),
    SubscribeResult { success: bool, collected: bool, name: String },
}

#[tracker::track]
pub struct PlaylistDetail {
    #[do_not_track]
    playlist_type: PlaylistType,
    #[do_not_track]
    detail: Option<DetailView>,
    #[do_not_track]
    tracks_arc: Option<Arc<Vec<Song>>>,
    #[do_not_track]
    ids_arc: Option<Arc<Vec<u64>>>,
    is_loading: bool,
    is_collected: bool,
    is_own: bool,
    #[do_not_track]
    user_id: u64,
    #[do_not_track]
    db: Arc<Mutex<Db>>,
    #[do_not_track]
    tracks_list: FactoryVecDeque<TrackRow>,
}

#[relm4::component(pub)]
impl Component for PlaylistDetail {
    type Init = (PlaylistType, Arc<Mutex<Db>>, u64);
    type Input = PlaylistDetailMsg;
    type Output = PlaylistDetailOutput;
    type CommandOutput = PlaylistDetailCmdMsg;

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
                            set_url: model.detail.as_ref()
                                .map(|d| format!("{}?param=300y300", d.cover_url))
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
                            #[watch]
                            set_label: model.detail.as_ref().map(|d| d.name.as_str()).unwrap_or_default(),
                            add_css_class: "title-1", // 超大主标题
                            set_halign: gtk::Align::Start
                        },
                        gtk::Label {
                            #[watch]
                            set_label: model.detail.as_ref()
                                .and_then(|d| d.creator.as_deref())
                                .unwrap_or(""),
                            add_css_class: "dim-label",
                            set_halign: gtk::Align::Start
                        },
                        gtk::Label {
                            #[watch]
                            set_label: model.detail.as_ref()
                                .and_then(|d| d.description.as_deref())
                                .unwrap_or("")
                                .replace("\n", "").as_str(),

                            set_wrap: true,
                            set_wrap_mode: gtk::pango::WrapMode::WordChar,

                            set_max_width_chars: 40,
                            set_lines: 3,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
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
                                #[track = "model.changed(PlaylistDetail::is_collected())"]
                                set_icon_name: if model.is_collected { "heart-filled" } else { "plus-large-symbolic" },
                                #[track = "model.changed(PlaylistDetail::is_collected())"]
                                set_tooltip_text: Some(if model.is_collected { "取消收藏" } else { "收藏" }),
                                set_size_request: (46, 46),
                                add_css_class: "circular",
                                #[watch]
                                set_sensitive: !model.is_own && !matches!(model.playlist_type, PlaylistType::DailyRecommend),
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
        (playlist_type, db, user_id): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let is_collected = match &playlist_type {
            PlaylistType::Playlist(id) => {
                db.lock().unwrap().is_collected(*id, CollectType::Playlist)
            }
            PlaylistType::Album(id) => {
                db.lock().unwrap().is_collected(*id, CollectType::Album)
            }
            PlaylistType::DailyRecommend => false,
        };

        let mut model = Self {
            playlist_type: playlist_type.clone(),
            detail: None,
            tracks_arc: None,
            ids_arc: None,
            is_loading: true,
            is_collected,
            is_own: false,
            user_id,
            db,
            tracks_list: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |msg| match msg {
                    TrackRowOutput::PlayClicked(id) => PlaylistDetailMsg::TrackPlayClicked(id),
                    TrackRowOutput::MoreClicked(id) => PlaylistDetailMsg::TrackMoreClicked(id),
                }),
            tracker: 0,
        };

        let track_list_box = model.tracks_list.widget();
        let widgets = view_output!();

        // 触发加载
        sender.input(match playlist_type {
            PlaylistType::Playlist(id) => PlaylistDetailMsg::LoadPlaylist(id),
            PlaylistType::Album(id) => PlaylistDetailMsg::LoadAlbum(id),
            PlaylistType::DailyRecommend => PlaylistDetailMsg::LoadDailyRecommend,
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.reset();
        trace!("PlaylistDetail Msg: {:?}", message);
        match message {
            PlaylistDetailMsg::LoadPlaylist(id) => {
                self.set_is_loading(true);
                sender.command(move |out, _shutdown| async move {
                    if let Ok(detail) = get_playlist_detail(id).await {
                        let _ = out.send(PlaylistDetailCmdMsg::PlaylistLoaded(detail));
                    }
                });
            }
            PlaylistDetailMsg::PlayAllClicked => {
                // 同时取用两个 Arc
                if let (Some(_detail), Some(tracks_arc), Some(ids_arc)) =
                    (&self.detail, &self.tracks_arc, &self.ids_arc)
                {
                    sender
                        .output(PlaylistDetailOutput::PlayQueue {
                            tracks: tracks_arc.clone(),
                            track_ids: ids_arc.clone(),
                            start_index: 0,
                            playlist: self.detail.clone().unwrap().into(),
                        })
                        .unwrap();
                }
            }
PlaylistDetailMsg::LikeClicked => {
                if self.is_own || matches!(self.playlist_type, PlaylistType::DailyRecommend) {
                    return;
                }
                let new_collected = !self.is_collected;
                let id = match &self.playlist_type {
                    PlaylistType::Playlist(id) => *id,
                    PlaylistType::Album(id) => *id,
                    PlaylistType::DailyRecommend => return,
                };
                let collect_type = match &self.playlist_type {
                    PlaylistType::Playlist(_) => CollectType::Playlist,
                    PlaylistType::Album(_) => CollectType::Album,
                    PlaylistType::DailyRecommend => return,
                };
                let name = self.detail.as_ref().map(|d| d.name.clone()).unwrap_or_default();
                sender.command(move |out, _shutdown| async move {
                    let result = match collect_type {
                        CollectType::Playlist => playlist_subscribe(id, new_collected).await,
                        CollectType::Album => album_subscribe(id, new_collected).await,
                    };
                    let _ = out.send(PlaylistDetailCmdMsg::SubscribeResult {
                        success: result.is_ok(),
                        collected: new_collected,
                        name,
                    });
                });
            }
            PlaylistDetailMsg::TrackPlayClicked(track_id) => {
                if let (Some(_detail), Some(tracks_arc), Some(ids_arc)) =
                    (&self.detail, &self.tracks_arc, &self.ids_arc)
                {
                    // 注意：这里在 ids_arc 上查找位置
                    let index = ids_arc.iter().position(|id| *id == track_id).unwrap_or(0);

                    sender
                        .output(PlaylistDetailOutput::PlayQueue {
                            tracks: tracks_arc.clone(),
                            track_ids: ids_arc.clone(),
                            start_index: index,
                            playlist: self.detail.clone().unwrap().into(),
                        })
                        .unwrap();
                }
            }
            PlaylistDetailMsg::TrackMoreClicked(track_id) => {
                eprintln!("点击了列表更多选项，音轨 ID: {}", track_id);
            }
            PlaylistDetailMsg::LoadAlbum(id) => {
                self.set_is_loading(true);
                sender.command(
                    move |out: relm4::Sender<PlaylistDetailCmdMsg>, _shutdown| async move {
                        if let Ok(detail) = get_album_detail(id).await {
                            let _ = out.send(PlaylistDetailCmdMsg::AlbumLoaded(detail));
                        }
                    },
                );
            }
            PlaylistDetailMsg::LoadDailyRecommend => {
                self.set_is_loading(true);
                sender.command(
                    move |out: relm4::Sender<PlaylistDetailCmdMsg>, _shutdown| async move {
                        if let Ok(songs) = get_recommend_song().await {
                            let _ = out.send(PlaylistDetailCmdMsg::DailyRecommendLoaded(songs));
                        }
                    },
                );
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            PlaylistDetailCmdMsg::PlaylistLoaded(detail) => {
                let dv: DetailView = detail.into();
                if matches!(self.playlist_type, PlaylistType::Playlist(_)) && dv.creator_id == self.user_id {
                    self.set_is_own(true);
                }
                self.apply_detail(dv);
            }
            PlaylistDetailCmdMsg::AlbumLoaded(detail) => {
                self.apply_detail(detail.into());
            }
            PlaylistDetailCmdMsg::DailyRecommendLoaded(songs) => {
                self.apply_detail(songs.into());
            }
            PlaylistDetailCmdMsg::SubscribeResult { success, collected, name } => {
                if success {
                    let id = match &self.playlist_type {
                        PlaylistType::Playlist(id) => *id,
                        PlaylistType::Album(id) => *id,
                        PlaylistType::DailyRecommend => return,
                    };
                    let collect_type = match &self.playlist_type {
                        PlaylistType::Playlist(_) => CollectType::Playlist,
                        PlaylistType::Album(_) => CollectType::Album,
                        PlaylistType::DailyRecommend => return,
                    };
                    self.db.lock().unwrap().set_collected(id, collect_type, collected);
                    self.set_is_collected(collected);
                    let toast = if collected {
                        format!("已收藏「{}」", name)
                    } else {
                        format!("已取消收藏「{}」", name)
                    };
                    sender.output(PlaylistDetailOutput::ShowToast(toast)).ok();
                } else {
                    sender.output(PlaylistDetailOutput::ShowToast("操作失败".to_string())).ok();
                }
            }
        }
    }
}

impl PlaylistDetail {
    fn apply_detail(&mut self, detail: DetailView) {
        self.tracks_list.guard().clear();

        let tracks_arc = Arc::new(detail.tracks.clone());
        let ids_arc = Arc::new(detail.track_ids.clone());

        let mut guard = self.tracks_list.guard();
        for (index, track) in tracks_arc.iter().enumerate() {
            guard.push_back(TrackRowInit {
                track: track.clone(),
                index,
            });
        }
        drop(guard);

        self.tracks_arc = Some(tracks_arc);
        self.ids_arc = Some(ids_arc);
        self.detail = Some(detail);
        self.set_is_loading(false);
    }
}
