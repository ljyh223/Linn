use iced::widget::{container, text};
use iced::{Element, Length};

/// 我喜欢的音乐页面
#[derive(Debug, Clone, Default)]
pub struct LikedSongsPage;

impl LikedSongsPage {
    /// 创建新的我喜欢的音乐页面
    pub fn new() -> Self {
        Self
    }

    /// 获取页面标题
    pub fn title() -> &'static str {
        "我喜欢的音乐"
    }

    /// 获取页面描述
    pub fn description() -> &'static str {
        "查看所有你喜欢的歌曲"
    }

    /// 渲染页面
    pub fn view(&self) -> Element<()> {
        container(
            text("我喜欢的音乐 - 开发中")
                .size(24)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40)
        .into()
    }
}
