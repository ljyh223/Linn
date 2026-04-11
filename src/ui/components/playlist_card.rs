//! 歌单卡片公共组件

use relm4::gtk::prelude::{BoxExt};
use relm4::gtk;

use crate::api::Playlist;
use crate::async_image;
/// 歌单卡片组件
pub struct PlaylistCard {
    inner: gtk::Box,
}

impl PlaylistCard {
    pub fn new(playlist: &Playlist) -> Self {
        let inner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .halign(gtk::Align::Start)
            .width_request(160)
            .build();

        // 封面 URL
        let cover_url = format!("{}?param=160y160", playlist.cover_url);

        log::debug!("Loading playlist cover from URL: {}", cover_url);

        // 使用 AsyncImage 宏
        let cover = async_image!(
            &cover_url,
            size: (160, 160), 
            radius: Lg,
        );

        inner.append(cover.widget());

        // 歌单名称
        let name_label = gtk::Label::builder()
            .label(&playlist.name)
            .halign(gtk::Align::Start)
            .lines(2)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .wrap(true)
            .css_classes(["caption"])
            .build();

        inner.append(&name_label);

        Self { inner }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.inner.as_ref()
    }
}