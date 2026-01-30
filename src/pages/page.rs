use iced::widget::svg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    DailyRecommend,
    Discover,
    LikedSongs,
    Favorites,
}

impl Page {
    pub const ALL: [Page; 4] = [
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
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Page::DailyRecommend => "每日推荐歌曲，根据你的音乐口味量身定制",
            Page::Discover => "探索新歌、排行榜、歌单和更多内容",
            Page::LikedSongs => "查看所有你喜欢的歌曲",
            Page::Favorites => "收藏的歌单、专辑和艺术家",
        }
    }

    pub fn icon(&self) -> svg::Handle {
        match self {
            Page::DailyRecommend => crate::ui::Icons::home(),
            Page::Discover => crate::ui::Icons::discover(),
            Page::LikedSongs => crate::ui::Icons::favorite(),
            Page::Favorites => crate::ui::Icons::star(),
        }
    }
}
