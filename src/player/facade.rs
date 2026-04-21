use relm4::Sender;
use std::{sync::Arc, thread};

use crate::{
    api::{Song, SoundQuality, get_song_detail, get_song_url},
    player::{
        engine::{GstEngine, GstEvent},
        messages::{
            InternalEvent, MprisCommand, MprisUpdate, PlaybackState, PlayerCommand, PlayerEvent,
        },
        mpris,
        queue::{QueueItem, QueueManager},
    },
    ui::window::WindowMsg,
};

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

        thread::spawn(move || {
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
            PlayerCommand::PlayQueue {
                songs,
                full_ids,
                playlist,
                start_index,
            } => {
                self.queue
                    .load(full_ids, songs.clone(), playlist.clone(), start_index);
                self.emit(PlayerEvent::SetQueue {
                    songs: songs,
                    playlist: Arc::new(playlist),
                    start_index,
                });
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
                if self.queue.advance() {
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
                    songs: self.queue.get_queue(),
                    playlist: Arc::new(self.queue.current_playlist.clone().unwrap_or_default()),
                    start_index: self.queue.current_index.unwrap_or(0),
                });
            }
            PlayerCommand::Play(index) => {
                self.queue.play(index);
                self.play_current();
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
            InternalEvent::UrlResolved { song_id, url } => {
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
                self.handle_cmd(PlayerCommand::Next);
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
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(get_song_url(song_id, SoundQuality::Standard)) {
                Ok(url) => {
                    let _ = tx.send(InternalEvent::UrlResolved { song_id, url });
                }
                Err(_) => {
                    let _ = tx.send(InternalEvent::UrlResolveFailed { song_id });
                }
            }
        });
    }

    fn spawn_song_fetch(&self, ids: Vec<u64>) {
        let tx = self.internal_tx.clone();
        thread::spawn(
            move || match futures::executor::block_on(get_song_detail(ids)) {
                Ok(songs) => {
                    let _ = tx.send(InternalEvent::SongsFetched { songs });
                }
                Err(e) => {
                    log::error!("batch fetch failed: {e:?}");
                }
            },
        );
    }

    // ── 工具 ─────────────────────────────────────────────────────────

    fn emit(&self, ev: PlayerEvent) {
        let _ = self.event_tx.send(WindowMsg::PlayerEventReceived(ev));
    }

    fn find_song(&self, song_id: u64) -> Option<Song> {
        self.queue.find_by_id(song_id)
    }
}
