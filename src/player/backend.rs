use std::{collections::HashMap, thread};

use gst::ClockTime;
use gst_play::{Play, PlayMessage, PlayState};
use relm4::Sender;

use crate::{api::{Song, SoundQuality}, player::{messages::{MprisCommand, MprisUpdate, PlaybackState, PlayerCommand, PlayerEvent}, mpris}, ui::window::WindowMsg};

const PRELOAD_SIZE: usize = 50;
enum QueueItem {
    Full(Song),     // 已经有完整数据
    Id(i64),        // 只有 id，需要懒加载
    Loading(i64),
}


pub struct PlayerBackend {
    // 高级播放器实例
    play: Play,
    is_playing: bool,
    current_playback_state: PlaybackState,
    // 接收 UI 发来的指令
    cmd_receiver: flume::Receiver<PlayerCommand>,
    // 向 UI 发送状态更新（这里使用 glib::Sender，因为必须在 GTK 主线程处理 UI 更新）
    event_sender: Sender<WindowMsg>,

    internal_tx: flume::Sender<PlayerCommand>,
    queue: Vec<QueueItem>,

    current_index: Option<usize>,
    is_waiting_to_play: bool, 

    // backend → mpris
    mpris_tx: flume::Sender<MprisUpdate>,
    // mpris → backend
    mpris_rx: flume::Receiver<MprisCommand>,


}

impl PlayerBackend {
    /// 启动播放引擎服务，返回向它发送指令的 Sender
    pub fn start(event_sender: Sender<WindowMsg>) -> flume::Sender<PlayerCommand> {
        let (cmd_tx, cmd_rx) = flume::unbounded();
        let (mpris_tx, mpris_rx) = flume::unbounded();
        let (cmd_tx2, cmd_rx2) = flume::unbounded();
        let internal_tx = cmd_tx.clone();

        mpris::start_mpris(mpris_rx.clone(), cmd_tx2.clone());
        thread::spawn(move || {
            let mut backend = PlayerBackend::new(cmd_rx, mpris_tx, cmd_rx2, event_sender, internal_tx);
            backend.run();
        });

        cmd_tx
    }
    fn new(
        cmd_receiver: flume::Receiver<PlayerCommand>,
        mpris_tx: flume::Sender<MprisUpdate>,
        cmd_tx2: flume::Receiver<MprisCommand>,
        event_sender: Sender<WindowMsg>,
        internal_tx: flume::Sender<PlayerCommand>,
    ) -> Self {
        let play = Play::default();
        Self {
            play,
            is_playing: false,
            current_playback_state: PlaybackState::Stopped,
            cmd_receiver,
            event_sender,

            queue: Vec::new(),
            current_index: None,

            mpris_tx,
            mpris_rx: cmd_tx2,
            internal_tx,
            is_waiting_to_play: false,
        }
    }

    fn run(&mut self) {
        let message_bus = self.play.message_bus();
        loop {
            if let Ok(cmd) = self.cmd_receiver.try_recv() {
                self.handle_command(cmd);
            }
            if let Ok(cmd) = self.mpris_rx.try_recv() {
                println!("MprisCommand received: {:?}", cmd);
                match cmd {
                    MprisCommand::Play => {
                        self.play.play();
                        self.is_playing = true;
                    },
                    MprisCommand::Pause => {
                        self.play.pause();
                        self.is_playing = false;
                    },
                    MprisCommand::Next => self.handle_command(PlayerCommand::Next),
                    MprisCommand::Previous => self.handle_command(PlayerCommand::Previous),
                    MprisCommand::Seek(offset_ms) => self.handle_command(PlayerCommand::Seek(offset_ms)),
                }
            }
            if let Some(msg) = message_bus.timed_pop(gst::ClockTime::from_mseconds(10)) {
                self.handle_gst_message(&msg);
            }
        }
    }

    fn handle_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::PlayTrack(url) => {
                log::info!("Player: Loading track -> {}", url);
                self.play.set_uri(Some(&url));
                self.play.play();
                self.is_playing=true;
            }
            PlayerCommand::TogglePlayPause => {
                if self.is_playing { self.play.pause() } else { self.play.play() }
            }
            PlayerCommand::PlayQueue { songs, full_ids, start_index } => {
                self.queue = self.build_queue_consume_tracks(&full_ids, songs);
                self.current_index = Some(start_index);
                self.play_current();
            },
            PlayerCommand::Seek(offset_ms) => {
                if let Some(current_pos) = self.play.position() {
                    let current_ms = current_pos.mseconds();
                    let target_ms = (current_ms  + offset_ms as u64).max(0);
                    self.play.seek(gst::ClockTime::from_mseconds(target_ms));
                }
            },
            PlayerCommand::Next => {
                self.is_waiting_to_play = false;
                if let Some(i) = self.current_index {
                    if i + 1 < self.queue.len() {
                        self.current_index = Some(i + 1);
                        println!("current_index :{:?}", self.current_index);
                        self.play_current();
                    }
                }
            }
            PlayerCommand::Previous => {
                if let Some(i) = self.current_index {
                    if i > 0 {
                        self.current_index = Some(i - 1);
                        self.play_current();
                    }
                }
            }
            PlayerCommand::SongsFetched { songs } => {
                let mut needs_play = false;
                let current_id = self.current_index
                    .and_then(|idx| self.queue.get(idx))
                    .and_then(|item| match item {
                        QueueItem::Loading(id) | QueueItem::Id(id) => Some(*id),
                        _ => None,
                    });

                // 更新队列中的状态
                for song in songs {
                    if let Some(pos) = self.queue.iter().position(|item| {
                        matches!(item, QueueItem::Loading(id) | QueueItem::Id(id) if *id == song.id)
                    }) {
                        self.queue[pos] = QueueItem::Full(song.clone());
                    }

                    // 如果刚加载完的这首歌正是我们正在等待播放的歌
                    if self.is_waiting_to_play && Some(song.id) == current_id {
                        needs_play = true;
                    }
                }

                // 数据有了，继续走 play_current 流程去获取 URL
                if needs_play {
                    self.play_current();
                }
            }

            PlayerCommand::UrlResolved { song_id, url } => {
                let is_still_current = self.current_index
                    .and_then(|idx| self.queue.get(idx))
                    .map(|item| matches!(item, QueueItem::Full(song) if song.id == song_id))
                    .unwrap_or(false);
                let song= self.queue.iter().find_map(|item| {
                    if let QueueItem::Full(s) = item {
                        if s.id == song_id {
                            return Some(s.clone());
                        }
                    }
                    None
                }).unwrap();

                if is_still_current {
                    log::info!("URL resolved, starting playback: {}", url);
                    self.play.set_uri(Some(&url));
                    self.play.play();
                    self.is_playing=true;

                    self.mpris_tx.send(
                        MprisUpdate::Metadata(song.clone())
                    ).ok();
                    
                    // 通知 UI 和 MPRIS 
                    // (如果需要 Metadata，你可以从 self.queue 中取出完整的 Song 发给 mpris_tx)
                }
            }
            PlayerCommand::UrlResolveFailed { song_id } => {
                log::error!("Failed to resolve URL for song ID: {}", song_id);
                // 自动跳到下一首 或者 报错给 UI
                self.handle_command(PlayerCommand::Next);
            }


        }
    }
    fn play_current(&mut self) {
        let Some(index) = self.current_index else { return };
        
        // 触发预加载
        self.preload_next();

        let item = self.queue.get_mut(index).unwrap();
        match item {
            QueueItem::Full(song) => {
                self.is_waiting_to_play = false;
                // 注意：这里也不要阻塞，起后台任务去拿 URL
                let song_id = song.id;
                let tx = self.internal_tx.clone();
                std::thread::spawn(move || {
                    use crate::api::get_song_url;
                    match futures::executor::block_on(get_song_url(song_id, SoundQuality::Standard)) {
                        Ok(url) => { let _ = tx.send(PlayerCommand::UrlResolved { song_id, url }); }
                        Err(_) => { let _ = tx.send(PlayerCommand::UrlResolveFailed { song_id }); }
                    }
                });
            }

            QueueItem::Id(id) => {
                // 极端情况：用户跳到了一个连详情都没预加载完的歌
                let song_id = *id;
                *item = QueueItem::Loading(song_id);
                self.is_waiting_to_play = true; // 标记等待播放
                
                let tx = self.internal_tx.clone();
                std::thread::spawn(move || {
                    use crate::api::get_song_detail;
                    if let Ok(songs) = futures::executor::block_on(get_song_detail(vec![song_id])) {
                        let _ = tx.send(PlayerCommand::SongsFetched { songs });
                    }
                });
            }

            QueueItem::Loading(_) => {
                log::info!("Song is currently loading, waiting to play...");
                self.is_waiting_to_play = true; // 已经是 Loading 状态，只需打上等待标记
            }
        }
    }

    fn handle_gst_message(&mut self, msg: &gst::Message) {
        // gstreamer_play 封装了消息解析
        if let Ok(play_msg) = PlayMessage::parse(msg) {
            match play_msg {
                PlayMessage::StateChanged(state) => {
                    log::debug!("Player state changed to: {:?}", state);
                    
                    let new_state = match state.state() {
                        PlayState::Stopped => PlaybackState::Stopped,
                        PlayState::Paused => PlaybackState::Paused,
                        PlayState::Playing => PlaybackState::Playing,
                        _ => return,
                    };
                    
                    // 1. 发送给 UI
                    if let Err(e) = self.event_sender.send(WindowMsg::PlayerEventReceived(PlayerEvent::StateChanged(new_state.clone()))) {
                        log::error!("Failed to send player event to UI: {:?}", e);
                    }

                    // 🔥 2. 必须发送给 MPRIS！否则 MPRIS 永远以为音乐是停止的
                    let _ = self.mpris_tx.send(MprisUpdate::PlaybackState(new_state));
                }
                PlayMessage::Error(e) => {
                    log::error!("GStreamer Error: {:?}", e);
                }
                PlayMessage::EndOfStream(_) => {
                    self.handle_command(PlayerCommand::Next);
                }
                _ => {}
            }
        }
    }
    fn preload_next(&mut self) {
        let Some(current) = self.current_index else { return };

        let mut ids_to_fetch = Vec::new();

        for i in (current + 1)..=(current + PRELOAD_SIZE) {
            if let Some(item) = self.queue.get_mut(i) {
                if let QueueItem::Id(id) = item {
                    ids_to_fetch.push(*id);
                    *item = QueueItem::Loading(*id); // 标记为正在加载
                }
            }
        }

        if ids_to_fetch.is_empty() { return; }

        // 将加载任务放到后台线程，坚决不阻塞当前 Event Loop
        let tx = self.internal_tx.clone();
        std::thread::spawn(move || {
            use crate::api::get_song_detail;
            // 在后台线程执行阻塞网络请求
            match futures::executor::block_on(get_song_detail(ids_to_fetch)) {
                Ok(songs) => {
                    // 加载成功后，给主循环发消息
                    let _ = tx.send(PlayerCommand::SongsFetched { songs });
                }
                Err(e) => log::error!("batch fetch failed: {:?}", e),
            }
        });
    }


    fn build_queue_consume_tracks(&self, full_ids: &[i64], tracks: Vec<Song>) -> Vec<QueueItem> {
        let mut track_map: HashMap<i64, Song> = tracks.into_iter().map(|s| (s.id, s)).collect();

        full_ids
            .iter()
            .map(|&id| {
                // remove 会取出所有权，若找不到就返回 Id
                track_map
                    .remove(&id)
                    .map(QueueItem::Full)
                    .unwrap_or(QueueItem::Id(id))
            })
            .collect()
    }

    
}