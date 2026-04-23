use relm4::Controller;
use strum::Display;

use crate::ui::{artist::ArtistPage, model::PlaylistType, playlist_detail::PlaylistDetail};

#[derive(Debug, Clone, PartialEq, Display)]
pub enum AppRoute {
    #[strum(serialize = "home")]
    Home,
    #[strum(serialize = "explore")]
    Explore,
    #[strum(serialize = "collection")]
    Collection,
    #[strum(serialize = "playlist-detail")]
    PlaylistDetail(PlaylistType),
    #[strum(serialize = "artist")]
    Artist(u64)
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

pub enum DetailCtrl {
    Playlist(Controller<PlaylistDetail>),
    Artist(Controller<ArtistPage>),
}