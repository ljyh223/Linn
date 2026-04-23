use log::trace;
use relm4::{FactorySender, RelmWidgetExt, gtk::{self, prelude::*}, prelude::{DynamicIndex, FactoryComponent}};

use crate::{api::Song, ui::components::image::AsyncImage};


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

#[derive(Debug)]
pub enum TrackRowOutput {
    PlayClicked(u64),
    MoreClicked(u64),
}

#[relm4::factory(pub)]
impl FactoryComponent for TrackRow {
    type Init = TrackRowInit;
    type Input = ();
    type Output = TrackRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        // 使用水平 Box 布局
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 16,
            set_margin_all: 8,
            set_valign: gtk::Align::Center,


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
                    set_max_width_chars: 20,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "heading", // GTK 自带样式：加粗标题
                },
                gtk::Label {
                    set_label: &self.track.artists.iter().take(3).map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
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
                        sender.output(TrackRowOutput::PlayClicked(track_id)).unwrap();
                    }
                },
                gtk::Button {
                    set_icon_name: "view-more-symbolic",
                    add_css_class: "circular",
                    add_css_class: "flat",
                    set_tooltip_text: Some("更多选项"),
                    connect_clicked[sender, track_id = self.track.id] => move |_| {
                        trace!("点击了更多按钮，ID: {}", track_id);
                        sender.output(TrackRowOutput::MoreClicked(track_id)).unwrap();
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