use std::sync::Arc;
use netease_cloud_music_api::MusicApi;
use relm4::prelude::FactoryComponent;

use crate::pages::playlist_detail::song_item::SongItemOutput;


#[derive(Debug, Clone)]
pub struct SongData {
    pub id: u64,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub cover: String,
    pub duration: u64,
}

#[derive(Debug, Clone)]
pub struct DetailData {
    pub id: u64,
    pub name: String,
    pub cover: String,
    pub description: Option<String>,
    pub creator: Option<String>,
    pub song_count: u64,
    pub play_count: u64,
}

pub struct PlaylistDetail {
    pub detail: Option<DetailData>,
    pub songs: Vec<SongData>,
    pub api: Arc<MusicApi>,
    pub playlist_id: u64,
    pub search: String,
    pub loading: bool,
}
