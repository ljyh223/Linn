#[derive(Debug)]
pub enum PlaylistDetailMsg {
    Load(u64),
    Loaded {
        detail: crate::pages::playlist_detail::model::DetailData,
        songs: Vec<crate::pages::playlist_detail::model::SongData>,
    },
    PlaySong(u64),
    Search(String),
}
