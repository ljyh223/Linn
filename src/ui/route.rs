use strum::Display;

#[derive(Debug, Clone, PartialEq, Display)]
pub enum AppRoute {
    #[strum(serialize = "home")]
    Home,
    #[strum(serialize = "explore")]
    Explore,
    #[strum(serialize = "collection")]
    Collection,
    #[strum(serialize = "playlist-detail")]
    PlaylistDetail(i64), // 携带歌单 ID
}