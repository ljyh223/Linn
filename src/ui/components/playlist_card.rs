use relm4::factory::{DynamicIndex, FactoryComponent};
use relm4::gtk::prelude::{BoxExt, ButtonExt, GestureSingleExt, OrientableExt, WidgetExt};
use relm4::{gtk, prelude::*};
use crate::ui::components::image::AsyncImage;

// ------------------- 公共数据结构（无需改动） -------------------

#[derive(Debug)]
pub struct PlaylistCardInit {
    pub id: u64,
    pub cover_url: String,
    pub title: String,
    pub show_play_button: bool,
}

impl PlaylistCardInit {
    pub fn new(id: u64, cover_url: String, title: String) -> Self {
        Self { id, cover_url, title, show_play_button: true }
    }
}

#[derive(Debug)]
pub enum PlaylistCardOutput {
    Clicked(u64),
    ClickedPlaylist(u64),
}

// ------------------- 核心宏：消除重复代码 -------------------

macro_rules! define_playlist_card {
    ($name:ident, $parent_widget:ty) => {
        #[derive(Debug)]
        pub struct $name {
            id: u64,
            cover_url: String,
            title: String,
            show_play_button: bool,
        }

        #[relm4::factory(pub)]
        impl FactoryComponent for $name {
            type Init = PlaylistCardInit;
            type Input = ();
            type Output = PlaylistCardOutput;
            type CommandOutput = ();
            // 魔法就在这里：动态指定父容器类型
            type ParentWidget = $parent_widget;

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

                    gtk::Overlay {
                        set_width_request: 160,
                        set_height_request: 160,

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
    };
}

// ------------------- 按需实例化组件 -------------------

// 1. 用于 FlowBox 的卡片（保持原名，不影响你现有的 Home 页面代码）
define_playlist_card!(PlaylistCard, gtk::FlowBox);

// 2. 用于普通 gtk::Box 的卡片
define_playlist_card!(BoxPlaylistCard, gtk::Box);

// 3. 如果你以后想放在 Grid 里，直接加一行即可：
// define_playlist_card!(GridPlaylistCard, gtk::Grid);
