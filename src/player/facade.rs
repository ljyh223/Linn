use relm4::Sender;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::{
    api::{
        Playlist, Song, SoundQuality, get_album_detail, get_home_category_daily_song_list,
        get_playlist_detail, get_recommend_song, get_song_detail, get_song_url, is_like_song,
        like_song, refresh_song_url,
    },
    db::Db,
    player::messages::PlayMode,
    player::{
        engine::{GstEngine, GstEvent},
        messages::{
            InternalEvent, MprisCommand, MprisUpdate, PlaybackState, PlayerCommand, PlayerEvent,
        },
        mpris,
        queue::{QueueItem, QueueManager},
    },
    ui::model::{PlaySource, PlaylistType},
};

fn async_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create async runtime"))
}

pub struct PlayerFacade {
    engine: GstEngine,
    queue: QueueManager,
    is_waiting_to_play: bool,
    db: Arc<Mutex<Db>>,

    /// 恢复会话后，等首次歌曲批量拉取完成，向 UI 补发 SetQueue
    restore_ui_refresh: bool,

    /// 恢复会话但不需要自动播放时：等当前歌曲 URL 就绪后立即暂停
    pause_after_start: bool,

    /// 已因播放错误刷新过 URL 的歌曲。每首歌曲最多自动恢复一次，避免失败时循环重试。
    retrying_song: Option<u64>,
    /// 刷新 URL 重建播放管线后需要恢复的播放位置。
    resume_position_after_refresh: Option<u64>,
    /// 有歌曲快照的启动恢复：等当前播放 URL 就绪后再后台加载整队详情，避免抢占首播请求。
    restore_queue_ids_after_start: Option<Vec<u64>>,
    /// 每首歌曲最近一次喜欢操作的版本。较早的异步查询不得覆盖它。
    like_status_generations: HashMap<u64, u64>,
    next_like_generation: u64,

    cmd_rx: flume::Receiver<PlayerCommand>,
    internal_rx: flume::Receiver<InternalEvent>,
    internal_tx: flume::Sender<InternalEvent>,

    event_tx: Sender<PlayerEvent>,

    mpris_tx: flume::Sender<MprisUpdate>,
    mpris_rx: flume::Receiver<MprisCommand>,
}

impl PlayerFacade {
    pub fn start(
        event_tx: Sender<PlayerEvent>,
        db: Arc<Mutex<Db>>,
    ) -> flume::Sender<PlayerCommand> {
        let (cmd_tx, cmd_rx) = flume::unbounded::<PlayerCommand>();
        let (internal_tx, internal_rx) = flume::unbounded::<InternalEvent>();
        let (mpris_update_tx, mpris_update_rx) = flume::unbounded::<MprisUpdate>();
        let (mpris_cmd_tx, mpris_cmd_rx) = flume::unbounded::<MprisCommand>();

        mpris::start_mpris(mpris_update_rx, mpris_cmd_tx);
        let _ = async_runtime();

        let saved_play_mode = db.lock().unwrap().get_play_mode();
        let saved_loop_enabled = db.lock().unwrap().get_loop_enabled();

        let mut queue = QueueManager::new();
        queue.set_play_mode(saved_play_mode);
        queue.set_loop_enabled(saved_loop_enabled);

        std::thread::spawn(move || {
            PlayerFacade {
                engine: GstEngine::new(),
                queue,
                is_waiting_to_play: false,
                restore_ui_refresh: false,
                pause_after_start: false,
                retrying_song: None,
                resume_position_after_refresh: None,
                restore_queue_ids_after_start: None,
                like_status_generations: HashMap::new(),
                next_like_generation: 0,
                db,
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
        self.emit_playback_settings();
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
                    MprisCommand::SeekRelative(offset_ms) => {
                        let position = self.engine.position_ms() as i64;
                        let duration = self.engine.duration_ms() as i64;
                        let target = position.saturating_add(offset_ms).clamp(0, duration) as u64;
                        self.handle_cmd(PlayerCommand::Seek(target));
                    }
                    MprisCommand::SetPosition(position_ms) => {
                        self.handle_cmd(PlayerCommand::Seek(position_ms));
                    }
                    MprisCommand::SetLoopStatus(status) => self.set_mpris_loop_status(status),
                    MprisCommand::SetShuffle(shuffle) => self.set_mpris_shuffle(shuffle),
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
                self.retrying_song = None;
                self.resume_position_after_refresh = None;
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
                        PlaylistType::DailyCategory {
                            tag_id,
                            category_id,
                            song_ids,
                            title,
                            cover,
                        } => {
                            self.spawn_daily_category_fetch(
                                tag_id,
                                category_id,
                                song_ids,
                                title,
                                cover,
                            );
                        }
                    },
                    PlaySource::DirectTracks(songs) => {
                        self.queue.load(
                            Arc::new(songs.iter().map(|s| s.id).collect()),
                            songs.clone(),
                            Playlist::from_suggest(
                                songs
                                    .first()
                                    .map(|s| s.cover_url.clone())
                                    .unwrap_or_default(),
                                songs.first().map(|s| s.name.clone()).unwrap_or_default(),
                            ),
                            start_index,
                        );
                        self.emit(PlayerEvent::SetQueue {
                            tracks: songs.clone(),
                            playlist: Arc::new(Playlist::from_suggest(
                                songs
                                    .first()
                                    .map(|s| s.cover_url.clone())
                                    .unwrap_or_default(),
                                songs.first().map(|s| s.name.clone()).unwrap_or_default(),
                            )),
                            start_index,
                        });
                    }
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
            PlayerCommand::Pause => {
                self.engine.pause();
            }
            PlayerCommand::Resume => {
                self.engine.resume();
            }
            PlayerCommand::Seek(offset_ms) => {
                self.engine.seek(offset_ms);
            }
            PlayerCommand::Next => {
                self.is_waiting_to_play = false;
                self.retrying_song = None;
                self.resume_position_after_refresh = None;
                if self.queue.advance(false) {
                    self.play_current();
                } else {
                    let _ = self.event_tx.send(PlayerEvent::EndOfQueue);
                }
            }
            PlayerCommand::Previous => {
                self.retrying_song = None;
                self.resume_position_after_refresh = None;
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
                self.retrying_song = None;
                self.resume_position_after_refresh = None;
                self.queue.play(index);
                self.play_current();
            }
            PlayerCommand::SetPlayMode(mode) => {
                self.queue.set_play_mode(mode);
                self.db.lock().unwrap().set_play_mode(mode);
                self.emit_playback_settings();
            }
            PlayerCommand::SetLoop(enabled) => {
                self.queue.set_loop_enabled(enabled);
                self.db.lock().unwrap().set_loop_enabled(enabled);
                self.emit_playback_settings();
            }
            PlayerCommand::SyncSettings => {
                self.emit_playback_settings();
            }
            PlayerCommand::RestoreSession {
                track_ids,
                current_index,
                current_song,
                playlist,
                autoplay,
            } => {
                let snapshot_tracks = current_song.clone().into_iter().collect::<Vec<_>>();
                self.queue.load(
                    track_ids.clone(),
                    Arc::new(snapshot_tracks),
                    playlist,
                    current_index,
                );
                self.restore_ui_refresh = true;
                if !autoplay {
                    self.pause_after_start = true;
                }
                if let Some(song) = current_song {
                    // 当前歌曲已在会话中持久化，先恢复播放；整队歌曲详情继续在后台校验。
                    self.is_waiting_to_play = false;
                    let _ = self.mpris_tx.send(MprisUpdate::Metadata(song));
                    self.restore_queue_ids_after_start = Some(track_ids.as_ref().clone());
                    self.play_current();
                } else {
                    // 升级前的会话没有歌曲快照，保留旧流程作为兼容回退。
                    self.is_waiting_to_play = true;
                    self.spawn_song_fetch(track_ids.as_ref().clone());
                }
            }
            PlayerCommand::LikeSong { song_id, liked } => {
                self.next_like_generation = self.next_like_generation.wrapping_add(1);
                let generation = self.next_like_generation;
                self.like_status_generations.insert(song_id, generation);
                let tx = self.internal_tx.clone();
                async_runtime().spawn(async move {
                    let succeeded = like_song(song_id, liked).await.is_ok();
                    let _ = tx.send(InternalEvent::LikeActionFinished {
                        song_id,
                        liked,
                        generation,
                        succeeded,
                    });
                });
            }
        }
    }

    fn handle_internal(&mut self, ev: InternalEvent) {
        match ev {
            InternalEvent::SongsFetched { songs } => {
                eprintln!("Songs fetched");
                let hit_current = self.queue.apply_fetched(songs);
                if self.restore_ui_refresh {
                    if let Some(playlist) = self.queue.current_playlist.clone() {
                        self.emit(PlayerEvent::SetQueue {
                            tracks: self.queue.get_queue(),
                            playlist: Arc::new(playlist),
                            start_index: self.queue.current_index.unwrap_or(0),
                        });
                    }
                    self.restore_ui_refresh = false;
                }
                if hit_current && self.is_waiting_to_play {
                    self.play_current();
                }
            }
            InternalEvent::UrlResolved {
                song_id,
                url,
                is_liked,
            } => {
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
                if let Some(track_ids) = self.restore_queue_ids_after_start.take() {
                    self.spawn_song_fetch(track_ids);
                }
                if let Some(position) = self.resume_position_after_refresh.take() {
                    self.engine.seek(position);
                }

                // 恢复但不需要自动播放：就绪后立即暂停，停在暂停态
                let state = if self.pause_after_start {
                    self.engine.pause();
                    self.pause_after_start = false;
                    PlaybackState::Paused
                } else {
                    PlaybackState::Playing
                };

                let _ = self.mpris_tx.send(MprisUpdate::Metadata(song.clone()));
                self.emit(PlayerEvent::TrackChanged {
                    song,
                    current_index: self.queue.current_index.unwrap_or(0),
                    is_liked,
                });
                self.emit(PlayerEvent::StateChanged(state));
                // if let(Some(start_index)) = self.queue.current_index {
                //     self.emit(PlayerEvent::SetQueue { songs: self.queue., start_index });
                // }
            }
            InternalEvent::UrlResolveFailed { song_id } => {
                eprintln!("URL resolve failed for {song_id}");
                log::warn!("URL resolve failed for {song_id}, skipping to next");
                self.handle_cmd(PlayerCommand::Next);
            }
            InternalEvent::LikeStatusLoaded {
                song_id,
                is_liked,
                generation,
            } => {
                if generation < self.like_generation(song_id) {
                    return;
                }
                self.emit_current_like_status(song_id, is_liked);
            }
            InternalEvent::LikeActionFinished {
                song_id,
                liked,
                generation,
                succeeded,
            } => {
                if generation != self.like_generation(song_id) {
                    return;
                }

                if succeeded {
                    self.emit_current_like_status(song_id, liked);
                    self.emit(PlayerEvent::ShowToast(if liked {
                        "已喜欢".to_string()
                    } else {
                        "已取消喜欢".to_string()
                    }));
                } else {
                    self.emit(PlayerEvent::ShowToast("操作失败".to_string()));
                    // 失败后重新查询，回滚 UI 的乐观状态；同一版本的查询才可生效。
                    self.spawn_like_status(song_id, generation);
                }
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
                        track_ids: Arc::new(album.tracks.iter().map(|a| a.id).collect()),
                        playlist: album.into(),
                    },
                    start_index: 0,
                });
            }
            InternalEvent::DailyRecommendFetched { songs } => {
                self.handle_cmd(PlayerCommand::Play {
                    source: PlaySource::LazyQueue {
                        tracks: Arc::new(songs.clone()),
                        track_ids: Arc::new(songs.iter().map(|s| s.id).collect()),
                        playlist: Playlist::from_daily_recommend(songs),
                    },
                    start_index: 0,
                });
            }
            InternalEvent::DailyCategoryFetched {
                songs,
                title,
                cover,
            } => {
                let playlist = Playlist {
                    id: 0,
                    name: title,
                    cover_url: cover,
                    creator_name: "网易云音乐".into(),
                    creator_id: 0,
                    description: String::new(),
                    play_count: 0,
                };
                self.handle_cmd(PlayerCommand::Play {
                    source: PlaySource::LazyQueue {
                        tracks: Arc::new(songs.clone()),
                        track_ids: Arc::new(songs.iter().map(|s| s.id).collect()),
                        playlist,
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
                let _ = self.mpris_tx.send(MprisUpdate::Position(position));
                self.emit(PlayerEvent::TimeUpdated { position, duration });
            }
            GstEvent::Error(msg) => {
                log::error!("GStreamer error: {msg}");
                if self.retry_current_after_error() {
                    log::info!(
                        "Refreshing expired or interrupted stream URL and retrying playback"
                    );
                } else {
                    self.emit(PlayerEvent::Error(msg));
                }
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
        let generation = self.like_generation(song_id);
        async_runtime().spawn(async move {
            let url_result = get_song_url(song_id, SoundQuality::Standard).await;
            match url_result {
                Ok(url) => {
                    let _ = tx.send(InternalEvent::UrlResolved {
                        song_id,
                        url,
                        is_liked: false,
                    });
                    let is_liked = is_like_song(song_id).await.unwrap_or(false);
                    let _ = tx.send(InternalEvent::LikeStatusLoaded {
                        song_id,
                        is_liked,
                        generation,
                    });
                }
                Err(_) => {
                    let _ = tx.send(InternalEvent::UrlResolveFailed { song_id });
                }
            }
        });
    }

    fn spawn_url_refresh(&self, song_id: u64) {
        let tx = self.internal_tx.clone();
        let generation = self.like_generation(song_id);
        async_runtime().spawn(async move {
            let url_result = refresh_song_url(song_id, SoundQuality::Standard).await;
            match url_result {
                Ok(url) => {
                    let _ = tx.send(InternalEvent::UrlResolved {
                        song_id,
                        url,
                        is_liked: false,
                    });
                    let is_liked = is_like_song(song_id).await.unwrap_or(false);
                    let _ = tx.send(InternalEvent::LikeStatusLoaded {
                        song_id,
                        is_liked,
                        generation,
                    });
                }
                Err(_) => {
                    let _ = tx.send(InternalEvent::UrlResolveFailed { song_id });
                }
            }
        });
    }

    fn spawn_like_status(&self, song_id: u64, generation: u64) {
        let tx = self.internal_tx.clone();
        async_runtime().spawn(async move {
            let is_liked = is_like_song(song_id).await.unwrap_or(false);
            let _ = tx.send(InternalEvent::LikeStatusLoaded {
                song_id,
                is_liked,
                generation,
            });
        });
    }

    fn like_generation(&self, song_id: u64) -> u64 {
        self.like_status_generations
            .get(&song_id)
            .copied()
            .unwrap_or(0)
    }

    fn emit_current_like_status(&self, song_id: u64, is_liked: bool) {
        if matches!(self.queue.current(), Some(QueueItem::Full(song)) if song.id == song_id)
            && let Some(song) = self.find_song(song_id)
        {
            self.emit(PlayerEvent::TrackChanged {
                song,
                current_index: self.queue.current_index.unwrap_or(0),
                is_liked,
            });
        }
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

    fn spawn_daily_category_fetch(
        &self,
        tag_id: u64,
        category_id: u64,
        song_ids: Vec<u64>,
        title: String,
        cover: String,
    ) {
        let tx = self.internal_tx.clone();
        async_runtime().spawn(async move {
            match get_home_category_daily_song_list(song_ids, category_id, tag_id).await {
                Ok(songs) => {
                    let _ = tx.send(InternalEvent::DailyCategoryFetched {
                        songs,
                        title,
                        cover,
                    });
                }
                Err(e) => {
                    log::error!("daily category fetch failed: {e:?}");
                }
            }
        });
    }

    // ── 工具 ─────────────────────────────────────────────────────────

    fn emit(&self, ev: PlayerEvent) {
        let _ = self.event_tx.send(ev);
    }

    fn emit_playback_settings(&self) {
        let (play_mode, loop_enabled) = self.queue.playback_settings();
        let _ = self.mpris_tx.send(MprisUpdate::PlaybackSettings {
            play_mode,
            loop_enabled,
        });
        self.emit(PlayerEvent::PlaybackSettingsChanged {
            play_mode,
            loop_enabled,
        });
    }

    fn set_mpris_loop_status(&mut self, status: mpris_server::LoopStatus) {
        let (current_mode, _) = self.queue.playback_settings();
        let (play_mode, loop_enabled) = match status {
            mpris_server::LoopStatus::Track => (PlayMode::SingleLoop, false),
            mpris_server::LoopStatus::Playlist => (
                if current_mode == PlayMode::SingleLoop {
                    PlayMode::Sequential
                } else {
                    current_mode
                },
                true,
            ),
            mpris_server::LoopStatus::None => (
                if current_mode == PlayMode::SingleLoop {
                    PlayMode::Sequential
                } else {
                    current_mode
                },
                false,
            ),
        };
        self.queue.set_play_mode(play_mode);
        self.queue.set_loop_enabled(loop_enabled);
        let db = self.db.lock().unwrap();
        db.set_play_mode(play_mode);
        db.set_loop_enabled(loop_enabled);
        drop(db);
        self.emit_playback_settings();
    }

    fn set_mpris_shuffle(&mut self, shuffle: bool) {
        let (play_mode, loop_enabled) = self.queue.playback_settings();
        let next_mode = if shuffle {
            PlayMode::Shuffle
        } else if play_mode == PlayMode::Shuffle {
            PlayMode::Sequential
        } else {
            play_mode
        };
        if next_mode == play_mode {
            return;
        }
        self.queue.set_play_mode(next_mode);
        self.db.lock().unwrap().set_play_mode(next_mode);
        // `Shuffle` 与列表循环是正交设置，保留当前循环开关。
        self.queue.set_loop_enabled(loop_enabled);
        self.emit_playback_settings();
    }

    fn retry_current_after_error(&mut self) -> bool {
        let Some(QueueItem::Full(song)) = self.queue.current() else {
            return false;
        };
        let song_id = song.id;
        if self.retrying_song == Some(song_id) {
            return false;
        }

        self.retrying_song = Some(song_id);
        self.resume_position_after_refresh = Some(self.engine.position_ms());
        self.spawn_url_refresh(song_id);
        true
    }

    fn find_song(&self, song_id: u64) -> Option<Song> {
        self.queue.find_by_id(song_id)
    }
}
