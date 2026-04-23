use std::sync::Arc;

use crate::{api::{AlbumDetail, Playlist, PlaylistDetail, Song}, ui::model::{PlaySource, PlaylistType}};

/// UI 或外部调用者发给播放器的指令（只含用户意图，无内部细节）
#[derive(Debug, Clone)]
pub enum PlayerCommand {
    Play {
        source: PlaySource,
        start_index: usize,
    },
    TogglePlayPause,
    Seek(u64),
    Next,
    Previous,
    Remove(usize),
    PlayAt(usize),
}

/// 播放器向 UI 发出的事件
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlaybackState),
    TimeUpdated { position: u64, duration: u64 },
    TrackChanged{ song: Song, current_index: usize },
    EndOfQueue,
    Error(String),

    SetQueue{ tracks: Arc<Vec<Song>>, playlist: Arc<Playlist>, start_index: usize },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PlaybackState {
    Playing,
    Paused,
    Buffering,
    Stopped,
}

/// 播放器内部异步回调（私有，不对外暴露）
#[derive(Debug)]
pub(crate) enum InternalEvent {
    SongsFetched { songs: Vec<Song> },
    UrlResolved { song_id: u64, url: String },
    UrlResolveFailed { song_id: u64 },
    PlaylistFetched { playlist: PlaylistDetail },
    AlbumFetched { album: AlbumDetail },
    DailyRecommendFetched{ songs: Vec<Song> }
}

/// MPRIS 服务 → 播放器
#[derive(Debug)]
pub enum MprisCommand {
    Play,
    Pause,
    Next,
    Previous,
    Seek(u64),
}

/// 播放器 → MPRIS 服务
pub enum MprisUpdate {
    PlaybackState(PlaybackState),
    Metadata(Song),
}