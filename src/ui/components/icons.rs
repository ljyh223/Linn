use iced::widget::svg;

pub struct Icons;

impl Icons {
    /// 为我推荐 - 首页图标
    pub fn home() -> svg::Handle {
        svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/icon/Home.svg"))
    }

    /// 发现音乐 - 发现图标
    pub fn discover() -> svg::Handle {
        svg::Handle::from_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/assets/icon/Discover.svg"
        ))
    }

    /// 我喜欢的音乐 - 心形图标
    pub fn favorite() -> svg::Handle {
        svg::Handle::from_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/assets/icon/Favorite.svg"
        ))
    }

    /// 我的收藏 - 星标图标
    pub fn star() -> svg::Handle {
        svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/icon/Star.svg"))
    }
}
