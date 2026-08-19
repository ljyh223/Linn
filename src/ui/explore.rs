//! Explore 发现页 —— 四区四形态
//!
//! 排行榜（网易云式并列榜卡） / 新歌速递（SongRow 分列横滚） / 新碟上架（横滚 CD 圆卡） / 最新 MV（横滚宽卡）。

use relm4::factory::FactoryVecDeque;
use relm4::gtk::prelude::*;
use relm4::prelude::*;

use crate::api::{
    Mv, Playlist, Song, get_album_detail, get_new_albums, get_new_mvs, get_new_songs, get_toplist,
    get_toplist_songs,
};
use crate::ui::components::album_disc::{AlbumDisc, AlbumDiscInit, AlbumDiscOutput};
use crate::ui::components::mv_card::{BoxMvCard, MvCardInit, MvCardOutput};
use crate::ui::components::scrollable_row::ScrollableRow;
use crate::ui::components::song_list::{
    SongListScroll, SongListScrollInit, SongListScrollInput, SongListScrollOutput,
};
use crate::ui::components::toplist_card::{
    BoardCardData, ToplistBoard, ToplistBoardInit, ToplistBoardInput, ToplistBoardOutput,
};
use crate::ui::components::track_row::{TrackRow, TrackRowInit, TrackRowOutput};

pub struct Explore {
    // 排行榜:并列榜卡
    toplist_board: Option<Controller<ToplistBoard>>,
    toplists_slot: gtk::Box,
    /// 已加载的榜单（点击时反查名称）
    toplists: Vec<Playlist>,

    // 新歌速递:单曲分列横滚（与搜索页共用组件）
    song_list: Controller<SongListScroll>,
    songs: Vec<Song>,

    // 新碟上架：横滚 CD 圆卡
    album_discs: FactoryVecDeque<AlbumDisc>,
    _album_row: Controller<ScrollableRow>,

    // 最新 MV：横滚宽卡
    mv_cards: FactoryVecDeque<BoxMvCard>,
    _mv_row: Controller<ScrollableRow>,

    // 榜单详情子视图
    stack: gtk::Stack,
    ranking_title: gtk::Label,
    ranking_list: gtk::ListBox,
    ranking_tracks: FactoryVecDeque<TrackRow>,
    ranking_songs: Vec<Song>,
}

#[derive(Debug)]
pub enum ExploreMsg {
    /// 榜单卡点击 → 进入榜单详情
    BoardClicked(ToplistBoardOutput),
    /// 新歌速递行播放
    SongClicked(u64),
    /// 新碟卡点击（异步取专辑曲目后播放）
    AlbumDiscClicked(AlbumDiscOutput),
    MvClicked(MvCardOutput),
    CloseRanking,
    /// 榜单歌曲行点击播放
    RankingPlayClicked(u64),
    RankingNoop,
}

#[derive(Debug)]
pub enum ExploreCmdMsg {
    ToplistsLoaded(Vec<Playlist>),
    /// 第 index 个榜卡的前三首歌
    BigSongsLoaded {
        index: usize,
        songs: Vec<Song>,
    },
    NewSongsLoaded(Vec<Song>),
    NewAlbumsLoaded(Vec<Playlist>),
    NewMvsLoaded(Vec<Mv>),
    RankingLoaded {
        songs: Vec<Song>,
    },
    AlbumTracksLoaded(Vec<Song>),
    LoadFailed,
}

#[derive(Debug)]
pub enum ExploreOutput {
    /// 播放一组歌曲（榜单/专辑）
    PlayTracks(Vec<Song>, usize),
    /// 打开 MV 播放页
    OpenMv(u64),
}

#[relm4::component(pub)]
impl Component for Explore {
    type Init = ();
    type Input = ExploreMsg;
    type Output = ExploreOutput;
    type CommandOutput = ExploreCmdMsg;

    view! {
            #[root]
            gtk::Box {
                #[name(stack)]
                gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,

                add_named[Some("main")] = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_margin_top: 8,
                    set_margin_bottom: 16,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,

                        // ── 1. 排行榜：并列榜卡（含前三首歌） ──
                        #[name(toplists_slot)]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                        },

                        // ── 2. 新歌速递：SongRow 分列横滚 ──
                        #[name(song_slot)]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                        },

                        // ── 3. 新碟上架：横滚 CD 圆卡 ──
                        #[name(album_slot)]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                        },

                        // ── 4. 最新 MV：横滚宽卡 ──
                        #[name(mv_slot)]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                        },
                    }
                },

                add_named[Some("ranking")] = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_margin_top: 8,
                    set_margin_bottom: 16,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Button {
                            set_icon_name: "go-previous-symbolic",
                            add_css_class: "circular",
                            add_css_class: "flat",
                            set_tooltip_text: Some("返回发现页"),
                            connect_clicked => ExploreMsg::CloseRanking,
                        },

                        #[name(ranking_title)]
                        gtk::Label {
                            set_label: "排行榜",
                            set_halign: gtk::Align::Start,
                            set_hexpand: true,
                            add_css_class: "title-3",
                        },
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,

                        #[name(ranking_list)]
                        gtk::ListBox {
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            set_show_separators: true,
                        },
                    },
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 新歌速递：与搜索页共用的单曲分列横滚
        let song_list = SongListScroll::builder()
            .launch(SongListScrollInit::new("新歌速递", 230, 230))
            .forward(sender.input_sender(), |out| match out {
                SongListScrollOutput::Clicked(id) => ExploreMsg::SongClicked(id),
            });
        // 新碟 → 横滚 CD 圆卡行；最新 MV → 横滚宽卡行
        let album_row = ScrollableRow::new("新碟上架", 230, 235);
        let mv_row = ScrollableRow::new("最新 MV", 165, 170);
        let album_box = album_row.model().content_box();
        let mv_box = mv_row.model().content_box();
        let ranking_title = gtk::Label::new(Some("排行榜"));

        let mut model = Self {
            toplist_board: None,
            toplists_slot: gtk::Box::new(gtk::Orientation::Vertical, 0),
            toplists: Vec::new(),
            song_list,
            songs: Vec::new(),
            album_discs: FactoryVecDeque::builder()
                .launch(album_box)
                .forward(sender.input_sender(), ExploreMsg::AlbumDiscClicked),
            _album_row: album_row,
            mv_cards: FactoryVecDeque::builder()
                .launch(mv_box)
                .forward(sender.input_sender(), ExploreMsg::MvClicked),
            _mv_row: mv_row,
            stack: gtk::Stack::default(),
            ranking_title,
            ranking_list: gtk::ListBox::new(),
            ranking_tracks: FactoryVecDeque::builder()
                .launch(gtk::ListBox::new())
                .forward(sender.input_sender(), |msg| match msg {
                    TrackRowOutput::PlayClicked(id) => ExploreMsg::RankingPlayClicked(id),
                    TrackRowOutput::MoreClicked(_) => ExploreMsg::RankingNoop,
                }),
            ranking_songs: Vec::new(),
        };

        let widgets = view_output!();

        // 回填 widget/工厂
        model.stack = widgets.stack.clone();
        model.ranking_title = widgets.ranking_title.clone();
        model.ranking_list = widgets.ranking_list.clone();
        model.toplists_slot = widgets.toplists_slot.clone();
        model.stack.set_visible_child_name("main");
        widgets.song_slot.append(model.song_list.widget());
        widgets.album_slot.append(model._album_row.widget());
        widgets.mv_slot.append(model._mv_row.widget());

        // 榜单详情工厂挂到真实 ListBox
        model.ranking_tracks = FactoryVecDeque::builder()
            .launch(model.ranking_list.clone())
            .forward(sender.input_sender(), |msg| match msg {
                TrackRowOutput::PlayClicked(id) => ExploreMsg::RankingPlayClicked(id),
                TrackRowOutput::MoreClicked(_) => ExploreMsg::RankingNoop,
            });

        // 并行加载 4 块内容
        sender.command(|out, _shutdown| async move {
            match get_toplist().await {
                Ok(list) => {
                    let _ = out.send(ExploreCmdMsg::ToplistsLoaded(list));
                }
                Err(_) => {
                    let _ = out.send(ExploreCmdMsg::LoadFailed);
                }
            }
        });
        sender.command(|out, _shutdown| async move {
            match get_new_songs().await {
                Ok(songs) => {
                    let _ = out.send(ExploreCmdMsg::NewSongsLoaded(songs));
                }
                Err(_) => {
                    let _ = out.send(ExploreCmdMsg::LoadFailed);
                }
            }
        });
        sender.command(|out, _shutdown| async move {
            match get_new_albums().await {
                Ok(albums) => {
                    let _ = out.send(ExploreCmdMsg::NewAlbumsLoaded(albums));
                }
                Err(_) => {
                    let _ = out.send(ExploreCmdMsg::LoadFailed);
                }
            }
        });
        sender.command(|out, _shutdown| async move {
            match get_new_mvs().await {
                Ok(mvs) => {
                    let _ = out.send(ExploreCmdMsg::NewMvsLoaded(mvs));
                }
                Err(_) => {
                    let _ = out.send(ExploreCmdMsg::LoadFailed);
                }
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ExploreMsg::BoardClicked(ToplistBoardOutput::Clicked(id)) => {
                let name = self
                    .toplists
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "排行榜".to_string());
                self.ranking_title.set_label(&name);
                self.stack.set_visible_child_name("ranking");
                sender.command(move |out, _shutdown| async move {
                    match get_toplist_songs(id).await {
                        Ok(songs) => {
                            let _ = out.send(ExploreCmdMsg::RankingLoaded { songs });
                        }
                        Err(_) => {
                            let _ = out.send(ExploreCmdMsg::LoadFailed);
                        }
                    }
                });
            }
            ExploreMsg::SongClicked(id) => {
                let song = self.songs.iter().find(|s| s.id == id).cloned();
                if let Some(song) = song {
                    let _ = sender.output(ExploreOutput::PlayTracks(vec![song], 0));
                }
            }
            ExploreMsg::AlbumDiscClicked(output) => match output {
                AlbumDiscOutput::Clicked(id) => {
                    sender.command(move |out, _shutdown| async move {
                        match get_album_detail(id).await {
                            Ok(detail) => {
                                let _ = out.send(ExploreCmdMsg::AlbumTracksLoaded(detail.tracks));
                            }
                            Err(e) => {
                                log::error!("获取专辑曲目失败: {e}");
                                let _ = out.send(ExploreCmdMsg::LoadFailed);
                            }
                        }
                    });
                }
            },
            ExploreMsg::MvClicked(output) => match output {
                MvCardOutput::Clicked(id) => {
                    let _ = sender.output(ExploreOutput::OpenMv(id));
                }
            },
            ExploreMsg::CloseRanking => {
                self.stack.set_visible_child_name("main");
            }
            ExploreMsg::RankingPlayClicked(id) => {
                let index = self
                    .ranking_songs
                    .iter()
                    .position(|s| s.id == id)
                    .unwrap_or(0);
                let songs = self.ranking_songs.clone();
                let _ = sender.output(ExploreOutput::PlayTracks(songs, index));
            }
            ExploreMsg::RankingNoop => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ExploreCmdMsg::ToplistsLoaded(list) => {
                self.toplists = list;
                if self.toplists.is_empty() {
                    return;
                }
                let cards: Vec<BoardCardData> = self
                    .toplists
                    .iter()
                    .take(8)
                    .map(BoardCardData::from_playlist)
                    .collect();
                let board = ToplistBoard::builder()
                    .launch(ToplistBoardInit { cards })
                    .forward(sender.input_sender(), ExploreMsg::BoardClicked);
                self.toplists_slot.append(board.widget());
                self.toplist_board = Some(board);
                for (index, card) in self.toplists.iter().take(8).enumerate() {
                    let id = card.id;
                    sender.command(move |out, _shutdown| async move {
                        match get_toplist_songs(id).await {
                            Ok(songs) => {
                                let _ = out.send(ExploreCmdMsg::BigSongsLoaded { index, songs });
                            }
                            Err(_) => {
                                let _ = out.send(ExploreCmdMsg::LoadFailed);
                            }
                        }
                    });
                }
            }
            ExploreCmdMsg::BigSongsLoaded { index, songs } => {
                if let Some(board) = &self.toplist_board {
                    board.emit(ToplistBoardInput::SetSongs { index, songs });
                }
            }
            ExploreCmdMsg::NewSongsLoaded(songs) => {
                self.songs = songs.clone();
                self.song_list.emit(SongListScrollInput::SetSongs(songs));
            }
            ExploreCmdMsg::NewAlbumsLoaded(albums) => {
                let mut guard = self.album_discs.guard();
                guard.clear();
                for a in albums.iter().take(12) {
                    guard.push_back(AlbumDiscInit::from_playlist(a));
                }
            }
            ExploreCmdMsg::NewMvsLoaded(mvs) => {
                let mut guard = self.mv_cards.guard();
                guard.clear();
                for m in mvs.iter().take(12) {
                    guard.push_back(MvCardInit::from_play_count(m));
                }
            }
            ExploreCmdMsg::RankingLoaded { songs } => {
                self.ranking_songs = songs;
                let mut guard = self.ranking_tracks.guard();
                guard.clear();
                for (i, s) in self.ranking_songs.iter().take(50).enumerate() {
                    guard.push_back(TrackRowInit {
                        track: s.clone(),
                        index: i,
                    });
                }
            }
            ExploreCmdMsg::AlbumTracksLoaded(tracks) => {
                let _ = sender.output(ExploreOutput::PlayTracks(tracks, 0));
            }
            ExploreCmdMsg::LoadFailed => {}
        }
    }
}
