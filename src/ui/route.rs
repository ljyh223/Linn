use strum::Display;

use crate::ui::model::PlaylistType;

#[derive(Debug, Clone, PartialEq, Display)]
pub enum AppRoute {
    #[strum(serialize = "home")]
    Home,
    #[strum(serialize = "explore")]
    Explore,
    #[strum(serialize = "collection")]
    Collection,
    #[strum(serialize = "playlist-detail")]
    PlaylistDetail(PlaylistType), // 携带歌单 ID
}

#[derive(Debug, Clone, PartialEq, Display)]
pub enum SidebarPage {
    #[strum(serialize = "home")]
    Player,
    #[strum(serialize = "explore")]
    Lyrics,
    #[strum(serialize = "collection")]
    Queue,
}