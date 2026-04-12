// player/messages.rs

use crate::api::Song;

/// UI 发给播放器的指令
#[derive(Debug)]
pub enum PlayerCommand {
    PlayTrack(String),    // 播放指定歌曲
    PlayQueue {
        songs: Vec<Song>,
        full_ids: Vec<i64>,
        start_index: usize,
    },
    TogglePlayPause,     // 播放/暂停
    Seek(i64),           // 调整进度 (百分比或秒)
    Next,                // 下一首
    Previous,            // 上一首

    SongsFetched { songs: Vec<Song> },
    UrlResolved { song_id: i64, url: String },
    UrlResolveFailed { song_id: i64 },
}

/// 播放器发给 UI 的状态更新（UI 据此刷新进度条和播放按钮）
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlaybackState), // 播放/暂停/缓冲中
    TimeUpdated { position: u64, duration: u64 }, // 时间更新 (秒/毫秒)
    TrackChanged(String),         // 自动切歌了，告诉 UI 更新封面和歌名
    EndOfQueue,                  // 列表播放完了
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PlaybackState {
    Playing,
    Paused,
    Buffering,
    Stopped,
}


pub enum MprisUpdate {
    PlaybackState(PlaybackState),
    Metadata(Song),
}

#[derive(Debug)]
pub enum MprisCommand {
    // TogglePlayPause,
    Play,
    Pause,
    Next,
    Previous,
    Seek(i64),
}

