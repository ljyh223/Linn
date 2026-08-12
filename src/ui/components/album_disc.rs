//! CD 唱片造型卡片（新碟上架）
//!
//! 封面以圆形呈现（corner_radius = 宽度一半），下方碟名 + 歌手。

use relm4::gtk::prelude::*;
use relm4::{gtk, prelude::*};

use crate::api::Playlist;
use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub struct AlbumDiscInit {
    pub id: u64,
    pub cover_url: String,
    pub name: String,
    pub artist_name: String,
}

impl AlbumDiscInit {
    pub fn from_playlist(playlist: &Playlist) -> Self {
        Self {
            id: playlist.id,
            cover_url: playlist.cover_url.clone(),
            name: playlist.name.clone(),
            artist_name: playlist.creator_name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AlbumDiscOutput {
    Clicked(u64),
}

pub struct AlbumDisc {
    id: u64,
    cover_url: String,
    name: String,
    artist_name: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for AlbumDisc {
    type Init = AlbumDiscInit;
    type Input = ();
    type Output = AlbumDiscOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_valign: gtk::Align::Center,
            set_width_request: 160,

            // 圆形封面（CD 造型）
            gtk::Overlay {
                set_width_request: 160,
                set_height_request: 160,

                AsyncImage {
                    set_width_request: 160,
                    set_height_request: 160,
                    set_corner_radius: 80.0, // 圆形
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_url: format!("{}?param=300y300", self.cover_url),
                    set_placeholder_icon: "folder-music-symbolic",
                    set_fallback_icon: "image-missing-symbolic",
                },

                add_overlay = &gtk::Box {
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    add_css_class: "cover-hover-overlay",

                    gtk::Button {
                        set_icon_name: "media-playback-start-symbolic",
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::End,
                        set_margin_end: 8,
                        set_margin_bottom: 8,
                        set_hexpand: true,
                        set_vexpand: true,
                        add_css_class: "cover-play-btn",
                        add_css_class: "circular",
                        connect_clicked[sender, id = self.id] => move |_| {
                            sender.output(AlbumDiscOutput::Clicked(id)).unwrap();
                        }
                    }
                },
            },

            gtk::Label {
                set_label: &self.name,
                set_halign: gtk::Align::Center,
                set_max_width_chars: 16,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "caption-heading",
            },

            gtk::Label {
                set_label: &self.artist_name,
                set_halign: gtk::Align::Center,
                set_max_width_chars: 16,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "caption",
                add_css_class: "dim-label",
                set_opacity: 0.8,
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(AlbumDiscOutput::Clicked(id)).unwrap();
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            id: init.id,
            cover_url: init.cover_url,
            name: init.name,
            artist_name: init.artist_name,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}