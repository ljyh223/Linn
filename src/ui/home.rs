use futures::stream::{self, StreamExt};
use log::trace;
use relm4::factory::FactoryVecDeque;
use relm4::gtk::{Adjustment, prelude::*};
use relm4::{ComponentParts, ComponentSender, gtk, prelude::*};
use tokio_util::sync::CancellationToken;

use super::components::home_block_card::{HomeBlockCard, HomeBlockCardInit, HomeBlockCardOutput};
use super::components::image::image_manager::ImageManager;
use super::components::playlist_card::{
    BoxPlaylistCard, PlaylistCard, PlaylistCardInit, PlaylistCardOutput,
};
use super::components::scrollable_row::ScrollableRow;
use crate::api::{
    get_home_block, get_playlist_detail, get_recommend_playlist, get_song_detail, HomeBlock,
    HomeBlockType, Playlist, PlaylistDetail, Song,
};
use crate::ui::model::PlaylistType;
use crate::utils::utils::{extract_dominant_color, time_greeting};

const RADAR_PLAYLIST_IDS: &[u64] = &[
    3136952023, 8402996200, 5320167908, 5327906368, 5362359247, 5300458264, 5341776086,
];
const CONCURRENCY_LIMIT: usize = 3;

pub struct Home {
    playlist_cards: FactoryVecDeque<PlaylistCard>,
    radar_cards: FactoryVecDeque<BoxPlaylistCard>,
    home_blocks: Vec<HomeBlock>,
    home_block_cards: FactoryVecDeque<HomeBlockCard>,
}

#[derive(Debug)]
pub enum HomeMsg {
    LoadPlaylists,
    LoadRadarPlaylists,
    LoadHomeBlocks,
    CardAction(PlaylistCardOutput),
    RadarCardAction(PlaylistCardOutput),
    HomeBlockCardAction(HomeBlockCardOutput),
}

#[derive(Debug)]
pub enum HomeCmdMsg {
    PlaylistsLoaded(Vec<Playlist>),
    RadarPlaylistsLoaded(Vec<PlaylistDetail>),
    HomeBlocksLoaded(Vec<HomeBlock>),
    QueueSongsLoaded(Vec<Song>),
}

#[derive(Debug)]
pub enum HomeOutput {
    OpenPlaylistDetail(u64),
    OpenDailyRecommend,
    OpenPlaylistType(PlaylistType),
    Playlist(PlaylistType),
    NavigateToArtist(u64),
    PlayDirectTracks(Vec<Song>),
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
                set_spacing: 12,
                set_margin_top: 16,
                set_margin_bottom: 16,
                set_margin_start: 16,
                set_margin_end: 16,

                // ── 推荐块（横向滚动列表） ──
                #[name(home_block_row)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                },

                // ── 雷达歌单（横向滚动列表） ──
                #[name(radar_row)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                },

                // ── 推荐歌单（FlowBox 网格） ──
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Label {
                        set_label: "推荐歌单",
                        add_css_class: "title-3",
                        set_halign: gtk::Align::Start,
                    },

                    #[name(cards_box)]
                    gtk::FlowBox {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_row_spacing: 16,
                        set_column_spacing: 12,
                        set_min_children_per_line: 2,
                        set_max_children_per_line: 6,
                        set_selection_mode: gtk::SelectionMode::None,
                        set_margin_start: 16,
                        set_margin_end: 16,
                    },
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 创建推荐块滚动行
        let home_block_row_widget = ScrollableRow::new(
            &time_greeting(),
            220,
            220,
        );

        // 创建雷达歌单滚动行
        let radar_row_widget = ScrollableRow::new(
            "雷达歌单",
            220,
            220,
        );

        let mut model = Self {
            playlist_cards: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), HomeMsg::CardAction),
            radar_cards: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .forward(sender.input_sender(), HomeMsg::RadarCardAction),
            home_blocks: Vec::new(),
            home_block_cards: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .forward(sender.input_sender(), |msg| {
                    HomeMsg::HomeBlockCardAction(msg)
                }),
        };

        let widgets = view_output!();

        // 将 ScrollableRow 添加到对应的容器
        widgets.home_block_row.append(home_block_row_widget.widget());
        widgets.radar_row.append(radar_row_widget.widget());

        // 重新创建 FactoryVecDeque，使用 ScrollableRow 的内容容器
        model.home_block_cards = FactoryVecDeque::builder()
            .launch(home_block_row_widget.content().clone())
            .forward(sender.input_sender(), |msg| {
                HomeMsg::HomeBlockCardAction(msg)
            });

        model.playlist_cards = FactoryVecDeque::builder()
            .launch(widgets.cards_box.clone())
            .forward(sender.input_sender(), HomeMsg::CardAction);

        model.radar_cards = FactoryVecDeque::builder()
            .launch(radar_row_widget.content().clone())
            .forward(sender.input_sender(), HomeMsg::RadarCardAction);

        sender.input(HomeMsg::LoadHomeBlocks);
        sender.input(HomeMsg::LoadRadarPlaylists);
        sender.input(HomeMsg::LoadPlaylists);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        trace!("Home: {message:?}");
        match message {
            HomeMsg::LoadPlaylists => {
                sender.command(|out, _shutdown| async move {
                    match get_recommend_playlist().await {
                        Ok(playlists) => {
                            let _ = out.send(HomeCmdMsg::PlaylistsLoaded(playlists));
                        }
                        Err(e) => log::error!("加载推荐歌单失败: {e}"),
                    }
                });
            }

            HomeMsg::LoadRadarPlaylists => {
                sender.command(|out, _shutdown| async move {
                    let ids = RADAR_PLAYLIST_IDS.to_vec();
                    let results: Vec<_> =
                        stream::iter(ids.into_iter().enumerate().map(|(i, id)| async move {
                            let result = get_playlist_detail(id).await;
                            (i, result)
                        }))
                        .buffer_unordered(CONCURRENCY_LIMIT)
                        .collect()
                        .await;

                    let mut results = results;
                    results.sort_by_key(|(i, _)| *i);
                    let playlists: Vec<PlaylistDetail> =
                        results.into_iter().filter_map(|(_, r)| r.ok()).collect();

                    let _ = out.send(HomeCmdMsg::RadarPlaylistsLoaded(playlists));
                });
            }

            HomeMsg::LoadHomeBlocks => {
                sender.command(|out, _shutdown| async move {
                    match get_home_block().await {
                        Ok(blocks) => {
                            let mut filtered: Vec<HomeBlock> = Vec::new();
                            for mut block in blocks {
                                match &block.type_ {
                                    HomeBlockType::Fm | HomeBlockType::Unknown => continue,
                                    _ => {}
                                }

                                let cover_url = format!("{}?param=300y300", block.cover);
                                let token = CancellationToken::new();
                                let color =
                                    match ImageManager::global().fetch(cover_url, token).await {
                                        Ok(bytes) => extract_dominant_color(&bytes),
                                        Err(_) => "#333333".to_string(),
                                    };
                                block.color = color;

                                filtered.push(block);
                            }
                            let _ = out.send(HomeCmdMsg::HomeBlocksLoaded(filtered));
                        }
                        Err(e) => log::error!("加载首页推荐块失败: {e}"),
                    }
                });
            }

            HomeMsg::CardAction(action) | HomeMsg::RadarCardAction(action) => match action {
                PlaylistCardOutput::Clicked(id) => {
                    let _ = sender.output(HomeOutput::OpenPlaylistDetail(id));
                }
                PlaylistCardOutput::ClickedPlaylist(playlist_id) => {
                    trace!("点击了歌单play: {playlist_id}");
                    let _ =
                        sender.output(HomeOutput::Playlist(PlaylistType::Playlist(playlist_id)));
                }
            },

            HomeMsg::HomeBlockCardAction(output) => {
                let HomeBlockCardOutput::Clicked(i) = output;
                let Some(block) = self.home_blocks.get(i) else {
                    return;
                };
                match &block.type_ {
                    HomeBlockType::Playlist(id) => {
                        let _ = sender.output(HomeOutput::OpenPlaylistDetail(*id));
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
                                    let _ = out.send(HomeCmdMsg::QueueSongsLoaded(songs));
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            HomeCmdMsg::PlaylistsLoaded(playlists) => {
                let mut guard = self.playlist_cards.guard();
                guard.clear();
                for playlist in playlists {
                    guard.push_back(PlaylistCardInit {
                        id: playlist.id,
                        cover_url: format!("{}?param=300y300", playlist.cover_url),
                        title: playlist.name.clone(),
                        show_play_button: true,
                    });
                }
            }

            HomeCmdMsg::RadarPlaylistsLoaded(playlists) => {
                let mut guard = self.radar_cards.guard();
                guard.clear();
                for detail in playlists {
                    guard.push_back(PlaylistCardInit {
                        id: detail.id,
                        cover_url: format!("{}?param=300y300", detail.cover_url),
                        title: detail.name.clone(),
                        show_play_button: true,
                    });
                }
            }

            HomeCmdMsg::HomeBlocksLoaded(blocks) => {
                self.home_blocks = blocks;
                let mut guard = self.home_block_cards.guard();
                guard.clear();
                for (i, block) in self.home_blocks.iter().enumerate() {
                    guard.push_back(HomeBlockCardInit {
                        index: i,
                        cover_url: format!("{}?param=300y300", block.cover),
                        title: block.title.clone(),
                        subtitle: block.sub_title.clone(),
                        color: block.color.clone(),
                    });
                }
            }

            HomeCmdMsg::QueueSongsLoaded(songs) => {
                let _ = sender.output(HomeOutput::PlayDirectTracks(songs));
            }
        }
    }
}
