//! 排行榜区：网易云式并列榜卡（封面 + 榜名 + 前三首歌名）
//!
//! `ToplistBoard` 用 FlowBox 承载等宽榜卡，自适应列数；
//! 每卡前三首歌异步到达（`ToplistBoardInput::SetSongs { index, songs }`）。

use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};
use relm4::gtk::prelude::*;
use relm4::{gtk, prelude::*};

use crate::api::{Playlist, Song};
use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub struct BoardCardData {
    pub id: u64,
    pub cover_url: String,
    pub name: String,
    pub update_frequency: String,
}

impl BoardCardData {
    pub fn from_playlist(p: &Playlist) -> Self {
        Self {
            id: p.id,
            cover_url: p.cover_url.clone(),
            name: p.name.clone(),
            update_frequency: p.description.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ToplistBoardOutput {
    Clicked(u64),
}

pub fn artist_names(song: &Song) -> String {
    song.artists
        .iter()
        .take(3)
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// ── 榜卡（每个榜单一张，前三首歌异步填充）────────────────────

pub struct ToplistCard {
    id: u64,
    cover_url: String,
    name: String,
    update_frequency: String,
    /// 前三首歌文案（"1 歌名 — 歌手"），空字符串表示未加载
    song_lines: [String; 3],
}

#[relm4::factory(pub)]
impl FactoryComponent for ToplistCard {
    type Init = BoardCardData;
    type Input = ();
    type Output = ToplistBoardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_width_request: 260,
            set_halign: gtk::Align::Start,
            set_hexpand: false,
            set_vexpand: false,
            add_css_class: "rank-card",

            // 顶行：榜名（居左） + 更新时间（右上角）
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,

                gtk::Label {
                    set_label: &self.name,
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    set_max_width_chars: 14,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "caption-heading",
                },

                gtk::Label {
                    set_label: &self.update_frequency,
                    set_halign: gtk::Align::End,
                    add_css_class: "caption",
                    add_css_class: "dim-label",
                },
            },

            // 中部：左封面 + 右前三首
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 14,

                AsyncImage {
                    set_width_request: 96,
                    set_height_request: 96,
                    set_corner_radius: 10.0,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_url: format!("{}?param=250y250", self.cover_url),
                    set_placeholder_icon: "view-list-symbolic",
                    set_fallback_icon: "image-missing-symbolic",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 6,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,

                    gtk::Label {
                        #[watch]
                        set_label: &self.song_lines[0],
                        set_halign: gtk::Align::Start,
                        set_max_width_chars: 20,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        #[watch]
                        set_visible: !self.song_lines[0].is_empty(),
                        add_css_class: "caption",
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &self.song_lines[1],
                        set_halign: gtk::Align::Start,
                        set_max_width_chars: 20,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        #[watch]
                        set_visible: !self.song_lines[1].is_empty(),
                        add_css_class: "caption",
                        add_css_class: "dim-label",
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &self.song_lines[2],
                        set_halign: gtk::Align::Start,
                        set_max_width_chars: 20,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        #[watch]
                        set_visible: !self.song_lines[2].is_empty(),
                        add_css_class: "caption",
                        add_css_class: "dim-label",
                    },
                },
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(ToplistBoardOutput::Clicked(id)).unwrap();
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
            update_frequency: init.update_frequency,
            song_lines: [String::new(), String::new(), String::new()],
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}

// ── Board 容器：FlowBox 等宽网格 ─────────────────────────────

#[derive(Debug)]
pub struct ToplistBoardInit {
    pub cards: Vec<BoardCardData>,
}

#[derive(Debug)]
pub enum ToplistBoardInput {
    /// 按 index 填充某个榜卡的前三首歌
    SetSongs { index: usize, songs: Vec<Song> },
}

pub struct ToplistBoard {
    cards: FactoryVecDeque<ToplistCard>,
}

#[relm4::component(pub)]
impl SimpleComponent for ToplistBoard {
    type Init = ToplistBoardInit;
    type Input = ToplistBoardInput;
    type Output = ToplistBoardOutput;

    view! {
        #[root]
        gtk::FlowBox {
            set_valign: gtk::Align::Start,

            #[local_ref]
            cards -> gtk::FlowBox {
                set_halign: gtk::Align::Fill,
                set_hexpand: true,
                set_homogeneous: true,
                set_max_children_per_line: 3,
                set_min_children_per_line: 3,
                set_column_spacing: 16,
                set_row_spacing: 16,
                set_selection_mode: gtk::SelectionMode::None,
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let cards = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(sender.output_sender(), |msg| msg);

        let mut model = ToplistBoard { cards };

        {
            let mut guard = model.cards.guard();
            for card in init.cards {
                guard.push_back(card);
            }
        }

        let cards = model.cards.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            ToplistBoardInput::SetSongs { index, songs } => {
                let mut guard = self.cards.guard();
                if let Some(card) = guard.get_mut(index) {
                    for (i, line) in card.song_lines.iter_mut().enumerate() {
                        *line = songs
                            .get(i)
                            .map(|s| format!("{}  {} — {}", i + 1, s.name, artist_names(s)))
                            .unwrap_or_default();
                    }
                }
            }
        }
    }
}
