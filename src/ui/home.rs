use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use futures::{StreamExt, stream};
use log::trace;
use relm4::factory::FactoryVecDeque;
use relm4::gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, gtk, prelude::*};
use tokio_util::sync::CancellationToken;

use super::components::home_block_card::{HomeBlockCard, HomeBlockCardInit, HomeBlockCardOutput};
use super::components::image::image_manager::ImageManager;
use super::components::playlist_card::{BoxPlaylistCard, PlaylistCardInit, PlaylistCardOutput};
use super::components::scrollable_row::ScrollableRow;
use super::components::song_list::{
    SongListScroll, SongListScrollInit, SongListScrollInput, SongListScrollOutput,
};
use crate::api::{HomeBlockType, HomeSection, Song, get_home_block, get_song_detail};
use crate::ui::model::PlaylistType;
use crate::utils::utils::{extract_dominant_color, image_url};

const COLOR_EXTRACTION_CONCURRENCY: usize = 6;

pub struct Home {
    sections: Vec<HomeSection>,
    section_widgets: Vec<SectionWidgets>,
    sections_slot: gtk::Box,
    color_css_provider: gtk::CssProvider,
}

enum SectionWidgets {
    Playlist {
        _row: Controller<ScrollableRow>,
        _cards: FactoryVecDeque<BoxPlaylistCard>,
    },
    HomeBlock {
        _row: Controller<ScrollableRow>,
        _cards: FactoryVecDeque<HomeBlockCard>,
    },
    Songs {
        list: Controller<SongListScroll>,
        songs: Vec<Song>,
    },
}

fn uses_playlist_cards(position_code: &str) -> bool {
    matches!(
        position_code,
        "PAGE_RECOMMEND_RADAR"
            | "PAGE_RECOMMEND_SPECIAL_CLOUD_VILLAGE_PLAYLIST"
            | "PAGE_RECOMMEND_MIXED_ARTIST_PLAYLIST"
            | "PAGE_RECOMMEND_RANK"
            | "PAGE_RECOMMEND_MY_SHEET"
            | "PAGE_RECOMMEND_COMBINATION"
            | "PAGE_RECOMMEND_FEELING_PLAYLIST_LOCATION"
            | "PAGE_RECOMMEND_SCENE_PLAYLIST_LOCATION"
            | "PAGE_RECOMMEND_MONTH_YEAR_PLAYLIST"
    )
}

#[derive(Debug)]
pub enum HomeMsg {
    LoadHomeBlocks,
    HomeBlockCardAction {
        section_index: usize,
        output: HomeBlockCardOutput,
    },
    PlaylistCardAction(PlaylistCardOutput),
    RecommendationSongClicked {
        section_index: usize,
        id: u64,
    },
}

#[derive(Debug)]
pub enum HomeCmdMsg {
    HomeSectionsLoaded(Vec<HomeSection>),
    HomeBlockColorsLoaded(Vec<(usize, usize, String)>),
    QueueSongsLoaded(Vec<(usize, Vec<Song>)>),
}

#[derive(Debug)]
pub enum HomeOutput {
    OpenPlaylistDetail(u64),
    OpenDailyRecommend,
    OpenPlaylistType(PlaylistType),
    Playlist(PlaylistType),
    NavigateToArtist(u64),
    PlayTracks(Vec<Song>, usize),
}

#[relm4::component(pub)]
impl Component for Home {
    type Init = ();
    type Input = HomeMsg;
    type CommandOutput = HomeCmdMsg;
    type Output = HomeOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_vexpand: true,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 24,
                set_margin_top: 24,
                set_margin_bottom: 24,
                set_margin_start: 24,
                set_margin_end: 24,

                // ── EAPI 模块区块；每个 positionCode 独立渲染 ──
                #[name(sections_slot)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 24,
                },

            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            sections: Vec::new(),
            section_widgets: Vec::new(),
            sections_slot: gtk::Box::default(),
            color_css_provider: gtk::CssProvider::new(),
        };

        let widgets = view_output!();

        model.sections_slot = widgets.sections_slot.clone();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &model.color_css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        sender.input(HomeMsg::LoadHomeBlocks);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        trace!("Home: {message:?}");
        match message {
            HomeMsg::LoadHomeBlocks => {
                sender.command(|out, _shutdown| async move {
                    match get_home_block().await {
                        Ok(sections) => {
                            let mut visible_sections = Vec::new();
                            for mut section in sections {
                                section.blocks.retain(|block| match &block.type_ {
                                    HomeBlockType::Fm | HomeBlockType::Unknown => false,
                                    _ => true,
                                });
                                if !section.blocks.is_empty() {
                                    visible_sections.push(section);
                                }
                            }
                            let covers = visible_sections
                                .iter()
                                .enumerate()
                                .flat_map(|(section_index, section)| {
                                    section.blocks.iter().enumerate().filter_map(
                                        move |(block_index, block)| {
                                            (!block.cover.is_empty()).then(|| {
                                                (section_index, block_index, block.cover.clone())
                                            })
                                        },
                                    )
                                })
                                .collect::<Vec<_>>();
                            let queue_ids = visible_sections
                                .iter()
                                .flat_map(|section| section.blocks.iter())
                                .filter_map(|block| match &block.type_ {
                                    HomeBlockType::Queue(ids) => Some(ids),
                                    _ => None,
                                })
                                .flatten()
                                .copied()
                                .collect::<Vec<_>>();
                            // 首页数据命中本地缓存后应立即渲染；封面主色只是装饰，不能阻塞首屏。
                            let _ =
                                out.send(HomeCmdMsg::HomeSectionsLoaded(visible_sections.clone()));
                            if !queue_ids.is_empty() {
                                let mut seen = HashSet::new();
                                let mut unique_ids = queue_ids;
                                unique_ids.retain(|id| seen.insert(*id));
                                match get_song_detail(unique_ids).await {
                                    Ok(songs) => {
                                        let songs_by_id = songs
                                            .into_iter()
                                            .map(|song| (song.id, song))
                                            .collect::<HashMap<_, _>>();
                                        let queue_sections = visible_sections
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(index, section)| {
                                                let ids =
                                                    section.blocks.iter().find_map(|block| {
                                                        match &block.type_ {
                                                            HomeBlockType::Queue(ids) => Some(ids),
                                                            _ => None,
                                                        }
                                                    })?;
                                                Some((
                                                    index,
                                                    ids.iter()
                                                        .filter_map(|id| {
                                                            songs_by_id.get(id).cloned()
                                                        })
                                                        .collect(),
                                                ))
                                            })
                                            .collect();
                                        let _ =
                                            out.send(HomeCmdMsg::QueueSongsLoaded(queue_sections));
                                    }
                                    Err(error) => log::warn!("获取首页推荐歌曲失败: {error}"),
                                }
                            }

                            // 在卡片已渲染、封面加载进行时，再低优先级计算主色；结果到达后只更新 CSS。
                            let colors = stream::iter(covers.into_iter().map(
                                |(section_index, block_index, cover)| async move {
                                    let color = ImageManager::global()
                                        .fetch(
                                            image_url(&cover, "300y300"),
                                            CancellationToken::new(),
                                        )
                                        .await
                                        .map(|bytes| extract_dominant_color(&bytes))
                                        .unwrap_or_else(|_| "#333333".to_string());
                                    (section_index, block_index, color)
                                },
                            ))
                            .buffer_unordered(COLOR_EXTRACTION_CONCURRENCY)
                            .collect::<Vec<_>>()
                            .await;
                            let _ = out.send(HomeCmdMsg::HomeBlockColorsLoaded(colors));
                        }
                        Err(e) => log::error!("加载首页推荐块失败: {e}"),
                    }
                });
            }

            HomeMsg::HomeBlockCardAction {
                section_index,
                output: HomeBlockCardOutput::Clicked(card_index),
            } => {
                let Some(block) = self
                    .sections
                    .get(section_index)
                    .and_then(|section| section.blocks.get(card_index))
                else {
                    return;
                };
                match &block.type_ {
                    HomeBlockType::Playlist(id) => {
                        let _ = sender.output(HomeOutput::OpenPlaylistDetail(*id));
                    }
                    HomeBlockType::Album(id) => {
                        let _ =
                            sender.output(HomeOutput::OpenPlaylistType(PlaylistType::Album(*id)));
                    }
                    HomeBlockType::Daily => {
                        let _ = sender.output(HomeOutput::OpenDailyRecommend);
                    }
                    HomeBlockType::DailyCategory {
                        tag_id,
                        category_id,
                        song_id,
                    } => {
                        let _ = sender.output(HomeOutput::OpenPlaylistType(
                            PlaylistType::DailyCategory {
                                tag_id: *tag_id,
                                category_id: *category_id,
                                song_ids: song_id.clone(),
                                title: block.title.clone(),
                                cover: block.cover.clone(),
                            },
                        ));
                    }
                    HomeBlockType::Fm => {}
                    HomeBlockType::Queue(ids) => {
                        let ids = ids.clone();
                        sender.command(move |out, _shutdown| async move {
                            match get_song_detail(ids).await {
                                Ok(songs) => {
                                    let _ = out.send(HomeCmdMsg::QueueSongsLoaded(vec![(
                                        section_index,
                                        songs,
                                    )]));
                                }
                                Err(e) => log::error!("获取队列歌曲详情失败: {e}"),
                            }
                        });
                    }
                    HomeBlockType::Artist(ids) => {
                        if let Some(&first_id) = ids.first() {
                            let _ = sender.output(HomeOutput::NavigateToArtist(first_id));
                        }
                    }
                    HomeBlockType::Unknown => {}
                }
            }
            HomeMsg::PlaylistCardAction(action) => match action {
                PlaylistCardOutput::Clicked(id) => {
                    let _ = sender.output(HomeOutput::OpenPlaylistDetail(id));
                }
                PlaylistCardOutput::ClickedPlaylist(id) => {
                    let _ = sender.output(HomeOutput::Playlist(PlaylistType::Playlist(id)));
                }
            },
            HomeMsg::RecommendationSongClicked { section_index, id } => {
                if let Some(SectionWidgets::Songs { songs, .. }) =
                    self.section_widgets.get(section_index)
                    && let Some(start_index) = songs.iter().position(|song| song.id == id)
                {
                    let _ = sender.output(HomeOutput::PlayTracks(songs.clone(), start_index));
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            HomeCmdMsg::HomeSectionsLoaded(sections) => {
                while let Some(child) = self.sections_slot.first_child() {
                    self.sections_slot.remove(&child);
                }
                self.section_widgets.clear();
                self.sections = sections;
                self.refresh_home_block_colors();
                for (section_index, section) in self.sections.iter().cloned().enumerate() {
                    if section
                        .blocks
                        .iter()
                        .all(|block| matches!(block.type_, HomeBlockType::Queue(_)))
                    {
                        let list = SongListScroll::builder()
                            .launch(SongListScrollInit::new(section.title, 230, 230))
                            .forward(sender.input_sender(), move |out| match out {
                                SongListScrollOutput::Clicked(id) => {
                                    HomeMsg::RecommendationSongClicked { section_index, id }
                                }
                            });
                        self.sections_slot.append(list.widget());
                        self.section_widgets.push(SectionWidgets::Songs {
                            list,
                            songs: Vec::new(),
                        });
                        continue;
                    }
                    let row = ScrollableRow::new(section.title.clone(), 220, 220);
                    let content = row.widgets().content_box.clone();
                    if uses_playlist_cards(&section.position_code) {
                        let mut cards = FactoryVecDeque::builder()
                            .launch(content)
                            .forward(sender.input_sender(), HomeMsg::PlaylistCardAction);
                        {
                            let mut guard = cards.guard();
                            for block in &section.blocks {
                                let HomeBlockType::Playlist(id) = &block.type_ else {
                                    continue;
                                };
                                guard.push_back(PlaylistCardInit {
                                    id: *id,
                                    cover_url: crate::utils::utils::image_url(
                                        &block.cover,
                                        "300y300",
                                    ),
                                    title: block.title.clone(),
                                    subtitle: (!block.sub_title.is_empty())
                                        .then(|| block.sub_title.clone()),
                                    show_play_button: true,
                                });
                            }
                        }
                        self.sections_slot.append(row.widget());
                        self.section_widgets.push(SectionWidgets::Playlist {
                            _row: row,
                            _cards: cards,
                        });
                    } else {
                        let mut cards = FactoryVecDeque::builder().launch(content).forward(
                            sender.input_sender(),
                            move |output| HomeMsg::HomeBlockCardAction {
                                section_index,
                                output,
                            },
                        );
                        {
                            let mut guard = cards.guard();
                            for (card_index, block) in section.blocks.iter().enumerate() {
                                guard.push_back(HomeBlockCardInit {
                                    index: card_index,
                                    color_class: format!("hb-color-{section_index}-{card_index}"),
                                    cover_url: crate::utils::utils::image_url(
                                        &block.cover,
                                        "300y300",
                                    ),
                                    title: block.title.clone(),
                                    subtitle: block.sub_title.clone(),
                                });
                            }
                        }
                        self.sections_slot.append(row.widget());
                        self.section_widgets.push(SectionWidgets::HomeBlock {
                            _row: row,
                            _cards: cards,
                        });
                    }
                }
            }

            HomeCmdMsg::HomeBlockColorsLoaded(colors) => {
                for (section_index, block_index, color) in colors {
                    if let Some(block) = self
                        .sections
                        .get_mut(section_index)
                        .and_then(|section| section.blocks.get_mut(block_index))
                    {
                        block.color = color;
                    }
                }
                self.refresh_home_block_colors();
            }

            HomeCmdMsg::QueueSongsLoaded(queue_sections) => {
                for (section_index, songs) in queue_sections {
                    if let Some(SectionWidgets::Songs {
                        list,
                        songs: stored_songs,
                    }) = self.section_widgets.get_mut(section_index)
                    {
                        *stored_songs = songs.clone();
                        list.emit(SongListScrollInput::SetSongs(songs));
                    }
                }
            }
        }
    }
}

impl Home {
    fn refresh_home_block_colors(&self) {
        let mut css = String::new();
        for (section_index, section) in self.sections.iter().enumerate() {
            for (card_index, block) in section.blocks.iter().enumerate() {
                if !block.color.is_empty() {
                    let _ = writeln!(
                        css,
                        ".hb-color-{section_index}-{card_index} {{ background-color: {}; }}",
                        block.color
                    );
                }
            }
        }
        self.color_css_provider.load_from_string(&css);
    }
}
