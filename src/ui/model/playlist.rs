use crate::api::Song;

#[derive(Debug, Clone,PartialEq)]
pub enum PlaylistType {
    Playlist(u64),
    Album(u64),
    DailyRecommend,
}


#[derive(Debug, Clone)]
pub struct DetailView {
    pub id: u64,
    pub cover_url: String,
    pub name: String,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub tracks: Vec<Song>,
    pub track_ids: Vec<u64>,
}
