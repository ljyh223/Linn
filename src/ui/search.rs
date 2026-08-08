use relm4::gtk::prelude::*;
use relm4::{Component, ComponentParts, ComponentSender, gtk};

use crate::api::{
    Album, Artist, Playlist, SearchSuggest, Song, search_albums, search_artists,
    search_playlists, search_songs, search_suggest,
};
use crate::ui::components::image::AsyncImage;
use crate::ui::components::scrollable_row::ScrollableRow;
use crate::ui::model::PlaylistType;
use crate::ui::route::AppRoute;

#[derive(Debug, Clone)]
pub enum SearchMsg {
    /// 回车提交：综合搜索（单曲 / 歌单 / 歌手 / 专辑）
    Submit(String),
    /// 输入变化：拉取搜索建议
    Suggest(String),
    /// 清空显示
    Clear,
    /// 单曲被点击（建议 / 结果）
    SongClicked(u64),
    /// 歌单被点击
    PlaylistClicked(u64),
    /// 歌手被点击
    ArtistClicked(u64),
    /// 专辑被点击
    AlbumClicked(u64),
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
    song_scroll: ScrollableRow,
    playlist_scroll: ScrollableRow,
    artist_scroll: ScrollableRow,
    album_scroll: ScrollableRow,
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
                    set_margin_start: 24,
                    set_margin_end: 24,

                    #[local_ref]
                    suggest_list -> gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                    },
                },

                add_named[Some("result")] = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_margin_start: 16,
                    set_margin_end: 16,

                    #[name(result_container)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_top: 16,
                        set_margin_bottom: 16,
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
        let mut model = Self {
            songs: Vec::new(),
            suggest_songs: Vec::new(),
            suggest_seq: 0,
            stack: gtk::Stack::default(),
            suggest_list: gtk::ListBox::default(),
            playlist_scroll: ScrollableRow::new("歌单", 220, 220),
            artist_scroll: ScrollableRow::new("歌手", 220, 220),
            album_scroll: ScrollableRow::new("专辑", 220, 220),
            song_scroll: ScrollableRow::new("单曲", 220, 220),
        };

        // view! 里的 #[local_ref] 需要同名局部变量
        let suggest_list = gtk::ListBox::new();
        let widgets = view_output!();

        // 关键：把 view! 中创建的真实 widget 回填到 model
        model.stack = widgets.stack.clone();
        model.suggest_list = widgets.suggest_list.clone();

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
                clear_children(self.song_scroll.content());
                clear_children(self.playlist_scroll.content());
                clear_children(self.artist_scroll.content());
                clear_children(self.album_scroll.content());
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
            SearchMsg::Clear => {
                self.stack.set_visible_child_name("empty");
                clear_list_box(&self.suggest_list);
                clear_children(self.song_scroll.content());
                clear_children(self.playlist_scroll.content());
                clear_children(self.artist_scroll.content());
                clear_children(self.album_scroll.content());
                self.songs.clear();
                self.suggest_songs.clear();
            }
            SearchMsg::SongClicked(id) => {
                eprintln!("SEARCH_DEBUG SongClicked id={}", id);
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
            SearchMsg::PlaylistClicked(id) => {
                let _ = sender.output(SearchOutput::Navigate(AppRoute::PlaylistDetail(
                    PlaylistType::Playlist(id),
                )));
            }
            SearchMsg::ArtistClicked(id) => {
                let _ = sender.output(SearchOutput::Navigate(AppRoute::Artist(id)));
            }
            SearchMsg::AlbumClicked(id) => {
                let _ = sender.output(SearchOutput::Navigate(AppRoute::PlaylistDetail(
                    PlaylistType::Album(id),
                )));
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
SearchCmdMsg::SuggestLoaded(seq, suggest) => {
                if seq != self.suggest_seq {
                    return;
                }
                eprintln!(
                    "SEARCH_DEBUG SuggestLoaded seq={} songs={} artists={} albums={}",
                    seq,
                    suggest.songs.len(),
                    suggest.artists.len(),
                    suggest.albums.len()
                );
                self.suggest_songs = suggest.songs.clone();
                clear_list_box(&self.suggest_list);

                for song in &suggest.songs {
                    let row = build_song_suggest_row(song, &sender);
                    self.suggest_list.append(&row);
                }
                for artist in &suggest.artists {
                    let row = build_named_suggest_row(
                        "avatar-default-symbolic",
                        &artist.name,
                        "歌手",
                        &sender,
                        SearchMsg::ArtistClicked(artist.id),
                    );
                    self.suggest_list.append(&row);
                }
                for album in &suggest.albums {
                    let row = build_named_suggest_row(
                        "media-optical-cd-symbolic",
                        &album.name,
                        "专辑",
                        &sender,
                        SearchMsg::AlbumClicked(album.id),
                    );
                    self.suggest_list.append(&row);
                }
            }
            SearchCmdMsg::SongsLoaded(songs) => {
                self.songs = songs;
                for chunk in self.songs.chunks(3) {
                    let column = build_song_column(chunk.to_vec(), &sender);
                    self.song_scroll.content().append(&column);
                }
            }
            SearchCmdMsg::PlaylistsLoaded(playlists) => {
                for pl in &playlists {
                    let card = build_result_card(
                        &sender,
                        &pl.cover_url,
                        &pl.name,
                        &pl.creator_name,
                        "歌单",
                        SearchMsg::PlaylistClicked(pl.id),
                    );
                    self.playlist_scroll.content().append(&card);
                }
            }
            SearchCmdMsg::ArtistsLoaded(artists) => {
                for artist in &artists {
                    let card = build_result_card(
                        &sender,
                        artist.avatar.as_deref().unwrap_or(""),
                        &artist.name,
                        "歌手",
                        "歌手",
                        SearchMsg::ArtistClicked(artist.id),
                    );
                    self.artist_scroll.content().append(&card);
                }
            }
            SearchCmdMsg::AlbumsLoaded(albums) => {
                for album in &albums {
                    let card = build_result_card(
                        &sender,
                        &album.cover_url,
                        &album.name,
                        "专辑",
                        "专辑",
                        SearchMsg::AlbumClicked(album.id),
                    );
                    self.album_scroll.content().append(&card);
                }
            }
        }
    }
}

// ───────────────────────── 工具函数 ─────────────────────────

/// 清空普通容器的全部子项（Box / ScrolledWindow 等）
fn clear_children(container: &impl IsA<gtk::Widget>) {
    while let Some(child) = container.first_child() {
        child.unparent();
    }
}

/// 清空 ListBox：必须用 ListBox::remove，unparent 会弄乱内部索引导致崩溃
fn clear_list_box(list_box: &gtk::ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
}

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

/// 单曲建议行：图标 + 歌名 - 歌手
fn build_song_suggest_row(song: &Song, sender: &ComponentSender<Search>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();

    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(8)
        .margin_end(8)
        .build();

    hbox.append(
        &gtk::Image::builder()
            .icon_name("audio-x-generic-symbolic")
            .pixel_size(18)
            .build(),
    );

    let artists = song
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    hbox.append(
        &gtk::Label::builder()
            .label(&format!("{} - {}", song.name, artists))
            .halign(gtk::Align::Start)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );

    row.set_child(Some(&hbox));

    make_clickable(&row, sender, SearchMsg::SongClicked(song.id));

    row
}

/// 歌手 / 专辑建议行：图标 + 主标题 + 副标题
fn build_named_suggest_row(
    icon_name: &str,
    title: &str,
    subtitle: &str,
    sender: &ComponentSender<Search>,
    msg: SearchMsg,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();

    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(8)
        .margin_end(8)
        .build();

    hbox.append(
        &gtk::Image::builder()
            .icon_name(icon_name)
            .pixel_size(18)
            .build(),
    );

    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .build();

    vbox.append(
        &gtk::Label::builder()
            .label(title)
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    vbox.append(
        &gtk::Label::builder()
            .label(subtitle)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    hbox.append(&vbox);

    row.set_child(Some(&hbox));
    make_clickable(&row, sender, msg);

    row
}

/// 单曲结果列：一列 3 首，加大列间距与行尺寸
fn build_song_column(songs: Vec<Song>, sender: &ComponentSender<Search>) -> gtk::Box {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .build();

    for song in songs {
        let row = build_song_row(&song, sender);
        column.append(&row);
    }

    column
}

/// 单曲结果行：封面 + 歌名 + 歌手（无序号）
fn build_song_row(song: &Song, sender: &ComponentSender<Search>) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .width_request(200)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    let cover = AsyncImage::new();
    cover.set_width_request(52);
    cover.set_height_request(52);
    cover.set_corner_radius(6.0);
    cover.set_placeholder_icon("audio-x-generic-symbolic");
    cover.set_url(format!("{}?param=100y100", song.cover_url));
    row.append(&cover);

    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(3)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();

    vbox.append(
        &gtk::Label::builder()
            .label(&song.name)
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(16)
            .build(),
    );

    let artists = song
        .artists
        .iter()
        .take(2)
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    vbox.append(
        &gtk::Label::builder()
            .label(&artists)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label", "caption"])
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(16)
            .build(),
    );

    row.append(&vbox);

    make_clickable(&row, sender, SearchMsg::SongClicked(song.id));

    row
}

/// 歌单 / 歌手 / 专辑卡片：封面 + 名称
fn build_result_card(
    sender: &ComponentSender<Search>,
    cover_url: &str,
    title: &str,
    subtitle: &str,
    badge: &str,
    msg: SearchMsg,
) -> gtk::Box {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .width_request(160)
        .build();

    let cover = AsyncImage::new();
    cover.set_width_request(160);
    cover.set_height_request(160);
    cover.set_corner_radius(8.0);
    cover.set_placeholder_icon("image-missing-symbolic");
    cover.set_url(cover_url.to_string());
    card.append(&cover);

    card.append(
        &gtk::Label::builder()
            .label(&format!("{badge} · {title}"))
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(16)
            .build(),
    );
    card.append(
        &gtk::Label::builder()
            .label(subtitle)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label", "caption"])
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(16)
            .build(),
    );

    make_clickable(&card, sender, msg);

    card
}

/// 为任意 widget 添加左键点击手势（比 ListBoxRow activate 更可靠）
fn make_clickable(
    widget: &impl IsA<gtk::Widget>,
    sender: &ComponentSender<Search>,
    msg: SearchMsg,
) {
    let sender = sender.clone();
    let gesture = gtk::GestureClick::new();
    gesture.set_button(1);
    gesture.connect_released(move |_, _, _, _| {
        sender.input(msg.clone());
    });
    widget.add_controller(gesture);
}