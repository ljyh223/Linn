use std::sync::Arc;

use crate::{
    api::{AlbumDetail, Playlist, PlaylistDetail, Song},
    ui::model::PlaySource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Sequential,
    SingleLoop,
    Shuffle,
}

/// UI 或外部调用者发给播放器的指令（只含用户意图，无内部细节）
#[derive(Debug, Clone)]
pub enum PlayerCommand {
    Play {
        source: PlaySource,
        start_index: usize,
    },
    TogglePlayPause,
    Pause,
    Resume,
    Seek(u64),
    Next,
    Previous,
    Remove(usize),
    PlayAt(usize),
    SetPlayMode(PlayMode),
    SetLoop(bool),
    /// 请求播放器把持久化的播放设置同步到 UI。
    SyncSettings,
    LikeSong {
        song_id: u64,
        liked: bool,
    },
    /// 启动时恢复上次播放：只重建队列；autoplay=false 时恢复到暂停态。
    RestoreSession {
        track_ids: Arc<Vec<u64>>,
        current_index: usize,
        current_song: Option<Song>,
        playlist: Playlist,
        autoplay: bool,
    },
}

/// 播放器向 UI 发出的事件
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlaybackState),
    PlaybackSettingsChanged {
        play_mode: PlayMode,
        loop_enabled: bool,
    },
    TimeUpdated {
        position: u64,
        duration: u64,
    },
    TrackChanged {
        song: Song,
        current_index: usize,
        is_liked: bool,
    },
    EndOfQueue,
    Error(String),

    SetQueue {
        tracks: Arc<Vec<Song>>,
        playlist: Arc<Playlist>,
        start_index: usize,
    },

    /// 显示 Toast 消息
    ShowToast(String),
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
    SongsFetched {
        songs: Vec<Song>,
    },
    UrlResolved {
        song_id: u64,
        url: String,
        is_liked: bool,
    },
    UrlResolveFailed {
        song_id: u64,
    },
    LikeStatusLoaded {
        song_id: u64,
        is_liked: bool,
        /// 发起查询时该歌曲的喜欢状态版本，用于忽略用户操作前的旧响应。
        generation: u64,
    },
    LikeActionFinished {
        song_id: u64,
        liked: bool,
        generation: u64,
        succeeded: bool,
    },
    PlaylistFetched {
        playlist: PlaylistDetail,
    },
    AlbumFetched {
        album: AlbumDetail,
    },
    DailyRecommendFetched {
        songs: Vec<Song>,
    },
    DailyCategoryFetched {
        songs: Vec<Song>,
        title: String,
        cover: String,
    },
}

/// MPRIS 服务 → 播放器
#[derive(Debug)]
pub enum MprisCommand {
    Play,
    Pause,
    Next,
    Previous,
    /// MPRIS `Seek` 是相对当前播放位置的偏移，单位为毫秒。
    SeekRelative(i64),
    /// MPRIS `SetPosition` 是绝对播放位置，单位为毫秒。
    SetPosition(u64),
    SetLoopStatus(mpris_server::LoopStatus),
    SetShuffle(bool),
}

/// 播放器 → MPRIS 服务
pub enum MprisUpdate {
    PlaybackState(PlaybackState),
    Metadata(Song),
    Position(u64),
    PlaybackSettings {
        play_mode: PlayMode,
        loop_enabled: bool,
    },
}
