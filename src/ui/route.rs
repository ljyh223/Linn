use relm4::Controller;
use strum::Display;

use crate::ui::{artist::ArtistPage, comments::CommentsPage, model::PlaylistType, playlist_detail::PlaylistDetail};

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
    Artist(u64),
    #[strum(serialize = "comments")]
    Comments(u64)
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
    Comments(Controller<CommentsPage>),
}

/// 侧栏三态循环：半展开 → 全覆盖 → 全收起
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarState {
    /// 半展开：侧栏正常显示
    HalfExpanded,
    /// 全覆盖：侧栏覆盖整个窗口（含 header）
    FullCover,
    /// 全收起：侧栏完全隐藏
    FullCollapsed,
}

impl SidebarState {
    /// 循环切换到下一个状态
    pub fn next(self) -> Self {
        match self {
            SidebarState::HalfExpanded => SidebarState::FullCover,
            SidebarState::FullCover => SidebarState::FullCollapsed,
            SidebarState::FullCollapsed => SidebarState::HalfExpanded,
        }
    }
}