//! 单曲分列横滚组件（每列 3 首，多列横向滚动）
//!
//! 搜索页与发现页共用：输入 `Vec<Song>`，内部按 3 首一列分组铺进
//! `ScrollableRow` 的横向滚动区，点击任意行输出 `Clicked(id)`。

use relm4::factory::FactoryVecDeque;
use relm4::gtk::prelude::*;
use relm4::{gtk, prelude::*};

use crate::api::Song;
use crate::ui::components::scrollable_row::ScrollableRow;
use crate::ui::components::song_row::{SongRow, SongRowInit, SongRowOutput};

#[derive(Debug)]
pub struct SongListScrollInit {
    pub title: String,
    pub min_height: i32,
    pub max_height: i32,
}

impl SongListScrollInit {
    pub fn new(title: impl Into<String>, min_height: i32, max_height: i32) -> Self {
        Self {
            title: title.into(),
            min_height,
            max_height,
        }
    }
}

#[derive(Debug)]
pub enum SongListScrollInput {
    SetSongs(Vec<Song>),
}

#[derive(Debug)]
pub enum SongListScrollOutput {
    Clicked(u64),
}

pub struct SongListScroll {
    scroll: Controller<ScrollableRow>,
    columns: Vec<(gtk::Box, FactoryVecDeque<SongRow>)>,
    songs: Vec<Song>,
}

#[relm4::component(pub)]
impl SimpleComponent for SongListScroll {
    type Init = SongListScrollInit;
    type Input = SongListScrollInput;
    type Output = SongListScrollOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let scroll = ScrollableRow::new(init.title, init.min_height, init.max_height);
        // 一列 = 行宽 240 + 列内边距 32；三列 + 两处行距 = 848，强制最小内容宽保证三列可见
        scroll
            .widgets()
            .scrolled
            .set_min_content_width(3 * (240 + 32) + 2 * 16);
        let mut model = Self {
            scroll,
            columns: Vec::new(),
            songs: Vec::new(),
        };
        let widgets = view_output!();
        root.append(model.scroll.widget());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SongListScrollInput::SetSongs(songs) => {
                self.songs = songs;
                self.clear_columns();
                let content = self.scroll.model().content_box();
                for chunk in self.songs.chunks(3) {
                    let column = gtk::Box::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .spacing(16)
                        .margin_start(16)
                        .margin_end(16)
                        .build();

                    let mut factory = FactoryVecDeque::builder().launch(column.clone()).forward(
                        sender.output_sender(),
                        |out| match out {
                            SongRowOutput::Clicked(id) => SongListScrollOutput::Clicked(id),
                        },
                    );
                    {
                        let mut guard = factory.guard();
                        for song in chunk {
                            guard.push_back(SongRowInit {
                                id: song.id,
                                name: song.name.clone(),
                                artists: song
                                    .artists
                                    .iter()
                                    .take(2)
                                    .map(|a| a.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                cover_url: format!("{}?param=100y100", song.cover_url),
                            });
                        }
                    }
                    content.append(&column);
                    self.columns.push((column, factory));
                }
            }
        }
    }
}

impl SongListScroll {
    /// 卸载所有单曲列（列容器从滚动内容移除，行工厂 drop 自动清理子控件）
    fn clear_columns(&mut self) {
        for (column, factory) in self.columns.drain(..) {
            column.unparent();
            drop(factory);
        }
    }
}
