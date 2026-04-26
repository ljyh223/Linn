use relm4::Sender;
use std::sync::Arc;

use crate::{
    api::{
        Playlist, PlaylistDetail, Song, SoundQuality, get_album_detail, get_playlist_detail,
        get_recommend_song, get_song_detail, get_song_url, is_like_song, like_song,
    },
    player::{
        engine::{GstEngine, GstEvent},
        messages::{
            InternalEvent, MprisCommand, MprisUpdate, PlayMode, PlaybackState, PlayerCommand, PlayerEvent,
        },
        mpris,
        queue::{QueueItem, QueueManager},
    },
    ui::{
        model::{PlaySource, PlaylistType},
        window::WindowMsg,
    },
};

fn async_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("Failed to create async runtime")
    })
}

pub struct PlayerFacade {
    engine: GstEngine,
    queue: QueueManager,
    is_waiting_to_play: bool,

    cmd_rx: flume::Receiver<PlayerCommand>,
    internal_rx: flume::Receiver<InternalEvent>,
    internal_tx: flume::Sender<InternalEvent>,

    event_tx: Sender<WindowMsg>,

    mpris_tx: flume::Sender<MprisUpdate>,
    mpris_rx: flume::Receiver<MprisCommand>,
}

impl PlayerFacade {
    pub fn start(event_tx: Sender<WindowMsg>) -> flume::Sender<PlayerCommand> {
        let (cmd_tx, cmd_rx) = flume::unbounded::<PlayerCommand>();
        let (internal_tx, internal_rx) = flume::unbounded::<InternalEvent>();
        let (mpris_update_tx, mpris_update_rx) = flume::unbounded::<MprisUpdate>();
        let (mpris_cmd_tx, mpris_cmd_rx) = flume::unbounded::<MprisCommand>();

        mpris::start_mpris(mpris_update_rx, mpris_cmd_tx);
        let _ = async_runtime();

        std::thread::spawn(move || {
            PlayerFacade {
                engine: GstEngine::new(),
                queue: QueueManager::new(),
                is_waiting_to_play: false,
                cmd_rx,
                internal_rx,
                internal_tx,
                event_tx,
                mpris_tx: mpris_update_tx,
                mpris_rx: mpris_cmd_rx,
            }
            .run();
        });

        cmd_tx
    }

    fn run(&mut self) {
        loop {
            // 1. 处理来自 UI 的指令
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                self.handle_cmd(cmd);
            }

            // 2. 处理来自 MPRIS 的指令（统一转成 PlayerCommand，复用同一段逻辑）
            while let Ok(cmd) = self.mpris_rx.try_recv() {
                match cmd {
                    MprisCommand::Play => {
                        self.engine.resume();
                    }
                    MprisCommand::Pause => {
                        self.engine.pause();
                    }
                    MprisCommand::Next => self.handle_cmd(PlayerCommand::Next),
                    MprisCommand::Previous => self.handle_cmd(PlayerCommand::Previous),
                    MprisCommand::Seek(ms) => self.handle_cmd(PlayerCommand::Seek(ms)),
                }
            }

            // 3. 处理异步工作线程的内部回调
            while let Ok(ev) = self.internal_rx.try_recv() {
                self.handle_internal(ev);
            }

            // 4. 轮询 GStreamer 消息（最多阻塞 10ms）
            if let Some(ev) = self.engine.poll() {
                self.handle_gst(ev);
            }
        }
    }

    fn handle_cmd(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::Play {
                source,
                start_index,
            } => {
                match source {
                    PlaySource::LazyQueue {
                        tracks,
                        track_ids,
                        playlist,
                    } => {
                        self.queue
                            .load(track_ids, tracks.clone(), playlist.clone(), start_index);
                        self.emit(PlayerEvent::SetQueue {
                            tracks,
                            playlist: Arc::new(playlist),
                            start_index,
                        });
                    }
                    PlaySource::ById(playlist_type) => match playlist_type {
                        PlaylistType::Playlist(id) => self.spawn_playlist_fetch(id),
                        PlaylistType::Album(id) => self.spawn_album_fetch(id),
                        PlaylistType::DailyRecommend => self.spwa_daily_recommend_fetch(),
                    },
                    PlaySource::DirectTracks(songs) => {}
                    PlaySource::ArtistQueue {
                        songs,
                        artist_name,
                        artist_id,
                    } => {
                        self.queue.load(
                            Arc::new(songs.clone().iter().map(|s| s.id).collect()),
                            songs.clone(),
                            Playlist::from_artist_hot_songs(
                                songs.first().unwrap().cover_url.clone(),
                                artist_name.clone(),
                                artist_id,
                            ),
                            start_index,
                        );
                        self.emit(PlayerEvent::SetQueue {
                            tracks: songs.clone(),
                            playlist: Arc::new(Playlist::from_artist_hot_songs(
                                songs.first().unwrap().cover_url.clone(),
                                artist_name,
                                artist_id,
                            )),
                            start_index,
                        });
                    }
                }

                self.is_waiting_to_play = false;
                self.play_current();
            }
            PlayerCommand::TogglePlayPause => {
                self.engine.toggle();
            }
            PlayerCommand::Seek(offset_ms) => {
                self.engine.seek(offset_ms);
            }
            PlayerCommand::Next => {
                self.is_waiting_to_play = false;
                if self.queue.advance(false) {
                    self.play_current();
                } else {
                    let _ = self
                        .event_tx
                        .send(WindowMsg::PlayerEventReceived(PlayerEvent::EndOfQueue));
                }
            }
            PlayerCommand::Previous => {
                if self.queue.go_back() {
                    self.play_current();
                }
            }
            PlayerCommand::Remove(index) => {
                self.queue.remove(index);
                self.emit(PlayerEvent::SetQueue {
                    tracks: self.queue.get_queue(),
                    playlist: Arc::new(self.queue.current_playlist.clone().unwrap_or_default()),
                    start_index: self.queue.current_index.unwrap_or(0),
                });
            }
            PlayerCommand::PlayAt(index) => {
                self.queue.play(index);
                self.play_current();
            }
            PlayerCommand::SetPlayMode(mode) => {
                self.queue.set_play_mode(mode);
            }
            PlayerCommand::SetLoop(enabled) => {
                self.queue.set_loop_enabled(enabled);
            }
            PlayerCommand::LikeSong { song_id, liked } => {
                let tx = self.event_tx.clone();
                async_runtime().spawn(async move {
                    let result = like_song(song_id, liked).await;
                    let msg = match (result.is_ok(), liked) {
                        (true, true) => "已喜欢".to_string(),
                        (true, false) => "已取消喜欢".to_string(),
                        (false, _) => "操作失败".to_string(),
                    };
                    let _ = tx.send(WindowMsg::ShowToast(msg));
                });
            }
        }
    }

    fn handle_internal(&mut self, ev: InternalEvent) {
        match ev {
            InternalEvent::SongsFetched { songs } => {
                eprintln!("Songs fetched");
                let hit_current = self.queue.apply_fetched(songs);
                if hit_current && self.is_waiting_to_play {
                    self.play_current();
                }
            }
            InternalEvent::UrlResolved { song_id, url, is_liked } => {
                eprintln!("Url resolved: {:?}", song_id);
                let is_current = self.queue.current().map_or(
                    false,
                    |item| matches!(item, QueueItem::Full(s) if s.id == song_id),
                );
                if !is_current {
                    return;
                }

                // 找到 song 的完整信息用于通知 UI / MPRIS
                let song = self.find_song(song_id).unwrap();
                self.engine.play_url(&url);

                let _ = self.mpris_tx.send(MprisUpdate::Metadata(song.clone()));
                self.emit(PlayerEvent::TrackChanged {
                    song,
                    current_index: self.queue.current_index.unwrap_or(0),
                    is_liked,
                });
                self.emit(PlayerEvent::StateChanged(PlaybackState::Playing));
                // if let(Some(start_index)) = self.queue.current_index {
                //     self.emit(PlayerEvent::SetQueue { songs: self.queue., start_index });
                // }
            }
            InternalEvent::UrlResolveFailed { song_id } => {
                eprintln!("URL resolve failed for {song_id}");
                log::warn!("URL resolve failed for {song_id}, skipping to next");
                self.handle_cmd(PlayerCommand::Next);
            }
            InternalEvent::PlaylistFetched {
                playlist: playlist_detail,
            } => {
                self.handle_cmd(PlayerCommand::Play {
                    source: PlaySource::LazyQueue {
                        tracks: Arc::new(playlist_detail.tracks.clone()),
                        track_ids: Arc::new(playlist_detail.track_ids.clone()),
                        playlist: playlist_detail.into(),
                    },
                    start_index: 0,
                });
            }
            InternalEvent::AlbumFetched { album } => {
                self.handle_cmd(PlayerCommand::Play {
                    source: PlaySource::LazyQueue {
                        tracks: Arc::new(album.tracks.clone()),
                        track_ids: Arc::new(album.tracks.iter().map(|a|a.id).collect()),
                        playlist: album.into(),
                    },
                    start_index: 0,
                });
            }
            InternalEvent::DailyRecommendFetched { songs } => {
                self.handle_cmd(PlayerCommand::Play {
                    source: PlaySource::LazyQueue {
                        tracks: Arc::new(songs.clone()),
                        track_ids: Arc::new(songs.iter().map(|s|s.id).collect()),
                        playlist: Playlist::from_daily_recommend(songs),
                    },
                    start_index: 0,
                });
            }
        }
    }

    fn handle_gst(&mut self, ev: GstEvent) {
        match ev {
            GstEvent::State(state) => {
                let _ = self
                    .mpris_tx
                    .send(MprisUpdate::PlaybackState(state.clone()));
                self.emit(PlayerEvent::StateChanged(state));
            }
            GstEvent::EndOfStream => {
                if self.queue.advance(true) {
                    self.play_current();
                }
            }
            GstEvent::Position { position, duration } => {
                self.emit(PlayerEvent::TimeUpdated { position, duration });
            }
            GstEvent::Error(msg) => {
                log::error!("GStreamer error: {msg}");
                self.emit(PlayerEvent::Error(msg));
            }
        }
    }

    fn play_current(&mut self) {
        // 触发预加载（纯队列操作，无副作用）
        let preload_ids = self.queue.take_preload_ids();
        if !preload_ids.is_empty() {
            self.spawn_song_fetch(preload_ids);
        }

        match self.queue.current() {
            None => {}
            Some(QueueItem::Full(song)) => {
                let song_id = song.id;
                self.is_waiting_to_play = false;
                self.spawn_url_resolve(song_id);
            }
            Some(QueueItem::Id(id)) => {
                let song_id = *id;
                self.is_waiting_to_play = true;
                self.spawn_song_fetch(vec![song_id]);
            }
            Some(QueueItem::Loading(_)) => {
                self.is_waiting_to_play = true;
            }
        }
    }

    // ── 异步任务派发 ────────────────────────────────────────────────

    fn spawn_url_resolve(&self, song_id: u64) {
        let tx = self.internal_tx.clone();
        async_runtime().spawn(async move {
            let url_result = get_song_url(song_id, SoundQuality::Standard).await;
            let like_result = is_like_song(song_id).await;
            let is_liked = like_result.unwrap_or(false);
            match url_result {
                Ok(url) => {
                    let _ = tx.send(InternalEvent::UrlResolved { song_id, url, is_liked });
                }
                Err(_) => {
                    let _ = tx.send(InternalEvent::UrlResolveFailed { song_id });
                }
            }
        });
    }

    fn spawn_song_fetch(&self, ids: Vec<u64>) {
        let tx = self.internal_tx.clone();
        async_runtime().spawn(async move {
            match get_song_detail(ids).await {
                Ok(songs) => {
                    let _ = tx.send(InternalEvent::SongsFetched { songs });
                }
                Err(e) => {
                    log::error!("batch fetch failed: {e:?}");
                }
            }
        });
    }

    fn spawn_playlist_fetch(&self, playlist_id: u64) {
        let tx = self.internal_tx.clone();
        eprint!("Fetching playlist {playlist_id}...");
        async_runtime().spawn(async move {
            match get_playlist_detail(playlist_id).await {
                Ok(playlist) => {
                    let _ = tx.send(InternalEvent::PlaylistFetched { playlist });
                }
                Err(e) => {
                    log::error!("playlist fetch failed: {e:?}");
                }
            }
        });
    }

    fn spawn_album_fetch(&self, album_id: u64) {
        let tx = self.internal_tx.clone();
        async_runtime().spawn(async move {
            match get_album_detail(album_id).await {
                Ok(album) => {
                    let _ = tx.send(InternalEvent::AlbumFetched { album });
                }
                Err(e) => {
                    log::error!("album fetch failed: {e:?}");
                }
            }
        });
    }

    fn spwa_daily_recommend_fetch(&self) {
        let tx = self.internal_tx.clone();
        async_runtime().spawn(async move {
            match get_recommend_song().await {
                Ok(songs) => {
                    let _ = tx.send(InternalEvent::DailyRecommendFetched { songs });
                }
                Err(e) => {
                    log::error!("daily recommend fetch failed: {e:?}");
                }
            }
        });
    }

    // ── 工具 ─────────────────────────────────────────────────────────

    fn emit(&self, ev: PlayerEvent) {
        let _ = self.event_tx.send(WindowMsg::PlayerEventReceived(ev));
    }

    fn find_song(&self, song_id: u64) -> Option<Song> {
        self.queue.find_by_id(song_id)
    }
}
