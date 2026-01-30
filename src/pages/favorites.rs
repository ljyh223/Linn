use iced::widget::{container, text};
use iced::{Element, Length};

/// 我的收藏页面
#[derive(Debug, Clone, Default)]
pub struct FavoritesPage;

impl FavoritesPage {
    /// 创建新的我的收藏页面
    pub fn new() -> Self {
        Self
    }

    /// 获取页面标题
    pub fn title() -> &'static str {
        "我的收藏"
    }

    /// 获取页面描述
    pub fn description() -> &'static str {
        "收藏的歌单、专辑和艺术家"
    }

    /// 渲染页面
    pub fn view(&self) -> Element<()> {
        container(
            text("我的收藏 - 开发中")
                .size(24)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40)
        .into()
    }
}
