use std::{cell::RefCell, rc::Rc};

use log::trace;
use relm4::{
    FactorySender, RelmWidgetExt,
    gtk::{self, prelude::*},
    prelude::{DynamicIndex, FactoryComponent},
    typed_view::list::RelmListItem,
};

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
                set_width_request: 300,

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

/// 供 [`TypedListView`](relm4::typed_view::list::TypedListView) 使用的歌曲列表项。
///
/// 与上面基于 `FactoryComponent` 的 `TrackRow` 不同，`TypedListView` 需要实现
/// [`RelmListItem`]，行内的按钮事件通过 item 自身携带的回调转发给父组件。
#[derive(Clone)]
pub struct TrackListItem {
    pub track: Song,
    pub index: usize,
    pub(crate) on_play: Rc<dyn Fn(u64)>,
    pub(crate) on_more: Rc<dyn Fn(u64)>,
}

impl std::fmt::Debug for TrackListItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackListItem")
            .field("track", &self.track)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl TrackListItem {
    pub fn new(
        track: Song,
        index: usize,
        on_play: Rc<dyn Fn(u64)>,
        on_more: Rc<dyn Fn(u64)>,
    ) -> Self {
        Self {
            track,
            index,
            on_play,
            on_more,
        }
    }
}

/// 行内按钮需要访问的当前行数据 + 回调。
///
/// 由于 ListView 的行 widget 会被复用，不能直接把按钮信号连到具体某个 item，
/// 因此在 `setup` 中按钮统一读取这个共享状态，`bind` 时再写入当前行的内容。
struct RowAction {
    track_id: u64,
    on_play: Rc<dyn Fn(u64)>,
    on_more: Rc<dyn Fn(u64)>,
}

pub struct TrackListItemWidgets {
    action: Rc<RefCell<Option<RowAction>>>,
    image: AsyncImage,
    name: gtk::Label,
    artists: gtk::Label,
    album: gtk::Label,
}

impl RelmListItem for TrackListItem {
    type Root = gtk::Box;
    type Widgets = TrackListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Self::Widgets) {
        let action: Rc<RefCell<Option<RowAction>>> = Rc::new(RefCell::new(None));

        relm4::view! {
            my_box = gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 16,
                set_margin_all: 8,
                set_valign: gtk::Align::Center,

                #[name = "image"]
                AsyncImage {
                    set_width_request: 48,
                    set_height_request: 48,
                    set_corner_radius: 4.0,
                    set_placeholder_icon: "missing-album-symbolic",
                },

                // 左中侧：歌名与歌手
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,
                    set_spacing: 4,
                    set_width_request: 300,

                    #[name = "name"]
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_max_width_chars: 20,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        add_css_class: "heading",
                    },
                    #[name = "artists"]
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                    }
                },

                // 中间：专辑名（占满剩余空间）
                #[name = "album"]
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "dim-label",
                },

                // 右侧：功能按钮
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    set_valign: gtk::Align::Center,

                    gtk::Button {
                        set_icon_name: "media-playback-start-symbolic",
                        add_css_class: "circular",
                        add_css_class: "flat",
                        set_tooltip_text: Some("播放"),
                        connect_clicked[action] => move |_| {
                            if let Some(row) = action.borrow().as_ref() {
                                (row.on_play)(row.track_id);
                            }
                        }
                    },
                    gtk::Button {
                        set_icon_name: "view-more-symbolic",
                        add_css_class: "circular",
                        add_css_class: "flat",
                        set_tooltip_text: Some("更多选项"),
                        connect_clicked[action] => move |_| {
                            if let Some(row) = action.borrow().as_ref() {
                                (row.on_more)(row.track_id);
                            }
                        }
                    }
                }
            }
        }

        let widgets = TrackListItemWidgets {
            action,
            image,
            name,
            artists,
            album,
        };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        *widgets.action.borrow_mut() = Some(RowAction {
            track_id: self.track.id,
            on_play: self.on_play.clone(),
            on_more: self.on_more.clone(),
        });

        widgets
            .image
            .set_url(format!("{}?param=100y100", self.track.cover_url));
        widgets.name.set_label(&self.track.name);
        widgets.artists.set_label(
            &self
                .track
                .artists
                .iter()
                .take(3)
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        widgets.album.set_label(&self.track.album.name);
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        *widgets.action.borrow_mut() = None;
    }
}
