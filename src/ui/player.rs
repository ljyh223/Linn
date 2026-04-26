use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use relm4::gtk::Orientation;
use relm4::gtk::{self, glib, prelude::*};
use relm4::prelude::*;

use crate::api::{Artist, Playlist, Song};
use crate::player::PlayMode;
use crate::ui::components::image::AsyncImage;
use crate::ui::model::PlaylistType;
use crate::ui::route::AppRoute;

#[tracker::track]
pub struct PlayerPage {
    song: Song,
    is_playing: bool,
    is_liked: bool,
    playlist: Arc<Playlist>,
    position: u64,
    volume: f64,
    play_mode: PlayMode,
    loop_enabled: bool,
    progress_scale: gtk::Scale,
    is_seeking: Rc<Cell<bool>>,
}

#[derive(Debug)]
pub enum PlayerPageOutput {
    TogglePlay,
    PrevTrack,
    NextTrack,
    Seek(u64),
    Remove(usize),
    PlayAt(usize),
    SetMode(PlayMode),
    SetLoop(bool),
    Navigate(AppRoute),
    OpenArtistDialog(Vec<Artist>),
    ToggleLike(u64, bool),
    CollectSong(u64),
}

#[derive(Debug)]
pub enum PlayerPageMsg {
    UpdateTrack(Song),
    UpdatePlayback(bool),
    SetQueue {
        tracks: Arc<Vec<Song>>,
        playlist: Arc<Playlist>,
        start_index: usize,
    },
    UpdateProgress {
        position: u64,
        duration: u64,
    },
    TogglePlay,
    PrevTrack,
    NextTrack,
    Seek(u64),
    ToggleLike,
    VolumeChanged(f64),
    ToggleMode,
    ToggleLoop(bool),
    ArtistClicked,
    AlbumClicked,
    PlaylistClicked,
    CollectClicked,
    SetLiked(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for PlayerPage {
    type Init = ();
    type Input = PlayerPageMsg;
    type Output = PlayerPageOutput;

    view! {
            #[root]
            gtk::Box {
                set_orientation: Orientation::Vertical,
                set_valign: gtk::Align::Center,
                set_halign: gtk::Align::Center,
                set_spacing: 20,
                set_margin_all: 24,
                set_vexpand: true,

                gtk::CenterBox {
                    set_orientation: Orientation::Horizontal,
                    set_width_request: 280,
                    #[wrap(Some)]
                    set_start_widget = &gtk::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 2,
                        set_halign: gtk::Align::Start,

                        gtk::Label {
                            set_label: "Playing From",
                            set_halign: gtk::Align::Start,
                            // add_css_class: "dim-label",
                        },

                        gtk::Label {
                            #[track = "model.changed(PlayerPage::playlist())"]
                            set_label: &model.playlist.name,
                            set_halign: gtk::Align::Start,
                            add_css_class: "caption-heading",
                        }
                    },
                    #[wrap(Some)]
                    set_end_widget = &AsyncImage {
                        #[track = "model.changed(PlayerPage::playlist())"]
                        set_url: format!("{}?param=100y100", model.playlist.cover_url.clone()),
                        set_width_request: 36,
                        set_height_request: 36,
                        set_corner_radius: 4.0,
                        set_placeholder_icon: "folder-music-symbolic",
                    },
                    add_controller = gtk::GestureClick::new() {
                        connect_pressed[sender] => move |_, _, _, _| {
                            sender.input(PlayerPageMsg::PlaylistClicked);
                        }
                    }
                },

                gtk::Box{
                    set_height_request: 32,
                },




                // ================= 2. 封面 =================
                AsyncImage {
                    set_width_request: 260,
                    set_height_request: 260,
                    set_corner_radius: 12.0,
                    #[track = "model.changed(PlayerPage::song())"]
                    set_url: format!("{}?param=1000y1000", model.song.cover_url.clone()),
                    set_placeholder_icon: "folder-music-symbolic",
                    set_fallback_icon: "missing-album",
                },

                // ================= 3. 歌手和专辑信息 =================
                gtk::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 4,
                    set_halign: gtk::Align::Center,

                    gtk::Label {
                        #[track = "model.changed(PlayerPage::song())"]
                        set_label: &model.song.name,
                        add_css_class: "title-1",
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_width_chars: 20,
                    },

                    gtk::Box{
                        set_orientation: Orientation::Horizontal,
                        set_align: gtk::Align::Center,
                        gtk::Button {
                            #[track = "model.changed(PlayerPage::song())"]
                            set_label: &model.song.artists.iter().take(2).map(|artist| artist.name.clone()).collect::<Vec<_>>().join(" / "),

                            add_css_class: "flat",
                            add_css_class: "inline",
                            set_halign: gtk::Align::Center,

                            connect_clicked => PlayerPageMsg::ArtistClicked,
                        },

                        gtk::Button {
                            add_css_class: "flat",
                            add_css_class: "inline",
                            set_halign: gtk::Align::Center,
                            connect_clicked => PlayerPageMsg::AlbumClicked,
                            gtk::Label {
                                #[watch]
                                set_label: &model.song.album.name,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_max_width_chars: 15,
                            }
                        }

                    }

                },

                // ================= 4. 相关操作 (音量、喜欢) =================
                gtk::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 12,
                    set_halign: gtk::Align::Center,

                    // 音量图标
                    gtk::Image {
                        #[track = "model.changed(PlayerPage::volume())"]
                        set_icon_name: if model.volume > 0.0 { Some("audio-volume-high-symbolic") } else { Some("audio-volume-muted-symbolic") },
                    },


                    // 喜欢按钮
                    gtk::Button {
                        #[track = "model.changed(PlayerPage::is_liked())"]
                        set_icon_name: if model.is_liked { "heart-filled" } else { "heart-outline-thick" },
                        add_css_class: "flat",
                        set_tooltip_text: Some("Like"),
                        connect_clicked => PlayerPageMsg::ToggleLike,
                    },

                    // 收藏到歌单按钮
                    gtk::Button {
                        set_icon_name: "list-add-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Collect to playlist"),
                        connect_clicked => PlayerPageMsg::CollectClicked,
                    }
                },

                // ================= 5. 进度条 =================
                gtk::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 4,
                    set_width_request: 280,


                    #[name(progress_scale)]
                    gtk::Scale {
                        set_orientation: Orientation::Horizontal,
                        set_range: (0.0, 100.0),
                        set_draw_value: false,
                        set_value: model.position as f64,
                        set_height_request: 20,

                        set_hexpand: true,
                        add_css_class: "player-progress",
                    },

                    // 使用 CenterBox 完美实现两端对齐
                    gtk::CenterBox {
                        set_width_request: 280,
                        #[wrap(Some)]
                        set_start_widget = &gtk::Label {
                            #[track = "model.changed(PlayerPage::position())"]
                            set_label: &format_time(model.position),
                            add_css_class: "caption",
                            add_css_class: "dim-label",
                        },
                        #[wrap(Some)]
                        set_center_widget = &gtk::Label {
                            set_label: "MAX",
                            add_css_class: "quality-badge"
                        },

                        #[wrap(Some)]
                        set_end_widget = &gtk::Label {
                            #[track = "model.changed(PlayerPage::song())"]
                            set_label: &format_time(model.song.duration),
                            add_css_class: "caption",
                            add_css_class: "dim-label",
                        }
                    },
                },
                gtk::Box{
                    set_height_request: 16,
                },

                // ================= 6. 控制器 =================
                gtk::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 16,
                    set_halign: gtk::Align::Center,
                    set_margin_top: 8,

                    // 播放模式（左）：顺序→单曲循环→随机，3态切换
                    gtk::Button {
                        #[track = "model.changed(PlayerPage::play_mode())"]
                        set_icon_name: match model.play_mode {
                            PlayMode::Sequential => "media-playlist-consecutive-symbolic",
                            PlayMode::SingleLoop => "media-playlist-repeat-song-symbolic",
                            PlayMode::Shuffle => "media-playlist-shuffle-symbolic"
                        },
                        add_css_class: "flat",
                        set_tooltip_text: Some(match model.play_mode {
                            PlayMode::Sequential => "顺序播放",
                            PlayMode::SingleLoop => "单曲循环",
                            PlayMode::Shuffle => "随机播放",
                        }),
                        set_size_request: (36, 36),
                        connect_clicked => PlayerPageMsg::ToggleMode,
                    },

                    // 上一首
                    gtk::Button {
                        set_icon_name: "media-skip-backward-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Previous"),
                        set_size_request: (36, 36),
                        connect_clicked => PlayerPageMsg::PrevTrack,
                    },

                    // 播放/暂停
                    gtk::Button {
                        #[track = "model.changed(PlayerPage::is_playing())"]
                        set_icon_name: if model.is_playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" },
                        add_css_class: "suggested-action",
                        set_size_request: (56, 36),
                        set_tooltip_text: Some("Play/Pause"),
                        connect_clicked => PlayerPageMsg::TogglePlay,
                    },

                    // 下一首
                    gtk::Button {
                        set_icon_name: "media-skip-forward-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Next"),
                        set_size_request: (36, 36),
                        connect_clicked => PlayerPageMsg::NextTrack,
                    },

                    // 循环模式（右）：ToggleButton，raised=开启，flat=关闭
                    gtk::ToggleButton {
                        #[track = "model.changed(PlayerPage::loop_enabled())"]
                        set_icon_name: "media-playlist-repeat-symbolic",
                        #[watch]
                        set_active: model.loop_enabled,
                        connect_active_notify[sender] => move |btn| {
                            sender.input(PlayerPageMsg::ToggleLoop(btn.is_active()));
                        },
                    }
                }
            }
        }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let is_seeking = Rc::new(Cell::new(false));

        // 【注意】这里的 progress_scale 必须先给一个默认值，等 view_output! 之后再替换
        let mut model = Self {
            song: Song::default(),
            is_playing: false,
            is_liked: false,
            position: 0,
            volume: 0.8,
            play_mode: PlayMode::Sequential,
            loop_enabled: true,
            progress_scale: gtk::Scale::default(), // 临时占位
            is_seeking: is_seeking.clone(),
            tracker: 0,

            playlist: Arc::new(Playlist::default()),
        };
        let widgets = view_output!();

        model.progress_scale = widgets.progress_scale.clone();

        // 【修改】绑定信号，不再保存 ID，而是捕获 is_seeking
        widgets
            .progress_scale
            .connect_change_value(move |_, _, val| {
                if !is_seeking.get() {
                    sender.input(PlayerPageMsg::Seek(val as u64));
                }
                glib::Propagation::Proceed
            });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match message {
            PlayerPageMsg::UpdateTrack(song) => {
                self.set_song(song);
            }
            PlayerPageMsg::UpdatePlayback(is_playing) => {
                self.set_is_playing(is_playing);
            }
            PlayerPageMsg::UpdateProgress { position, duration } => {
                self.set_position(position);

                self.progress_scale.set_range(0.0, duration as f64);
                self.is_seeking.set(true);
                self.progress_scale.set_value(position as f64);
                self.is_seeking.set(false);
            }
            PlayerPageMsg::TogglePlay => {
                sender.output(PlayerPageOutput::TogglePlay).unwrap();
            }
            PlayerPageMsg::PrevTrack => {
                sender.output(PlayerPageOutput::PrevTrack).unwrap();
            }
            PlayerPageMsg::NextTrack => {
                sender.output(PlayerPageOutput::NextTrack).unwrap();
            }
            PlayerPageMsg::Seek(val) => {
                // eprintln!("seek: {}", val);
                self.is_seeking.set(true);
                self.progress_scale.set_value(val as f64); // 乐观更新
                self.is_seeking.set(false);
                self.set_position(val);
                sender.output(PlayerPageOutput::Seek(val)).unwrap();
            }

            PlayerPageMsg::ToggleLike => {
                let new_liked = !self.is_liked;
                self.set_is_liked(new_liked);
                let song_id = self.song.id;
                sender.output(PlayerPageOutput::ToggleLike(song_id, new_liked)).unwrap();
            }
            PlayerPageMsg::VolumeChanged(val) => {
                self.volume = val;
            }
            PlayerPageMsg::ToggleMode => {
                let next = match self.play_mode {
                    PlayMode::Sequential => PlayMode::SingleLoop,
                    PlayMode::SingleLoop => PlayMode::Shuffle,
                    PlayMode::Shuffle => PlayMode::Sequential,
                };
                self.set_play_mode(next);
                sender.output(PlayerPageOutput::SetMode(next)).unwrap();
            }
            PlayerPageMsg::ToggleLoop(enabled) => {
                self.set_loop_enabled(enabled);
                sender.output(PlayerPageOutput::SetLoop(enabled)).unwrap();
            }
            PlayerPageMsg::SetQueue {
                tracks,
                playlist,
                start_index,
            } => {
                self.set_playlist(playlist);
            }
            PlayerPageMsg::ArtistClicked => {
                sender
                    .output(PlayerPageOutput::OpenArtistDialog(
                        self.song.artists.clone(),
                    ))
                    .unwrap();
            }
            PlayerPageMsg::AlbumClicked => {
                sender
                    .output(PlayerPageOutput::Navigate(AppRoute::PlaylistDetail(
                        PlaylistType::Album(self.song.album.id.clone()),
                    )))
                    .unwrap();
            }
            PlayerPageMsg::PlaylistClicked => {
                sender
                    .output(PlayerPageOutput::Navigate(AppRoute::PlaylistDetail(
                        PlaylistType::Playlist(self.playlist.id.clone()),
                    )))
                    .unwrap();
            }
            PlayerPageMsg::CollectClicked => {
                sender.output(PlayerPageOutput::CollectSong(self.song.id)).unwrap();
            }
            PlayerPageMsg::SetLiked(liked) => {
                self.set_is_liked(liked);
            }
        }
    }
}

// 辅助函数：将毫秒格式化为 mm:ss
fn format_time(ms: u64) -> String {
    let total_sec = ms / 1000;
    let mins = total_sec / 60;
    let secs = total_sec % 60;
    format!("{}:{:02}", mins, secs)
}
