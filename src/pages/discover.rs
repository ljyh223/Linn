use iced::widget::{container, text};
use iced::{Element, Length};

/// 发现音乐页面
#[derive(Debug, Clone, Default)]
pub struct DiscoverPage;

impl DiscoverPage {
    /// 创建新的发现音乐页面
    pub fn new() -> Self {
        Self
    }

    /// 获取页面标题
    pub fn title() -> &'static str {
        "发现音乐"
    }

    /// 获取页面描述
    pub fn description() -> &'static str {
        "探索新歌、排行榜、歌单和更多内容"
    }

    /// 渲染页面
    pub fn view(&self) -> Element<()> {
        container(
            text("发现音乐页面 - 开发中")
                .size(24)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40)
        .into()
    }
}
