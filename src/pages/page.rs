use iced::widget::svg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    DailyRecommend,
    Discover,
    LikedSongs,
    Favorites,
    PlaylistDetail(u64), // 歌单详情页，存储 playlist_id
}

impl Page {
    // 主导航页面的列表（不包含 PlaylistDetail）
    pub const NAV_PAGES: [Page; 4] = [
        Page::DailyRecommend,
        Page::Discover,
        Page::LikedSongs,
        Page::Favorites,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Page::DailyRecommend => "为我推荐",
            Page::Discover => "发现音乐",
            Page::LikedSongs => "我喜欢的音乐",
            Page::Favorites => "我的收藏",
            Page::PlaylistDetail(_) => "歌单详情",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Page::DailyRecommend => "每日推荐歌曲，根据你的音乐口味量身定制",
            Page::Discover => "探索新歌、排行榜、歌单和更多内容",
            Page::LikedSongs => "查看所有你喜欢的歌曲",
            Page::Favorites => "收藏的歌单、专辑和艺术家",
            Page::PlaylistDetail(_) => "查看歌单中的所有歌曲",
        }
    }

    pub fn icon(&self) -> svg::Handle {
        match self {
            Page::DailyRecommend => crate::ui::Icons::home(),
            Page::Discover => crate::ui::Icons::discover(),
            Page::LikedSongs => crate::ui::Icons::favorite(),
            Page::Favorites => crate::ui::Icons::star(),
            Page::PlaylistDetail(_) => crate::ui::Icons::star(), // 使用相同的图标
        }
    }
}
