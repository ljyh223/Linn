use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use relm4::gtk::Orientation;
use relm4::gtk::{self, glib, prelude::*};
use relm4::prelude::*;

use crate::api::{Playlist, Song};
use crate::ui::components::image::AsyncImage;

#[tracker::track]
pub struct PlayerPage {
    cover_url: String,
    song_name: String,
    artist_album: String,
    is_playing: bool,
    is_liked: bool,
    playlist: Arc<Playlist>,
    // 进度 (单位: 秒)
    position: u64,
    duration: u64,

    volume: f64,

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
    Play(usize),
}

#[derive(Debug)]
pub enum PlayerPageMsg {
    // 来自外部的状态更新
    UpdateTrack(Song),
    UpdatePlayback(bool),
    SetQueue {
        songs: Arc<Vec<Song>>,
        playlist: Arc<Playlist>,
        start_index: usize,
    },
    // 来自播放器的进度更新, 单位是毫秒
    UpdateProgress {
        position: u64,
        duration: u64,
    },

    // 用户交互
    TogglePlay,
    PrevTrack,
    NextTrack,
    Seek(u64),
    ToggleLike,
    VolumeChanged(f64),
    ToggleMode,
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
            set_width_request: 320,
            set_vexpand: true,

            gtk::CenterBox {
                set_orientation: Orientation::Horizontal,
                set_margin_bottom: 16,
                set_width_request: 280,
                #[wrap(Some)]
                set_start_widget = &gtk::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 2, // 建议加一点点间距，让两行文字不那么贴在一起
                    set_halign: gtk::Align::Start, // Box 本身也靠左对齐

                    gtk::Label {
                        set_label: "Playing From",
                        set_halign: gtk::Align::Start, // 左对齐
                        add_css_class: "dim-label",    // 次色（灰色）
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
                    set_url: model.playlist.cover_url.clone(),
                    set_width_request: 36,
                    set_height_request: 36,
                    set_corner_radius: 4.0,
                    set_placeholder_icon: "folder-music-symbolic",
                },
            },




            // ================= 2. 封面 =================
            AsyncImage {
                set_width_request: 260,
                set_height_request: 260,
                set_corner_radius: 16.0,
                #[track = "model.changed(PlayerPage::cover_url())"]
                set_url: model.cover_url.clone(),
                set_placeholder_icon: "folder-music-symbolic",
                set_fallback_icon: "missing-album",
            },

            // ================= 3. 歌手和专辑信息 =================
            gtk::Box {
                set_orientation: Orientation::Vertical,
                set_spacing: 4,
                set_halign: gtk::Align::Center,

                gtk::Label {
                    #[track = "model.changed(PlayerPage::song_name())"]
                    set_label: &model.song_name,
                    add_css_class: "title-1",
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_width_chars: 20,
                },
                gtk::Label {
                    #[track = "model.changed(PlayerPage::artist_album())"]
                    set_label: &model.artist_album,
                    add_css_class: "body",
                    add_css_class: "dim-label",
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_width_chars: 25,
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
                // 音量条
                // gtk::Scale {
                //     set_orientation: Orientation::Horizontal,
                //     set_width_request: 120,
                //     set_adjustment: &gtk::Adjustment::new(0.0, 0.0, 1.0, 0.01, 0.1, 0.0),
                //     set_draw_value: false,
                //     set_value: model.volume,
                //     add_css_class: "flat",
                //     connect_value_changed[sender] => move |scale| {
                //         sender.input(PlayerPageMsg::VolumeChanged(scale.value()));
                //     }
                // },

                // gtk::Separator { set_orientation: Orientation::Vertical },

                // 喜欢按钮
                gtk::Button {
                    #[track = "model.changed(PlayerPage::is_liked())"]
                    set_icon_name: if model.is_liked { "heart-filled" } else { "heart-outline-thick" },
                    add_css_class: "flat",
                    set_tooltip_text: Some("Like"),
                    connect_clicked => PlayerPageMsg::ToggleLike,
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
                        #[track = "model.changed(PlayerPage::duration())"]
                        set_label: &format_time(model.duration),
                        add_css_class: "caption",
                        add_css_class: "dim-label",
                    }
                },

            },

            // ================= 6. 控制器 =================
            gtk::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 16,
                set_halign: gtk::Align::Center,
                set_margin_top: 8,

                // 播放模式
                gtk::Button {
                    set_icon_name: "media-playlist-repeat-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Play mode"),
                    set_size_request: (36, 36),
                    connect_clicked => PlayerPageMsg::ToggleMode,
                },

                // 上一首
                gtk::Button {
                    set_icon_name: "media-skip-backward-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Previous"),
                    set_size_request: (42, 42),
                    connect_clicked => PlayerPageMsg::PrevTrack,
                },

                // 播放/暂停 (核心大按钮)
                gtk::Button {
                    #[track = "model.changed(PlayerPage::is_playing())"]
                    set_icon_name: if model.is_playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" },
                    add_css_class: "circular",
                    add_css_class: "suggested-action",
                    set_size_request: (56, 56),
                    set_tooltip_text: Some("Play/Pause"),
                    connect_clicked => PlayerPageMsg::TogglePlay,
                },

                // 下一首
                gtk::Button {
                    set_icon_name: "media-skip-forward-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Next"),
                    set_size_request: (42, 42),
                    connect_clicked => PlayerPageMsg::NextTrack,
                },

                // 播放列表/队列
                gtk::Button {
                    set_icon_name: "view-list-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Queue"),
                    set_size_request: (36, 36),
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
            cover_url: String::new(),
            song_name: "未在播放".to_string(),
            artist_album: "选择一首歌曲开始".to_string(),
            is_playing: false,
            is_liked: false,
            position: 0,
            duration: 0,
            volume: 0.8,
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
                self.set_song_name(song.name);
                self.set_artist_album(format!(
                    "{} - {}",
                    song.artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>()
                        .join("/"),
                    song.album.name
                ));
                self.set_cover_url(song.cover_url);
            }
            PlayerPageMsg::UpdatePlayback(is_playing) => {
                self.set_is_playing(is_playing);
            }
            PlayerPageMsg::UpdateProgress { position, duration } => {
                // eprintln!("progress: {} / {}", position, duration);
                self.set_position(position);
                self.set_duration(duration);

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

            // ToggleLike, VolumeChanged 这些不需要发给后端的，保持原样不动即可
            PlayerPageMsg::ToggleLike => {
                self.is_liked = !self.is_liked;
            }
            PlayerPageMsg::VolumeChanged(val) => {
                self.volume = val;
            }
            PlayerPageMsg::ToggleMode => {}
            PlayerPageMsg::SetQueue {
                songs,
                playlist,
                start_index,
            } => {
                self.set_playlist(playlist);
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
