use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::ui::components::image::async_image::AsyncImage;

/// Truncate text to fit within max_chars (adds "..." if truncated)
fn truncate_text(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();

    if char_count <= max_chars {
        return text.to_string();
    }

    // Collect chars and truncate
    let chars: Vec<char> = text.chars().collect();
    let end_index = max_chars.saturating_sub(3); // Leave room for "..."

    if end_index == 0 {
        return "...".to_string();
    }

    let truncated: String = chars[..end_index].iter().collect();
    format!("{}...", truncated)
}

/// 歌单卡片数据
#[derive(Debug, Clone)]
pub struct PlaylistCardData {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub creator: String,
}

impl From<&crate::models::Playlist> for PlaylistCardData {
    fn from(playlist: &crate::models::Playlist) -> Self {
        Self {
            id: playlist.id,
            name: playlist.name.clone(),
            cover_url: playlist.cover_url.clone(),
            creator: playlist.creator.clone(),
        }
    }
}

pub fn create_playlist_card(
    data: PlaylistCardData
) -> Element<'static, ()> {
    let cover = AsyncImage::new(data.cover_url.clone())
                .width(Length::Fixed(140.0))
                .height(Length::Fixed(140.0))
                .border_radius(50.0) // Circle
                .view();
    let info = create_card_info(&data);

    container(column![cover, info])
        .width(Length::Fixed(200.0))
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.12, 0.12, 0.14,
            ))),
            border: iced::border::Border {
                color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                width: 1.0,
                radius: 16.0.into(),
            },
            shadow: iced::Shadow {
                color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                offset: iced::Vector { x: 0.0, y: 2.0 },
                blur_radius: 8.0,
            },
            ..Default::default()
        })
        .into()
}

/// 创建卡片信息（12px padding）
fn create_card_info(data: &PlaylistCardData) -> Element<'static, ()> {
    // Truncate long text - 使用更短的字符数以适应 200px 卡片宽度
    // 中文字符比英文宽，所以需要更少的字符
    let title = truncate_text(&data.name, 12);  // 12 chars for title (~100-120px)
    let creator = truncate_text(&data.creator, 10);  // 10 chars for creator

    container(
        column![
            // 标题 - 16px
            text(title)
                .size(16)
                .width(Length::Fill)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::WHITE),
                }),
            // 作者 - 13px
            text(format!("by {}", creator))
                .size(13)
                .width(Length::Fill)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.55, 0.55, 0.6)),
                }),
        ]
        .spacing(4)
        .width(Length::Fill)
    )
    .padding(12)
    .width(Length::Fill)
    .clip(true)  // 防止文本溢出
    .into()
}
