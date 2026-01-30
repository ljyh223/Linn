use crate::models::{Song, SortField, SortOrder};
use crate::ui::AsyncImage;
use crate::ui::components::song_card::{create_song_card, SongCardData};
use crate::utils::ImageSize;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};
use std::collections::HashMap;

/// 歌曲列表消息
#[derive(Debug, Clone)]
pub enum SongListMessage {
    SortChanged(SortField),
    SongHovered(usize),
    CoverLoaded(String, Result<iced::widget::image::Handle, String>),
}

/// 歌曲列表状态
#[derive(Debug, Clone)]
pub struct SongListState {
    pub songs: Vec<Song>,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub hovered_index: Option<usize>,
    pub cover_images: HashMap<String, AsyncImage>,
}

impl SongListState {
    /// 创建新的歌曲列表状态
    pub fn new(songs: Vec<Song>) -> Self {
        Self {
            songs,
            sort_field: SortField::Default,
            sort_order: SortOrder::Default,
            hovered_index: None,
            cover_images: HashMap::new(),
        }
    }

    /// 处理消息
    pub fn update(&mut self, message: SongListMessage) {
        match message {
            SongListMessage::SortChanged(field) => {
                if self.sort_field == field {
                    // 同一个字段，切换排序顺序
                    self.sort_order = self.sort_order.toggle();
                } else {
                    // 新字段，重置为升序
                    self.sort_field = field;
                    self.sort_order = SortOrder::Asc;
                }

                // 执行排序
                crate::services::SongService::sort_songs(
                    &mut self.songs,
                    self.sort_field,
                    self.sort_order,
                );
            }
            SongListMessage::SongHovered(index) => {
                self.hovered_index = Some(index);
            }
            SongListMessage::CoverLoaded(url, result) => {
                match result {
                    Ok(handle) => {
                        self.cover_images.insert(url, AsyncImage::loaded(handle));
                    }
                    Err(_) => {
                        self.cover_images.insert(url, AsyncImage::failed());
                    }
                }
            }
        }
    }

    /// 初始化封面图片（设置为 Loading 状态）
    pub fn init_cover_images(&mut self) {
        for song in &self.songs {
            let url = song.cover_url.clone();
            if !self.cover_images.contains_key(&url) && !url.is_empty() {
                self.cover_images.insert(url, AsyncImage::loading());
            }
        }
    }

    /// 获取封面图片
    pub fn get_cover_image(&self, url: &str) -> AsyncImage {
        self.cover_images
            .get(url)
            .cloned()
            .unwrap_or_else(AsyncImage::loading)
    }
}

/// 创建歌曲列表（包含表头和歌曲列表）
pub fn create_song_list(state: &SongListState) -> Element<SongListMessage> {
    let header = create_header(state.sort_field, state.sort_order);

    let song_cards: Vec<Element<SongListMessage>> = state
        .songs
        .iter()
        .enumerate()
        .map(|(index, song)| {
            let size_url = ImageSize::Small.apply_to_url(&song.cover_url);
            let cover_image = state.get_cover_image(&size_url);
            let card_data = SongCardData {
                index: index + 1,
                id: song.id,
                name: song.name.clone(),
                artists: song.artists_string(),
                album: song.album.name.clone(),
                duration: song.format_duration(),
                cover_url: song.cover_url.clone(),
                cover_image,
            };

            let is_hovered = state.hovered_index == Some(index);

            create_song_card(card_data, is_hovered)
                .map(|_| SongListMessage::SortChanged(SortField::Default))
        })
        .collect();

    let songs_column = column(song_cards).width(Length::Fill);

    // 使用 scrollable 包裹歌曲列表
    let scrollable_songs = scrollable(songs_column)
        .width(Length::Fill)
        .height(Length::Fill);

    column![header, scrollable_songs]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 创建表头（可排序）
fn create_header(
    sort_field: SortField,
    sort_order: SortOrder,
) -> Element<'static, SongListMessage> {
    // 创建排序列按钮
    let sortable_column = |label: String, field: SortField, width: Length| -> Element<SongListMessage> {
        let is_active = sort_field == field && sort_order != SortOrder::Default;
        let arrow = sort_order.arrow_symbol();

        let label_text = if is_active {
            text(format!("{}{}", label, arrow))
                .size(13)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.3, 0.6, 1.0)), // 激活颜色：蓝色
                })
        } else {
            text(label.clone())
                .size(13)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.55, 0.55, 0.6)),
                })
        };

        let btn = button(label_text)
            .width(width)
            .on_press(SongListMessage::SortChanged(field))
            .style(move |_theme, _status| button::Style {
                background: None,
                text_color: if is_active {
                    iced::Color::from_rgb(0.3, 0.6, 1.0)
                } else {
                    iced::Color::from_rgb(0.55, 0.55, 0.6)
                },
                border: iced::border::Border {
                    color: iced::Color::TRANSPARENT,
                    ..Default::default()
                },
                ..Default::default()
            });

        container(btn)
            .width(width)
            .height(Length::Fixed(40.0))
            .align_y(iced::alignment::Vertical::Center)
            .into()
    };

    // 固定列（不可排序）
    let fixed_column = |label: String, width: Length| -> Element<SongListMessage> {
        container(
            text(label)
                .size(13)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.55, 0.55, 0.6)),
                }),
        )
        .width(width)
        .height(Length::Fixed(40.0))
        .align_y(iced::alignment::Vertical::Center)
        .into()
    };

    // 封面（48px，不可排序）
    let cover_header = fixed_column("封面".to_string(), Length::Fixed(48.0));

    // 序号（50px，不可排序）
    let index_header = fixed_column("#".to_string(), Length::Fixed(50.0));

    // 歌名（可排序）
    let title_header = sortable_column("歌名".to_string(), SortField::Title, Length::Fill);

    // 歌手（可排序）
    let artist_header = sortable_column("歌手".to_string(), SortField::Artist, Length::Fill);

    // 专辑（可排序）
    let album_header = sortable_column("专辑".to_string(), SortField::Album, Length::Fill);

    // 时长（可排序，80px）
    let duration_header = sortable_column("时长".to_string(), SortField::Duration, Length::Fixed(80.0));

    let header_row = row![cover_header, index_header, title_header, artist_header, album_header, duration_header]
        .spacing(12)
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center);

    container(header_row)
        .width(Length::Fill)
        .padding(iced::Padding {
            left: 16.0,
            right: 16.0,
            top: 0.0,
            bottom: 0.0,
        })
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.06, 0.06, 0.08,
            ))),
            ..Default::default()
        })
        .into()
}
