// 页面类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Discover,        // 为我推荐
    Explore,         // 发现音乐
    Library,         // 我的收藏
    Favorites,       // 我喜欢的歌曲
}

impl Page {
    pub fn title(&self) -> &'static str {
        match self {
            Page::Discover => "为我推荐",
            Page::Explore => "发现音乐",
            Page::Library => "我的收藏",
            Page::Favorites => "我喜欢的歌曲",
        }
    }

    pub fn stack_name(&self) -> &'static str {
        match self {
            Page::Discover => "discover_page",
            Page::Explore => "explore_page",
            Page::Library => "library_page",
            Page::Favorites => "favorites_page",
        }
    }

    pub fn content_label(&self) -> &'static str {
        match self {
            Page::Discover => "为我推荐页面",
            Page::Explore => "发现音乐页面",
            Page::Library => "我的收藏页面",
            Page::Favorites => "我喜欢的歌曲页面",
        }
    }
}

use relm4::prelude::*;

pub fn create_page_label(page: Page) -> gtk::Label {
    gtk::Label::builder()
        .label(page.content_label())
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["dim-label", "title-1"])
        .build()
}
