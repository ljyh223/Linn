use iced::widget::{container, row, text};
use iced::{Element, Length};

use crate::ui::components::image::async_image::AsyncImage;
use crate::utils::ImageSize;

/// 歌曲卡片数据
#[derive(Debug, Clone)]
pub struct SongCardData {
    pub index: usize,
    pub id: u64,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub duration: String,
    pub cover_url: String
}

/// 歌曲卡片消息
#[derive(Debug, Clone)]
pub enum SongCardMessage {
    Clicked(usize),
    Hovered(usize),
}

/// 创建歌曲卡片（单行）
///
/// 新布局（带专辑封面）：
/// ┌────────────────────────────────────────────────────────┐
/// │ [封面] | # | 歌名          | 歌手        | 专辑  | 时长 │
/// │  48px  |50| (Fill)        | (Fill)      |(Fill) | 80px │
/// └────────────────────────────────────────────────────────┘
pub fn create_song_card(
    data: SongCardData,
    is_hovered: bool,
) -> Element<'static, SongCardMessage> {
    // 行背景样式
    let background_color = if is_hovered {
        iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)
    } else {
        iced::Color::from_rgba(1.0, 1.0, 1.0, 0.02)
    };

    use iced::widget::list;
    // 专辑封面（48x48）
    let cover = AsyncImage::new(data.cover_url.clone())
        .width(Length::Fixed(48.0))
        .height(Length::Fixed(48.0))
        .border_radius(8.0)
        .size(ImageSize::Small)
        .view();

    // 序号（50px）
    let index = text(data.index.to_string())
        .size(14)
        .width(Length::Fixed(50.0))
        .style(|_theme| text::Style {
            color: Some(iced::Color::from_rgb(0.55, 0.55, 0.6)),
        });

    // 歌名（Fill，文本左对齐）
    let name = truncate_text(&data.name, 35);
    let name_text = text(name)
        .size(14)
        .width(Length::Fill)
        .style(|_theme| text::Style {
            color: Some(iced::Color::WHITE),
        });

    // 歌手（Fill，文本左对齐）
    let artists = truncate_text(&data.artists, 25);
    let artists_text = text(artists)
        .size(13)
        .width(Length::Fill)
        .style(|_theme| text::Style {
            color: Some(iced::Color::from_rgb(0.7, 0.7, 0.75)),
        });

    // 专辑（Fill，文本左对齐）
    let album = truncate_text(&data.album, 25);
    let album_text = text(album)
        .size(13)
        .width(Length::Fill)
        .style(|_theme| text::Style {
            color: Some(iced::Color::from_rgb(0.7, 0.7, 0.75)),
        });

    // 时长（80px）
    let duration_text = text(data.duration)
        .size(13)
        .width(Length::Fixed(80.0))
        .style(|_theme| text::Style {
            color: Some(iced::Color::from_rgb(0.55, 0.55, 0.6)),
        });

    // 行容器
    let row_content = row![cover, index, name_text, artists_text, album_text, duration_text]
        .spacing(12)
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center);

    container(row_content)
        .width(Length::Fill)
        .height(Length::Fixed(56.0))
        .padding(iced::Padding {
            left: 16.0,
            right: 16.0,
            top: 0.0,
            bottom: 0.0,
        })
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(background_color)),
            ..Default::default()
        })
        .into()
}

/// 截断文本到指定字符数
fn truncate_text(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();

    if char_count <= max_chars {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let end_index = max_chars.saturating_sub(3);

    if end_index == 0 {
        return "...".to_string();
    }

    let truncated: String = chars[..end_index].iter().collect();
    format!("{}...", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_short() {
        assert_eq!(truncate_text("Short", 10), "Short");
    }

    #[test]
    fn test_truncate_text_long() {
        assert_eq!(truncate_text("This is a very long text", 10), "This is...");
    }

    #[test]
    fn test_truncate_text_exact() {
        assert_eq!(truncate_text("Exact", 5), "Exact");
    }

    #[test]
    fn test_truncate_text_very_short() {
        assert_eq!(truncate_text("Hi", 2), "Hi");
    }
}
