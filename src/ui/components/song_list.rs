use crate::models::{Song, SortField, SortOrder};
use crate::ui::components::song_card::{create_song_card, SongCardData};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

/// 歌曲列表消息
#[derive(Debug, Clone)]
pub enum SongListMessage {
    SortChanged(SortField),
    ViewportChanged(iced::widget::scrollable::Viewport), // 新增：视口变化追踪
}

/// 歌曲列表状态
#[derive(Debug, Clone)]
pub struct SongListState {
    pub songs: Vec<Song>,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub hovered_index: Option<usize>,
    pub viewport: Option<iced::widget::scrollable::Viewport>, // 新增：视口追踪
}

impl SongListState {
    /// 创建新的歌曲列表状态
    pub fn new(songs: Vec<Song>) -> Self {
        Self {
            songs,
            sort_field: SortField::Default,
            sort_order: SortOrder::Default,
            hovered_index: None,
            viewport: None, // 新增：初始化视口为 None
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

            SongListMessage::ViewportChanged(viewport) => {
                // 新增：保存视口状态
                self.viewport = Some(viewport);
            }
        }
    }

}

impl SongListState {
    /// 计算当前可见的歌曲索引范围
    fn calculate_visible_range(&self) -> (usize, usize) {
        const SONG_HEIGHT: f32 = 64.0; // 每行歌曲高度
        const SPACING: f32 = 0.0;       // 歌曲行之间没有额外间距
        const BUFFER_ROWS: usize = 5;   // 预加载 5 行作为缓冲

        // 如果还没有视口信息，返回初始范围（前 20 个）
        let viewport = match &self.viewport {
            Some(v) => v,
            None => return (0, self.songs.len().min(20)),
        };

        let bounds = viewport.bounds();
        let relative_y = viewport.relative_offset().y;

        // 计算可见的行范围
        let start_y = relative_y * bounds.height;
        let visible_height = bounds.height;

        let start_row = (start_y / (SONG_HEIGHT + SPACING)).floor() as usize;
        let rows_visible = (visible_height / (SONG_HEIGHT + SPACING)).ceil() as usize;
        let end_row = start_row + rows_visible + BUFFER_ROWS;

        let start_idx = start_row.saturating_sub(BUFFER_ROWS);
        let end_idx = (end_row).min(self.songs.len());

        (start_idx, end_idx)
    }
}

/// 创建歌曲列表（包含表头和歌曲列表）
pub fn create_song_list(state: &SongListState) -> Element<SongListMessage> {
    let header = create_header(state.sort_field, state.sort_order);

    // 计算可见范围
    let (start_idx, end_idx) = state.calculate_visible_range();

    // 只为可见歌曲创建 widget
    let song_cards: Vec<Element<SongListMessage>> = state
        .songs
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .map(|(index, song)| {
            let card_data = SongCardData {
                index: index + 1,
                id: song.id,
                name: song.name.clone(),
                artists: song.artists_string(),
                album: song.album.name.clone(),
                duration: song.format_duration(),
                cover_url: song.cover_url.clone()
            };

            let is_hovered = state.hovered_index == Some(index);

            create_song_card(card_data, is_hovered)
                .map(|_| SongListMessage::SortChanged(SortField::Default))
        })
        .collect();

    // 计算顶部占位高度（为不可见的歌曲留出空间）
    let song_height = 64.0; // 每行歌曲高度
    let top_spacing = (start_idx as f32) * song_height;

    // 构建带占位的滚动内容
    let scrollable_content = column![
        container(text(""))
            .height(Length::Fixed(top_spacing))
            .width(Length::Fill),
        column(song_cards).width(Length::Fill),
    ];

    let scrollable_songs = scrollable(scrollable_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(|viewport| SongListMessage::ViewportChanged(viewport));

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
