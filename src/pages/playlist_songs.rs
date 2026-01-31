use crate::models::{PlaylistDetail};
use crate::services::SongService;
use crate::ui::components::image::async_image::AsyncImage;
use crate::ui::components::{create_song_list, SongListMessage, SongListState};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task};
use std::sync::Arc;

/// 歌单详情页面的消息
#[derive(Debug, Clone)]
pub enum PlaylistSongsMessage {
    FetchSongs(u64),
    SongsFetched(Result<PlaylistDetail, String>),
    Retry,
    SongListMessage(SongListMessage)
}

/// 歌单详情页面
pub struct PlaylistSongsPage {
    song_service: Arc<SongService>,
    playlist_detail: Option<PlaylistDetail>,
    song_list_state: SongListState,
    is_loading: bool,
    error_message: Option<String>,
    window_size: iced::Size
}

impl PlaylistSongsPage {
    /// 创建新的歌单详情页面
    pub fn new(
        song_service: Arc<SongService>,
        window_size: iced::Size,
    ) -> Self {
        Self {
            song_service,
            playlist_detail: None,
            song_list_state: SongListState::new(Vec::new()),
            is_loading: false,
            error_message: None,
            window_size
        }
    }

    /// 获取页面标题
    pub fn title(&self) -> String {
        if let Some(detail) = &self.playlist_detail {
            detail.name.clone()
        } else {
            "歌单详情".to_string()
        }
    }

    /// 处理消息
    pub fn update(&mut self, message: PlaylistSongsMessage) -> Task<PlaylistSongsMessage> {
        match message {
            PlaylistSongsMessage::FetchSongs(playlist_id) => {
                self.is_loading = true;
                self.error_message = None;
                let service = self.song_service.clone();

                Task::perform(
                    async move {
                        service
                            .get_playlist_songs(playlist_id)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    PlaylistSongsMessage::SongsFetched,
                )
            }

            PlaylistSongsMessage::SongsFetched(result) => {
                self.is_loading = false;
                match result {
                    Ok(detail) => {
                        self.playlist_detail = Some(detail.clone());
                        self.song_list_state = SongListState::new(detail.songs);
                        // ✅ 不再初始化封面，由 AsyncImage 自动处理
                    }
                    Err(error) => {
                        self.error_message = Some(error);
                    }
                }
                Task::none() // 注意：这里也要返回 Task::none()
            }

            PlaylistSongsMessage::Retry => {
                if let Some(detail) = &self.playlist_detail {
                    self.is_loading = true;
                    self.error_message = None;
                    let service = self.song_service.clone();
                    let playlist_id = detail.id;

                    Task::perform(
                        async move {
                            service
                                .get_playlist_songs(playlist_id)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        PlaylistSongsMessage::SongsFetched,
                    )
                } else {
                    Task::none()
                }
            }

            PlaylistSongsMessage::SongListMessage(msg) => {
                self.song_list_state.update(msg);
                Task::none()
            }
        }
    }

    /// 渲染页面
    pub fn view(&self) -> Element<PlaylistSongsMessage> {
        let content = if self.is_loading {
            self.view_loading()
        } else if let Some(error) = &self.error_message {
            self.view_error(error)
        } else if self.playlist_detail.is_none() {
            self.view_empty()
        } else if self.song_list_state.songs.is_empty() {
            self.view_no_songs()
        } else {
            self.view_song_list()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.08, 0.08, 0.1,
                ))),
                ..Default::default()
            })
            .into()
    }

    /// 更新窗口大小
    pub fn set_window_size(&mut self, size: iced::Size) {
        self.window_size = size;
    }

    /// 获取歌单歌曲
    pub fn fetch_songs(&mut self, playlist_id: u64) -> Task<PlaylistSongsMessage> {
        self.is_loading = true;
        self.error_message = None;
        let service = self.song_service.clone();

        Task::perform(
            async move {
                service
                    .get_playlist_songs(playlist_id)
                    .await
                    .map_err(|e| e.to_string())
            },
            PlaylistSongsMessage::SongsFetched,
        )
    }

    // === 私有方法 ===

    fn view_loading(&self) -> Element<PlaylistSongsMessage> {
        container(
            column![
                text("正在加载歌单...").size(20),
            ]
            .spacing(10)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn view_error(&self, error: &str) -> Element<PlaylistSongsMessage> {
        let error_msg = error.to_string();
        container(
            column![
                text("加载失败").size(24),
                text(error_msg)
                    .size(14)
                    .style(|_theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.8, 0.3, 0.3)),
                    }),
                button("重试")
                    .on_press(PlaylistSongsMessage::Retry)
                    .padding(iced::Padding::from([8, 16])),
            ]
            .spacing(16)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn view_empty(&self) -> Element<PlaylistSongsMessage> {
        container(
            column![
                text("歌单详情").size(24),
                text("暂无歌单信息").size(14),
            ]
            .spacing(10)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn view_no_songs(&self) -> Element<PlaylistSongsMessage> {
        container(
            column![
                text(self.title()).size(32),
                text("该歌单暂无歌曲").size(16),
            ]
            .spacing(10)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn view_song_list(&self) -> Element<PlaylistSongsMessage> {
        let detail = self.playlist_detail.as_ref().unwrap();

        // 创建增强的 Header（带封面和完整信息）
        let header = self.create_enhanced_header(detail);

        let song_list = create_song_list(&self.song_list_state)
            .map(PlaylistSongsMessage::SongListMessage);

        column![header, song_list]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// 创建增强的 Header（包含封面和完整信息）
    fn create_enhanced_header(&self, detail: &PlaylistDetail) -> Element<'static, PlaylistSongsMessage> {
        // 复制数据以避免生命周期问题
        let name = detail.name.clone();
        let description = if !detail.description.is_empty() {
            truncate_text(&detail.description, 150)
        } else {
            String::new()
        };
        let song_count = self.song_list_state.songs.len();

        // 歌单封面 (200x200)
        let cover = AsyncImage::new(detail.cover_url.clone())
                .width(Length::Fixed(200.0))
                .height(Length::Fixed(200.0))
                .border_radius(50.0) // Circle
                .size(crate::utils::ImageSize::Large)
                .view();

        // 歌单信息
        let info = column![
            // 标题
            text(name)
                .size(28)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::WHITE),
                }),
            // 分隔线
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        1.0, 1.0, 1.0, 0.1,
                    ))),
                    ..Default::default()
                }),
            // 描述（如果有）
            text(description)
                .size(14)
                .style(|_theme: &iced::Theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.7, 0.7, 0.75)),
                }),
            // 歌曲数量
            row![
                text("💿")
                    .size(16)
                    .style(|_theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.7, 0.7, 0.75)),
                    }),
                text(format!("{} 首歌曲", song_count))
                    .size(14)
                    .style(|_theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.7, 0.7, 0.75)),
                    }),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .width(Length::Fill);

        // 主要布局：封面 + 信息
        let main_content = row![cover, info]
            .spacing(24)
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Top);

        // 按钮行
        let button_row = row![
            button(
                row![
                    text("▶").size(14),
                    text("播放全部").size(14),
                ]
                .spacing(8)
            )
            .padding(iced::Padding::from([10, 20]))
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.3, 0.6, 1.0,
                ))),
                text_color: iced::Color::WHITE,
                border: iced::border::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 20.0.into(),
                },
                ..Default::default()
            }),
            button(
                row![
                    text("⭐").size(14),
                    text("收藏").size(14),
                ]
                .spacing(8)
            )
            .padding(iced::Padding::from([10, 20]))
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    1.0, 1.0, 1.0, 0.1,
                ))),
                text_color: iced::Color::WHITE,
                border: iced::border::Border {
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                    width: 1.0,
                    radius: 20.0.into(),
                },
                ..Default::default()
            }),
        ]
        .spacing(12);

        container(column![main_content, button_row].spacing(16))
            .padding(20)
            .width(Length::Fill)
            .into()
    }
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