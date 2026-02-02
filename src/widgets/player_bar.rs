use gtk::prelude::*;
use relm4::{gtk, ComponentParts, SimpleComponent};

#[derive(Debug)]
pub enum PlayerBarInput {
    // 更新当前播放歌曲
    UpdateCurrentSong { title: String, artist: String },
    // 更新播放状态
    UpdatePlayState(bool),
    // 更新进度
    UpdateProgress(f64),
}

#[derive(Debug)]
pub enum PlayerBarOutput {
    // 播放/暂停
    TogglePlay,
    // 上一首
    Previous,
    // 下一首
    Next,
    // 进度改变
    Seek(f64),
}

pub struct PlayerBar {
    is_playing: bool,
    current_progress: f64,
    current_song: Option<(String, String)>, // (title, artist)
}

impl SimpleComponent for PlayerBar {
    type Init = ();
    type Input = PlayerBarInput;
    type Output = PlayerBarOutput;
    type Widgets = PlayerBarWidgets;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_css_classes: &["player-bar", "toolbar"],
            set_spacing: 12,
            set_margin_start: 12,
            set_margin_end: 12,
            set_margin_top: 8,
            set_margin_bottom: 8,

            // 封面占位
            append = &gtk::Image {
                set_width_request: 64,
                set_height_request: 64,
                set_css_classes: &["album-cover"],
            }

            // 歌曲信息
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_hexpand: true,
                set_valign: gtk::Align::Center,

                append = &gtk::Label {
                    set_label: "未播放",
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["song-title"],
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                }

                append = &gtk::Label {
                    set_label: "",
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["song-artist"],
                    add_css_class: "dim-label",
                }
            }

            // 播放控制按钮
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_valign: gtk::Align::Center,

                // 上一首
                append = &gtk::Button {
                    set_icon_name: "media-skip-backward-symbolic",
                    set_css_classes: &["flat", "image-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(PlayerBarOutput::Previous);
                    }
                }

                // 播放/暂停
                append: play_button = &gtk::Button {
                    set_icon_name: "media-playback-start-symbolic",
                    set_css_classes: &["circular", "suggested-action"],
                    set_width_request: 48,
                    set_height_request: 48,
                    connect_clicked[sender] => move |_| {
                        sender.output(PlayerBarOutput::TogglePlay);
                    }
                }

                // 下一首
                append = &gtk::Button {
                    set_icon_name: "media-skip-forward-symbolic",
                    set_css_classes: &["flat", "image-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(PlayerBarOutput::Next);
                    }
                }
            }

            // 音量控制
            append = &gtk::VolumeButton {
                set_valign: gtk::Align::Center,
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PlayerBar {
            is_playing: false,
            current_progress: 0.0,
            current_song: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            PlayerBarInput::UpdateCurrentSong { title, artist } => {
                self.current_song = Some((title, artist));
            }
            PlayerBarInput::UpdatePlayState(is_playing) => {
                self.is_playing = is_playing;
            }
            PlayerBarInput::UpdateProgress(progress) => {
                self.current_progress = progress;
            }
        }
    }

    fn pre_view() {
        // 更新 UI 状态
    }
}
