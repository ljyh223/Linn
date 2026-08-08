pub mod components;

use relm4::factory::FactoryVecDeque;
use relm4::gtk::prelude::*;
use relm4::{Component, ComponentParts, ComponentSender, Controller, gtk};
use relm4::prelude::*;

use crate::api::{
    Album, Artist, Playlist, SearchSuggest, Song, search_albums, search_artists,
    search_playlists, search_songs, search_suggest,
};
use crate::ui::components::artist_card::{ArtistCard, ArtistCardInit, ArtistCardOutput};
use crate::ui::components::playlist_card::{
    BoxPlaylistCard, PlaylistCardInit, PlaylistCardOutput,
};
use crate::ui::components::scrollable_row::ScrollableRow;
use crate::ui::model::PlaylistType;
use crate::ui::route::AppRoute;

use components::song_row::{SongRow, SongRowInit, SongRowOutput};
use components::suggest_row::{
    SuggestEntityInit, SuggestEntityRow, SuggestRowOutput, SuggestSongInit, SuggestSongRow,
};

#[derive(Debug, Clone)]
pub enum SearchMsg {
    /// 回车提交：综合搜索（单曲 / 歌单 / 歌手 / 专辑）
    Submit(String),
    /// 输入变化：拉取搜索建议
    Suggest(String),
    /// 单曲被点击（建议 / 结果）
    SongClicked(u64),
    /// 建议里的歌手被点击
    ArtistClicked(u64),
    /// 建议里的专辑被点击
    AlbumClicked(u64),
    /// 结果区的歌单卡片被点击（包含播放按钮）
    PlaylistCardClicked(PlaylistCardOutput),
    /// 结果区的专辑卡片被点击
    AlbumCardClicked(PlaylistCardOutput),
    /// 结果区的歌手圆形卡片被点击
    ArtistCardClicked(ArtistCardOutput),
}

#[derive(Debug)]
pub enum SearchCmdMsg {
    SuggestLoaded(u64, SearchSuggest),
    SongsLoaded(Vec<Song>),
    PlaylistsLoaded(Vec<Playlist>),
    ArtistsLoaded(Vec<Artist>),
    AlbumsLoaded(Vec<Album>),
}

#[derive(Debug)]
pub enum SearchOutput {
    PlaySong(Song),
    Navigate(AppRoute),
}

pub struct Search {
    songs: Vec<Song>,
    suggest_songs: Vec<Song>,
    /// 建议请求序号，丢弃乱序返回的旧结果
    suggest_seq: u64,
    stack: gtk::Stack,
    suggest_list: gtk::ListBox,
    suggest_song_rows: FactoryVecDeque<SuggestSongRow>,
    suggest_artist_rows: FactoryVecDeque<SuggestEntityRow>,
    suggest_album_rows: FactoryVecDeque<SuggestEntityRow>,
    song_scroll: Controller<ScrollableRow>,
    playlist_scroll: Controller<ScrollableRow>,
    artist_scroll: Controller<ScrollableRow>,
    album_scroll: Controller<ScrollableRow>,
    playlist_cards: FactoryVecDeque<BoxPlaylistCard>,
    album_cards: FactoryVecDeque<BoxPlaylistCard>,
    artist_cards: FactoryVecDeque<ArtistCard>,
    /// 单曲结果列：每列一个独立工厂（保持 3 首一列的原布局）
    song_columns: Vec<(gtk::Box, FactoryVecDeque<SongRow>)>,
}

#[relm4::component(pub)]
impl Component for Search {
    type Init = ();
    type Input = SearchMsg;
    type Output = SearchOutput;
    type CommandOutput = SearchCmdMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,

            #[name(stack)]
            gtk::Stack {
                set_vexpand: true,
                set_transition_type: gtk::StackTransitionType::Crossfade,

                add_named[Some("empty")] = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::Label {
                        set_label: "输入关键词，搜索歌曲 / 歌单 / 歌手 / 专辑",
                        add_css_class: "dim-label",
                        set_wrap: true,
                        set_justify: gtk::Justification::Center,
                    },
                },

                add_named[Some("suggest")] = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[local_ref]
                    suggest_list -> gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        set_margin_start: 16,
                        set_margin_end: 16,
                    },
                },

                add_named[Some("result")] = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[name(result_container)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_top: 16,
                        set_margin_bottom: 16,
                        set_margin_start: 16,
                        set_margin_end: 16,
                    },
                },
            },
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let song_row = ScrollableRow::new("单曲", 220, 220);
        let playlist_row = ScrollableRow::new("歌单", 220, 220);
        let artist_row = ScrollableRow::new("歌手", 220, 220);
        let album_row = ScrollableRow::new("专辑", 220, 220);

        let playlist_cards = FactoryVecDeque::builder()
            .launch(playlist_row.widgets().content_box.clone())
            .forward(sender.input_sender(), SearchMsg::PlaylistCardClicked);
        let album_cards = FactoryVecDeque::builder()
            .launch(album_row.widgets().content_box.clone())
            .forward(sender.input_sender(), SearchMsg::AlbumCardClicked);
        let artist_cards = FactoryVecDeque::builder()
            .launch(artist_row.widgets().content_box.clone())
            .forward(sender.input_sender(), SearchMsg::ArtistCardClicked);

        let mut model = Self {
            songs: Vec::new(),
            suggest_songs: Vec::new(),
            suggest_seq: 0,
            stack: gtk::Stack::default(),
            suggest_list: gtk::ListBox::default(),
            suggest_song_rows: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |out| match out {
                    SuggestRowOutput::Clicked(id) => SearchMsg::SongClicked(id),
                }),
            suggest_artist_rows: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |out| match out {
                    SuggestRowOutput::Clicked(id) => SearchMsg::ArtistClicked(id),
                }),
            suggest_album_rows: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |out| match out {
                    SuggestRowOutput::Clicked(id) => SearchMsg::AlbumClicked(id),
                }),
            song_scroll: song_row,
            playlist_scroll: playlist_row,
            artist_scroll: artist_row,
            album_scroll: album_row,
            playlist_cards,
            album_cards,
            artist_cards,
            song_columns: Vec::new(),
        };

        // view! 里的 #[local_ref] 需要同名局部变量
        let suggest_list = gtk::ListBox::new();
        let widgets = view_output!();

        // 关键：把 view! 中创建的真实 widget 回填到 model
        model.stack = widgets.stack.clone();
        model.suggest_list = widgets.suggest_list.clone();

        // 建议列表工厂挂到真实的 ListBox 上
        model.suggest_song_rows = FactoryVecDeque::builder()
            .launch(widgets.suggest_list.clone())
            .forward(sender.input_sender(), |out| match out {
                SuggestRowOutput::Clicked(id) => SearchMsg::SongClicked(id),
            });
        model.suggest_artist_rows = FactoryVecDeque::builder()
            .launch(widgets.suggest_list.clone())
            .forward(sender.input_sender(), |out| match out {
                SuggestRowOutput::Clicked(id) => SearchMsg::ArtistClicked(id),
            });
        model.suggest_album_rows = FactoryVecDeque::builder()
            .launch(widgets.suggest_list.clone())
            .forward(sender.input_sender(), |out| match out {
                SuggestRowOutput::Clicked(id) => SearchMsg::AlbumClicked(id),
            });

        // 把四个滚动行挂到结果容器（单曲用纵向列表内分组）
        widgets
            .result_container
            .append(model.song_scroll.widget());
        widgets
            .result_container
            .append(model.playlist_scroll.widget());
        widgets.result_container.append(model.artist_scroll.widget());
        widgets.result_container.append(model.album_scroll.widget());

        let _ = sender;

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SearchMsg::Submit(query) => {
                self.stack.set_visible_child_name("result");
                self.clear_results();
                self.songs.clear();
                self.suggest_songs.clear();

                spawn_search_commands(&sender, &query);
            }
            SearchMsg::Suggest(query) => {
                if query.trim().is_empty() {
                    self.stack.set_visible_child_name("empty");
                    return;
                }
                self.stack.set_visible_child_name("suggest");

                self.suggest_seq += 1;
                let seq = self.suggest_seq;
                let sender = sender.clone();
                let query = query.clone();
                sender.command(move |out, _| async move {
                    match search_suggest(&query).await {
                        Ok(suggest) => {
                            let _ = out.send(SearchCmdMsg::SuggestLoaded(seq, suggest));
                        }
                        Err(err) => {
                            log::error!("获取搜索建议失败: {}", err);
                            let _ = out
                                .send(SearchCmdMsg::SuggestLoaded(seq, SearchSuggest::default()));
                        }
                    }
                });
            }
            SearchMsg::SongClicked(id) => {
                let song = self
                    .songs
                    .iter()
                    .chain(self.suggest_songs.iter())
                    .find(|song| song.id == id)
                    .cloned();
                if let Some(song) = song {
                    let _ = sender.output(SearchOutput::PlaySong(song));
                }
            }
            SearchMsg::ArtistClicked(id) => {
                let _ = sender.output(SearchOutput::Navigate(AppRoute::Artist(id)));
            }
            SearchMsg::AlbumClicked(id) => {
                let _ = sender.output(SearchOutput::Navigate(AppRoute::PlaylistDetail(
                    PlaylistType::Album(id),
                )));
            }
            SearchMsg::PlaylistCardClicked(output) => match output {
                PlaylistCardOutput::Clicked(id) | PlaylistCardOutput::ClickedPlaylist(id) => {
                    let _ = sender.output(SearchOutput::Navigate(AppRoute::PlaylistDetail(
                        PlaylistType::Playlist(id),
                    )));
                }
            },
            SearchMsg::AlbumCardClicked(output) => match output {
                PlaylistCardOutput::Clicked(id) | PlaylistCardOutput::ClickedPlaylist(id) => {
                    let _ = sender.output(SearchOutput::Navigate(AppRoute::PlaylistDetail(
                        PlaylistType::Album(id),
                    )));
                }
            },
            SearchMsg::ArtistCardClicked(output) => match output {
                ArtistCardOutput::Clicked(id) => {
                    let _ = sender.output(SearchOutput::Navigate(AppRoute::Artist(id)));
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            SearchCmdMsg::SuggestLoaded(seq, suggest) => {
                if seq != self.suggest_seq {
                    return;
                }
                self.suggest_songs = suggest.songs.clone();
                self.suggest_song_rows.guard().clear();
                self.suggest_artist_rows.guard().clear();
                self.suggest_album_rows.guard().clear();

                for song in &suggest.songs {
                    self.suggest_song_rows.guard().push_back(SuggestSongInit {
                        id: song.id,
                        name: song.name.clone(),
                        artists: song
                            .artists
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    });
                }
                for artist in &suggest.artists {
                    self.suggest_artist_rows.guard().push_back(SuggestEntityInit {
                        id: artist.id,
                        icon_name: "avatar-default-symbolic".to_string(),
                        title: artist.name.clone(),
                        subtitle: "歌手".to_string(),
                    });
                }
                for album in &suggest.albums {
                    self.suggest_album_rows.guard().push_back(SuggestEntityInit {
                        id: album.id,
                        icon_name: "media-optical-cd-symbolic".to_string(),
                        title: album.name.clone(),
                        subtitle: "专辑".to_string(),
                    });
                }
            }
            SearchCmdMsg::SongsLoaded(songs) => {
                self.songs = songs;
                self.clear_song_columns();
                for chunk in self.songs.chunks(3) {
                    let column = gtk::Box::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .spacing(16)
                        .build();

                    let mut factory = FactoryVecDeque::builder()
                        .launch(column.clone())
                        .forward(sender.input_sender(), |out| match out {
                            SongRowOutput::Clicked(id) => SearchMsg::SongClicked(id),
                        });
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

                    self.song_scroll.widgets().content_box.append(&column);
                    self.song_columns.push((column, factory));
                }
            }
            SearchCmdMsg::PlaylistsLoaded(playlists) => {
                let mut guard = self.playlist_cards.guard();
                guard.clear();
                for pl in &playlists {
                    guard.push_back(
                        PlaylistCardInit::new(
                            pl.id,
                            format!("{}?param=300y300", pl.cover_url),
                            pl.name.clone(),
                        )
                        .with_subtitle(pl.creator_name.clone()),
                    );
                }
            }
            SearchCmdMsg::ArtistsLoaded(artists) => {
                let mut guard = self.artist_cards.guard();
                guard.clear();
                for artist in &artists {
                    guard.push_back(ArtistCardInit {
                        id: artist.id,
                        avatar_url: format!(
                            "{}?param=300y300",
                            artist.avatar.as_deref().unwrap_or_default()
                        ),
                        name: artist.name.clone(),
                    });
                }
            }
            SearchCmdMsg::AlbumsLoaded(albums) => {
                let mut guard = self.album_cards.guard();
                guard.clear();
                for album in &albums {
                    guard.push_back(PlaylistCardInit {
                        id: album.id,
                        cover_url: format!("{}?param=300y300", album.cover_url),
                        title: album.name.clone(),
                        subtitle: None,
                        show_play_button: false,
                    });
                }
            }
        }
    }
}

impl Search {
    /// 清空结果区：卡片工厂直接 clear，单曲列卸载并丢弃
    fn clear_results(&mut self) {
        self.playlist_cards.guard().clear();
        self.album_cards.guard().clear();
        self.artist_cards.guard().clear();
        self.suggest_song_rows.guard().clear();
        self.suggest_artist_rows.guard().clear();
        self.suggest_album_rows.guard().clear();
        self.clear_song_columns();
    }

    /// 卸载所有单曲结果列（列容器从滚动内容中移除，行工厂 drop 自动清理子控件）
    fn clear_song_columns(&mut self) {
        for (column, factory) in self.song_columns.drain(..) {
            column.unparent();
            drop(factory);
        }
    }
}

// ───────────────────────── 工具函数 ─────────────────────────

/// 并行发起四类搜索请求
fn spawn_search_commands(sender: &ComponentSender<Search>, query: &str) {
    let q_songs = query.to_string();
    sender.command(move |out, _| async move {
        match search_songs(&q_songs, 100, 0).await {
            Ok(result) => {
                let _ = out.send(SearchCmdMsg::SongsLoaded(result.items));
            }
            Err(err) => log::error!("搜索单曲失败: {}", err),
        }
    });

    let q_playlists = query.to_string();
    sender.command(move |out, _| async move {
        match search_playlists(&q_playlists, 20, 0).await {
            Ok(result) => {
                let _ = out.send(SearchCmdMsg::PlaylistsLoaded(result.items));
            }
            Err(err) => log::error!("搜索歌单失败: {}", err),
        }
    });

    let q_artists = query.to_string();
    sender.command(move |out, _| async move {
        match search_artists(&q_artists, 20, 0).await {
            Ok(result) => {
                let _ = out.send(SearchCmdMsg::ArtistsLoaded(result.items));
            }
            Err(err) => log::error!("搜索歌手失败: {}", err),
        }
    });

    let q_albums = query.to_string();
    sender.command(move |out, _| async move {
        match search_albums(&q_albums, 20, 0).await {
            Ok(result) => {
                let _ = out.send(SearchCmdMsg::AlbumsLoaded(result.items));
            }
            Err(err) => log::error!("搜索专辑失败: {}", err),
        }
    });
}