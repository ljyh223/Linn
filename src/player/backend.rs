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
        gst::init().expect("Failed to initialize GStreamer.");

        // 创建 gstreamer-play 实例
        let play = Play::default();
        Self {
            play,
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
                match cmd {
                    MprisCommand::Play => self.play.play(),
                    MprisCommand::Pause => self.play.pause(),
                    MprisCommand::Next => self.handle_command(PlayerCommand::Next),
                    MprisCommand::Previous => self.handle_command(PlayerCommand::Previous),
                    MprisCommand::Seek(offset) => {
                        self.play.seek(ClockTime::from_mseconds(offset as u64));
                    }
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
                // 设置 URL 并直接播放
                self.play.set_uri(Some(&url));
                self.play.play();
            }
            PlayerCommand::TogglePlayPause => {
                // play 内部并没有提供单独的 toggle 方法，我们要自己判断状态
                // 暂时使用偷懒方法：直接翻转状态（实际应该结合监听来的状态）
            }
            PlayerCommand::PlayQueue { songs, full_ids, start_index } => {
                self.queue = self.build_queue_consume_tracks(&full_ids, songs);
                self.current_index = Some(start_index);
                self.play_current();
            },
            PlayerCommand::Seek(_) => {},
            PlayerCommand::Next => {
                self.is_waiting_to_play = false;
                if let Some(i) = self.current_index {
                    if i + 1 < self.queue.len() {
                        self.current_index = Some(i + 1);
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
                // 检查这首拿到 URL 的歌是否还是当前选中的那首
                // (防止网慢的时候，用户连切好几首歌，导致播放了错的歌)
                let is_still_current = self.current_index
                    .and_then(|idx| self.queue.get(idx))
                    .map(|item| matches!(item, QueueItem::Full(song) if song.id == song_id))
                    .unwrap_or(false);

                if is_still_current {
                    log::info!("URL resolved, starting playback: {}", url);
                    self.play.set_uri(Some(&url));
                    self.play.play();
                    
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

    fn play_song(&mut self, song: &Song) {
        if let Some(url) = self.resolve_url(song.id){
            self.play.set_uri(Some(&url));
            self.play.play();

            let _ = self.event_sender.send(
                WindowMsg::PlayerEventReceived(
                    PlayerEvent::TrackChanged(url.clone())
                )
            );

            self.mpris_tx.send(
                MprisUpdate::Metadata(song.clone())
            ).ok();
        }else {
            let _ = self.event_sender.send(
                WindowMsg::PlayerEventReceived(
                    PlayerEvent::Error(format!("无法获取歌曲URL，ID: {}", song.id))
                )
            );
            log::error!("Failed to resolve URL for song ID: {}", song.id);
        }
    }

    fn handle_gst_message(&mut self, msg: &gst::Message) {
        // gstreamer_play 封装了消息解析
        if let Ok(play_msg) = PlayMessage::parse(msg) {
            match play_msg {
                PlayMessage::StateChanged(state) => {
                    log::debug!("Player state changed to: {:?}", state);
                    // 我们把 Gst 的内部状态转化为我们自己的状态，发送给 UI
                    let new_state = match state.state() {
                        PlayState::Stopped => PlaybackState::Stopped,
                        PlayState::Paused => PlaybackState::Paused,
                        PlayState::Playing => PlaybackState::Playing,
                        _ => return,
                    };
                    
                    if let Err(e) = self.event_sender.send(WindowMsg::PlayerEventReceived(PlayerEvent::StateChanged(new_state))) {
                        log::error!("Failed to send player event to UI: {:?}", e);
                    }
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

    fn fetch_songs_batch(&self, ids: Vec<i64>) -> Vec<Song> {
        use crate::api::get_song_detail;

        match futures::executor::block_on(get_song_detail(ids)) {
            Ok(list) => list,
            Err(e) => {
                log::error!("batch fetch failed: {:?}", e);
                Vec::new()
            }
        }
}


    fn resolve_url(&self, id: i64) -> Option<String> {
        use crate::api::get_song_url;

        match futures::executor::block_on(
            get_song_url(id, SoundQuality::Standard)
        ) {
            Ok(url) => Some(url),
            Err(e) => {
                log::error!("resolve_url failed: {:?}", e);
                None
            }
        }
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