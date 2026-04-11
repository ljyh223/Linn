#[derive(Debug)]
pub struct Playlist{
    pub id: i64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: i64,
    pub description: String,
    pub play_count: i64,
}

pub struct PlaylistDetail {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: i64,
    pub description: String,
    pub play_count: i64,
    pub tracks: Vec<Song>,
}


pub struct Song {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: i64,
}
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
}

pub struct Album {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
}