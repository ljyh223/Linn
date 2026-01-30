use crate::ui::AsyncImage;
use iced::widget::{column, container, text};
use iced::{Element, Length};

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

/// 创建现代化的歌单卡片（参考 Vue Grid 设计）
pub fn create_playlist_card(
    data: PlaylistCardData,
    image_state: AsyncImage,
) -> Element<'static, ()> {
    let cover = create_cover(image_state);
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

/// 创建封面（正方形 1:1，16px 圆角）
fn create_cover(image_state: AsyncImage) -> Element<'static, ()> {
    // Use the new chainable API
    image_state
        .build()
        .width(Length::Fixed(200.0))
        .height(Length::Fixed(200.0))
        .corner_radius(16.0)
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

// ========== 保留旧的实现以兼容代码 ==========

/// 歌单卡片组件（旧实现，保留以兼容）
pub struct PlaylistCard {
    data: PlaylistCardData,
    image_state: AsyncImage,
}

impl PlaylistCard {
    pub fn new(data: PlaylistCardData, image_state: AsyncImage) -> Self {
        Self { data, image_state }
    }

    pub fn view(&self) -> Element<()> {
        let cover = self.view_cover();
        let info = self.view_info();

        container(column![cover, info].spacing(10))
            .padding(12)
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.12, 0.12, 0.15,
                ))),
                border: iced::border::Border {
                    color: iced::Color::from_rgb(0.2, 0.2, 0.25),
                    width: 1.0,
                    radius: (8.0).into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_cover(&self) -> Element<()> {
        let size = Length::Fixed(160.0);

        let content: Element<()> = match &self.image_state {
            AsyncImage::Loaded(handle) => {
                iced::widget::image(handle.clone()).width(size).height(size).into()
            }
            AsyncImage::Loading => container(
                text("加载中...")
                    .size(14)
                    .style(|_theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.5, 0.5, 0.5)),
                    }),
            )
            .width(size)
            .height(size)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.1, 0.1, 0.12,
                ))),
                ..Default::default()
            })
            .into(),
            AsyncImage::Failed => container(
                text("加载失败")
                    .size(14)
                    .style(|_theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.7, 0.3, 0.3)),
                    }),
            )
            .width(size)
            .height(size)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.15, 0.1, 0.1,
                ))),
                ..Default::default()
            })
            .into(),
        };

        container(content)
            .width(size)
            .height(size)
            .style(|_theme| container::Style {
                border: iced::border::Border {
                    color: iced::Color::from_rgb(0.2, 0.2, 0.25),
                    width: 1.0,
                    radius: (6.0).into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_info(&self) -> Element<()> {
        column![
            text(&self.data.name)
                .size(14)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::WHITE),
                }),
            text(format!("by {}", self.data.creator))
                .size(12)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                }),
        ]
        .spacing(4)
        .into()
    }
}

fn create_card_info_old<'a>(data: &'a PlaylistCardData) -> Element<'a, ()> {
    column![
        text(&data.name)
            .size(14)
            .style(|_theme| text::Style {
                color: Some(iced::Color::WHITE),
            }),
        text(format!("by {}", data.creator))
            .size(12)
            .style(|_theme| text::Style {
                color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
            }),
    ]
    .spacing(4)
    .into()
}
