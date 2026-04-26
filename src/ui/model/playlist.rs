use std::sync::Arc;

use crate::api::{Playlist, Song};

#[derive(Debug, Clone,PartialEq)]
pub enum PlaylistType {
    Playlist(u64),
    Album(u64),
    DailyRecommend,
}

#[derive(Debug, Clone)]
pub enum PlaySource {
    // 需求1：已有部分 tracks，后续靠 ids 懒加载
    LazyQueue {
        tracks: Arc<Vec<Song>>,
        track_ids: Arc<Vec<u64>>,
        playlist: Playlist,
    },
    // 需求2：不进界面，直接用 ID 去拉取
    ById(PlaylistType),
    // 需求3：你现在的需求，直接拿到了完整的 tracks
    DirectTracks(Arc<Vec<Song>>),
    ArtistQueue{
        songs: Arc<Vec<Song>>,
        artist_name: String,
        artist_id: u64,
    }
}


#[derive(Debug, Clone)]
pub struct DetailView {
    pub id: u64,
    pub cover_url: String,
    pub name: String,
    pub creator: Option<String>,
    pub creator_id: u64,
    pub description: Option<String>,
    pub tracks: Vec<Song>,
    pub track_ids: Vec<u64>,
}
