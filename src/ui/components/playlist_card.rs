use relm4::factory::FactoryComponent;
use relm4::gtk::prelude::{BoxExt, ButtonExt};
use relm4::{gtk::{self, prelude::{GestureSingleExt, OrientableExt, WidgetExt}}, prelude::*};
use relm4::factory::DynamicIndex;

use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub struct PlaylistCardInit {
    pub id: u64,
    pub cover_url: String,
    pub title: String,
    pub show_play_button: bool, // 新增，默认 true
}

impl PlaylistCardInit {
    pub fn new(id: u64, cover_url: String, title: String) -> Self {
        Self { id, cover_url, title, show_play_button: true }
    }
}

#[derive(Debug)]
pub struct PlaylistCard {
    id: u64,
    cover_url: String,
    title: String,
    show_play_button: bool,
}

#[derive(Debug)]
pub enum PlaylistCardOutput {
    Clicked(u64),
    ClickedPlaylist(u64),
}

#[relm4::factory(pub)]
impl FactoryComponent for PlaylistCard {
    type Init = PlaylistCardInit;
    type Input = ();
    type Output = PlaylistCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            set_hexpand: false,
            set_vexpand: false,
            set_width_request: 160,
            add_css_class: "playlist-card",

            // Overlay 叠层容器
            gtk::Overlay {
                set_width_request: 160,
                set_height_request: 160,

                // 底层：封面图片
                AsyncImage {
                    set_width_request: 160,
                    set_height_request: 160,
                    set_corner_radius: 8.0,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_url: self.cover_url.clone(),
                    set_placeholder_icon: "folder-music-symbolic",
                    set_fallback_icon: "image-missing-symbolic",
                    add_css_class: "rounded-cover",
                },

                // 遮罩层 + 播放按钮（叠在上面）
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
                        set_visible: self.show_play_button,
                        add_css_class: "cover-play-btn",
                        add_css_class: "circular",
                        // 点击直接触发 Clicked，和整个卡片行为一致
                        connect_clicked[sender, id = self.id] => move |_| {
                            sender.output(PlaylistCardOutput::ClickedPlaylist(id)).unwrap();
                        }
                    }
                },
            },

            gtk::Label {
                set_label: &self.title,
                set_halign: gtk::Align::Start,
                set_max_width_chars: 15,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "playlist-title",
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(PlaylistCardOutput::Clicked(id)).unwrap();
                    }
                }
            }
        }
    }

    fn init_model(
        init: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            id: init.id,
            cover_url: init.cover_url,
            title: init.title,
            show_play_button: init.show_play_button,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}