use std::sync::Arc;

use relm4::gtk::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent, gtk, view,
};

use crate::api::{
    Album, ArtistDetail, Mv, Song, get_artist_album, get_artist_detail, get_artist_mv,
    get_artist_song,
};
use crate::ui::components::artist::album_grid::AlbumGridInput;
use crate::ui::components::artist::mv_grid::{MvCardOutput, MvGridInput};
use crate::ui::components::artist::song_list::SongListInput;
use crate::ui::components::artist::{AlbumGrid, MvGrid, SongList};
use crate::ui::components::image::AsyncImage;
use crate::ui::components::playlist_card::PlaylistCardOutput;
use crate::ui::components::track_row::TrackRowOutput;
use crate::ui::model::PlaylistType;
use crate::ui::route::AppRoute;

pub struct ArtistPage {
    // 业务状态
    artist_id: u64,
    artist: ArtistDetail,
    songs_arc: Arc<Vec<Song>>,

    // 子组件控制器
    songs: Controller<SongList>,
    albums: Controller<AlbumGrid>,
    mvs: Controller<MvGrid>,

    bio_label: gtk::Label,
}

#[derive(Debug)]
pub enum ArtistMsg {
    LoadArtistDetail(u64),
    LoadArtistAlbums(u64),
    LoadArtistMvs(u64),
    LoadArtistSongs(u64),
    PlayAll,
    Follow,

    TrackRowPlayClicked(u64),
    TrackRowMoreClicked(u64),
    AlbumGridClicked(u64),
    AlbumGridPlayClicked(u64),
    MvGridClicked(u64),
}

#[derive(Debug)]
pub enum ArtistCmdMsg {
    LoadArtistDetailed(ArtistDetail),
    LoadArtistAlbumsed(Vec<Album>),
    LoadArtistMvsed(Vec<Mv>),
    LoadArtistSongsed(Vec<Song>),
}
#[derive(Debug)]
pub enum ArtistPageOutput {
    PlayQueue {
        artist_id: u64,
        artist_name: String,
        songs: Arc<Vec<Song>>,
        start_index: usize,
    },
    Navigate(AppRoute),
}

#[relm4::component(pub)]
impl Component for ArtistPage {
    type Init = u64;
    type Input = ArtistMsg;
    type Output = ArtistPageOutput;
    type CommandOutput = ArtistCmdMsg;

    // 纯正的 view! 声明式写法
    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_expand: true,

            // ================= 顶部 Header =================
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 24,
                set_margin_all: 24,

                AsyncImage {
                    set_width_request: 260,
                    set_height_request: 260,
                    set_corner_radius: 16.0,
                    // #[track = "model.changed(ArtistPage::avatar_url())"]
                    #[watch]
                    set_url: format!("{}?param=300y300", model.artist.avatar.clone()),
                    set_placeholder_icon: "folder-music-symbolic",
                    set_fallback_icon: "missing-album",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_halign: gtk::Align::Start,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Start,
                        set_spacing: 8,
                        gtk::Label {
                            #[watch]
                            set_label: &model.artist.name,
                            add_css_class: "title-1",
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &model.artist.trans_name,
                            add_css_class: "title-2",
                            add_css_class: "dim-label",
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &model.artist.identify_desc,
                            add_css_class: "dim-label",
                        }
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.artist.alias_text,
                        add_css_class: "caption",
                        set_valign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.artist.signature,
                        add_css_class: "caption",
                        add_css_class: "dim-label",
                        set_valign: gtk::Align::Start,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,
                        set_margin_top: 8,
                        gtk::Label {
                            #[watch]
                            set_label: &format!("单曲 {}", model.artist.music_size),
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &format!("专辑 {}", model.artist.album_size),
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &format!("MV {}", model.artist.mv_size),
                        },
                    },

                    gtk::Box {
                        set_spacing: 12,
                        set_margin_top: 16,
                        gtk::Button {
                            set_label: "播放全部",
                            add_css_class: "suggested-action",
                            connect_clicked[sender] => move |_| {
                                sender.input(ArtistMsg::PlayAll);
                            }
                        },
                        gtk::Button {
                            set_label: "收藏/关注",
                            connect_clicked[sender] => move |_| {
                                sender.input(ArtistMsg::Follow);
                            }
                        }
                    }
                }
            },

            gtk::Separator {},

            // ================= 中部导航条 =================
            // 必须用 #[name] 预留，因为要在 init 中通过 widgets.xxx.set_stack() 绑定
            #[name = "tab_switcher"]
            gtk::StackSwitcher {
                set_halign: gtk::Align::Center,
                set_margin_top: 8,
                set_margin_bottom: 8,
            },

            // ================= 底部内容区 =================
            // 必须用 #[name] 预留，因为要在 init 中动态调用 add_titled 添加子组件
            #[name = "content_stack"]
            gtk::Stack {
                set_vexpand: true,
                set_transition_duration: 200,
            },

            // #[name = "bio_page"]
            // gtk::ScrolledWindow {
            //     set_vexpand: true,
            //     gtk::Label {
            //         #[watch]
            //         set_label: &model.artist.brief_desc,
            //         set_wrap: true,
            //         set_visible: false,
            //         set_margin_all: 24,
            //         set_xalign: 0.0,
            //     }
            // }
        },
        #[name = "bio_label"]
        gtk::Label {}
    }

    fn init(
        artist_id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 1. 提前声明 model (必须包含子组件的 Controller)
        let songs =
            SongList::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    TrackRowOutput::PlayClicked(id) => ArtistMsg::TrackRowPlayClicked(id),
                    TrackRowOutput::MoreClicked(id) => ArtistMsg::TrackRowMoreClicked(id),
                });
        let albums =
            AlbumGrid::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    PlaylistCardOutput::Clicked(id) => ArtistMsg::AlbumGridClicked(id),
                    PlaylistCardOutput::ClickedPlaylist(id) => ArtistMsg::AlbumGridPlayClicked(id),
                });
        let mvs = MvGrid::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                MvCardOutput::Clicked(id) => ArtistMsg::MvGridClicked(id),
            });

        let model = ArtistPage {
            artist_id,
            artist: ArtistDetail::default(),
            songs,
            albums,
            mvs,
            songs_arc: Arc::new(Vec::new()),
            bio_label: gtk::Label::default(),
        };

        // 2. 执行 view! 宏生成 widgets 结构体
        let mut widgets = view_output!();

        widgets.tab_switcher.set_stack(Some(&widgets.content_stack));
        widgets
            .content_stack
            .add_titled(model.songs.widget(), Some("songs"), "热门单曲");
        widgets
            .content_stack
            .add_titled(model.albums.widget(), Some("albums"), "所有专辑");
        widgets
            .content_stack
            .add_titled(model.mvs.widget(), Some("mvs"), "相关MV");

        // 手动构建 bio_page
        let bio_label = gtk::Label::builder()
            .wrap(true)
            .margin_start(24)
            .margin_end(24)
            .margin_top(24)
            .margin_bottom(24)
            .xalign(0.0)
            .build();
        let bio_page = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&bio_label)
            .build();
        widgets
            .content_stack
            .add_titled(&bio_page, Some("bio"), "艺人简介");

        // 把 label 存起来供后续更新
        widgets.bio_label = bio_label;

        sender.input(ArtistMsg::LoadArtistDetail(artist_id));
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ArtistMsg::PlayAll => {}
            ArtistMsg::Follow => sender
                .output(ArtistPageOutput::PlayQueue {
                    artist_id: self.artist_id,
                    artist_name: self.artist.name.clone(),
                    songs: self
                        .songs_arc
                        .clone(),
                    start_index: 0,
                })
                .unwrap(),
            ArtistMsg::LoadArtistDetail(id) => sender.command(move |out, _shutdown| async move {
                match get_artist_detail(id).await {
                    Ok(detail) => {
                        let _ = out.send(ArtistCmdMsg::LoadArtistDetailed(detail));
                    }
                    Err(err) => {
                        log::error!("Failed to load artist detail: {}", err);
                    }
                }
            }),
            ArtistMsg::LoadArtistSongs(id) => sender.command(move |out, _shutdown| async move {
                match get_artist_song(id).await {
                    Ok(items) => {
                        let _ = out.send(ArtistCmdMsg::LoadArtistSongsed(items));
                    }
                    Err(err) => {
                        log::error!("Failed to load artist songs: {}", err);
                    }
                }
            }),
            ArtistMsg::LoadArtistAlbums(id) => sender.command(move |out, _shutdown| async move {
                match get_artist_album(id).await {
                    Ok(items) => {
                        let _ = out.send(ArtistCmdMsg::LoadArtistAlbumsed(items));
                    }
                    Err(err) => {
                        log::error!("Failed to load artist albums: {}", err);
                    }
                }
            }),
            ArtistMsg::LoadArtistMvs(id) => sender.command(move |out, _shutdown| async move {
                match get_artist_mv(id).await {
                    Ok(items) => {
                        let _ = out.send(ArtistCmdMsg::LoadArtistMvsed(items));
                    }
                    Err(err) => {
                        log::error!("Failed to load artist mvs: {}", err);
                    }
                }
            }),
            ArtistMsg::TrackRowPlayClicked(id) => {
                let index = self
                    .songs_arc
                    .iter()
                    .position(|song| song.id == id)
                    .unwrap();
                sender
                    .output(ArtistPageOutput::PlayQueue {
                        artist_id: self.artist_id,
                        artist_name: self.artist.name.clone(),
                        songs: self
                            .songs_arc
                            .clone(),
                        start_index: index,
                    })
                    .unwrap();
            }
            ArtistMsg::TrackRowMoreClicked(_) => {}
            ArtistMsg::AlbumGridClicked(id) => sender
                .output(ArtistPageOutput::Navigate(AppRoute::PlaylistDetail(
                    PlaylistType::Album(id),
                )))
                .unwrap(),
            ArtistMsg::AlbumGridPlayClicked(_) => {
                sender
                    .output(ArtistPageOutput::PlayQueue {
                        artist_id: self.artist_id,
                        artist_name: self.artist.name.clone(),
                        songs: self
                            .songs_arc
                            .clone(),
                        start_index: 0,
                    })
                    .unwrap();
            },
            ArtistMsg::MvGridClicked(id) => {

            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ArtistCmdMsg::LoadArtistDetailed(artist_detail) => {
                self.artist = artist_detail;
                self.bio_label.set_label(&self.artist.brief_desc);
                // 触发加载其余数据
                sender.input(ArtistMsg::LoadArtistSongs(self.artist_id));
                sender.input(ArtistMsg::LoadArtistAlbums(self.artist_id));
                sender.input(ArtistMsg::LoadArtistMvs(self.artist_id));
            }
            ArtistCmdMsg::LoadArtistSongsed(songs) => {
                self.songs_arc = Arc::new(songs.clone());
                self.songs.emit(SongListInput::SetSongs(songs));
            }
            ArtistCmdMsg::LoadArtistAlbumsed(albums) => {
                self.albums.emit(AlbumGridInput::SetAlbums(albums));
            }
            ArtistCmdMsg::LoadArtistMvsed(mvs) => {
                self.mvs.emit(MvGridInput::SetMvs(mvs));
            }
        }
    }
}
