use relm4::{gtk::{self, prelude::*}, ComponentParts, ComponentSender, SimpleComponent};

#[derive(Debug)]
pub enum PlayerBarInput {
    UpdateCurrentSong { title: String, artist: String },
    UpdatePlayState(bool),
    UpdateProgress(f64),
}

#[derive(Debug)]
pub enum PlayerBarOutput {
    TogglePlay,
    Previous,
    Next,
    Seek(f64),
}

pub struct PlayerBar {
    is_playing: bool,
    current_progress: f64,
    current_song: Option<(String, String)>,
}

#[relm4::component(pub)]
impl SimpleComponent for PlayerBar {
    type Init = ();
    type Input = PlayerBarInput;
    type Output = PlayerBarOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_css_classes: &["player-bar", "toolbar"],
            set_spacing: 12,
            set_margin_start: 12,
            set_margin_end: 12,
            set_margin_top: 8,
            set_margin_bottom: 8,

            gtk::Image {
                set_width_request: 64,
                set_height_request: 64,
                set_css_classes: &["album-cover"],
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_hexpand: true,
                set_valign: gtk::Align::Center,

                gtk::Label {
                    set_label: "未播放",
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["song-title"],
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },

                gtk::Label {
                    set_label: "",
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["song-artist"],
                    add_css_class: "dim-label",
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_valign: gtk::Align::Center,

                gtk::Button {
                    set_icon_name: "media-skip-backward-symbolic",
                    set_css_classes: &["flat", "image-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(PlayerBarOutput::Previous);
                    }
                },

                gtk::Button {
                    set_icon_name: "media-playback-start-symbolic",
                    set_css_classes: &["circular", "suggested-action"],
                    set_width_request: 48,
                    set_height_request: 48,
                    connect_clicked[sender] => move |_| {
                        sender.output(PlayerBarOutput::TogglePlay);
                    }
                },

                gtk::Button {
                    set_icon_name: "media-skip-forward-symbolic",
                    set_css_classes: &["flat", "image-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(PlayerBarOutput::Next);
                    }
                },
            },

            gtk::VolumeButton {
                set_valign: gtk::Align::Center,
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
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
}
