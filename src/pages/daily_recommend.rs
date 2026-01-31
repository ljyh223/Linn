use crate::models::Playlist;
use crate::ui::components::{PlaylistCardData, create_playlist_card};
use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length, Task};
use std::sync::Arc;

/// 每日推荐页面的消息
#[derive(Debug, Clone)]
pub enum DailyRecommendMessage {
    FetchRecommendations,
    RecommendationsFetched(Result<Vec<Playlist>, String>),
    NavigatePlaylist(u64), // 新增：点击歌单卡片导航
    ViewportChanged(iced::widget::scrollable::Viewport), // 新增：视口变化追踪
}

/// 每日推荐页面
pub struct DailyRecommendPage {
    playlist_service: Arc<crate::services::PlaylistService>,
    playlists: Vec<Playlist>,
    is_loading: bool,
    error_message: Option<String>,
    window_size: iced::Size,
    viewport: Option<iced::widget::scrollable::Viewport>, // 新增：视口追踪
}

impl DailyRecommendPage {
    /// 创建新的每日推荐页面
    pub fn new(
        playlist_service: Arc<crate::services::PlaylistService>,
        window_size: iced::Size,
    ) -> Self {
        Self {
            playlist_service,
            playlists: Vec::new(),
            is_loading: false,
            error_message: None,
            window_size,
            viewport: None, // 新增：初始化视口为 None
        }
    }

    /// 获取页面标题
    pub fn title() -> &'static str {
        "为我推荐"
    }

    /// 获取页面描述
    pub fn description() -> &'static str {
        "每日推荐歌曲，根据你的音乐口味量身定制"
    }

    /// 处理消息
    pub fn update(&mut self, message: DailyRecommendMessage) -> Task<DailyRecommendMessage> {
        match message {
            DailyRecommendMessage::FetchRecommendations => {
                self.is_loading = true;
                let service = self.playlist_service.clone();

                Task::perform(
                    async move {
                        service
                            .get_recommendations()
                            .await
                            .map_err(|e| e.to_string())
                    },
                    DailyRecommendMessage::RecommendationsFetched,
                )
            }

            DailyRecommendMessage::RecommendationsFetched(result) => {
                self.is_loading = false;

                match result {
                    Ok(playlists) => {
                        self.playlists = playlists;
                        return Task::none();
                    }
                    Err(error) => {
                        self.error_message = Some(error);
                    }
                }

                Task::none()
            }

            DailyRecommendMessage::NavigatePlaylist(_) => {
                // 这个消息由 App 层处理导航
                Task::none()
            }

            DailyRecommendMessage::ViewportChanged(viewport) => {
                // 新增：保存视口状态
                self.viewport = Some(viewport);
                Task::none()
            }
        }
    }

    /// 渲染页面
    pub fn view(&self) -> Element<'_, DailyRecommendMessage> {
        let content = if self.is_loading {
            self.view_loading()
        } else if let Some(error) = &self.error_message {
            self.view_error(error)
        } else if self.playlists.is_empty() {
            self.view_empty()
        } else {
            self.view_playlist_list()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.08, 0.08, 0.1,
                ))),
                ..Default::default()
            })
            .into()
    }

    /// 加载推荐歌单
    pub fn fetch_recommendations(&mut self) -> Task<DailyRecommendMessage> {
        self.is_loading = true;
        let service = self.playlist_service.clone();

        Task::perform(
            async move {
                service
                    .get_recommendations()
                    .await
                    .map_err(|e| e.to_string())
            },
            DailyRecommendMessage::RecommendationsFetched,
        )
    }

    /// 更新窗口大小
    pub fn set_window_size(&mut self, size: iced::Size) {
        self.window_size = size;
    }

    /// 检查数据是否已加载
    pub fn is_data_loaded(&self) -> bool {
        !self.playlists.is_empty()
    }

    // === 私有方法 ===

    fn view_loading(&self) -> Element<'_, DailyRecommendMessage> {
        column![
            text(Self::title()).size(32),
            text("正在获取推荐歌单...").size(16),
        ]
        .spacing(10)
        .into()
    }

    fn view_error(&self, error: &str) -> Element<'_, DailyRecommendMessage> {
        column![
            text(Self::title()).size(32),
            text(error.to_string())
                .size(16)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.8, 0.3, 0.3)),
                }),
            button("重试").on_press(DailyRecommendMessage::FetchRecommendations),
        ]
        .spacing(10)
        .into()
    }

    fn view_empty(&self) -> Element<'_, DailyRecommendMessage> {
        column![
            text(Self::title()).size(32),
            text(Self::description()).size(16),
            button("获取推荐").on_press(DailyRecommendMessage::FetchRecommendations),
        ]
        .spacing(10)
        .into()
    }

    /// 计算当前可见的卡片索引范围
    fn calculate_visible_range(&self) -> (usize, usize) {
        const CARD_HEIGHT: f32 = 260.0; // 160 cover + ~100 info
        const SPACING: f32 = 20.0;      // 卡片间距
        const BUFFER_ROWS: usize = 2;   // 预加载 2 行作为缓冲

        // 如果还没有视口信息，返回初始范围（前 15 个）
        let viewport = match &self.viewport {
            Some(v) => v,
            None => return (0, self.playlists.len().min(15)),
        };

        let bounds = viewport.bounds();
        let relative_y = viewport.relative_offset().y;

        // 计算可见的行范围
        let start_y = relative_y * bounds.height;
        let visible_height = bounds.height;

        let start_row = (start_y / (CARD_HEIGHT + SPACING)).floor() as usize;
        let rows_visible = (visible_height / (CARD_HEIGHT + SPACING)).ceil() as usize;
        let end_row = start_row + rows_visible + BUFFER_ROWS;

        // 根据列数转换为卡片索引
        let available_width = self.window_size.width - 200.0 - 80.0;
        let columns = ((available_width) / 200.0).floor().max(1.0) as usize;

        let start_idx = start_row.saturating_sub(BUFFER_ROWS) * columns;
        let end_idx = (end_row * columns).min(self.playlists.len());

        (start_idx, end_idx)
    }

    fn view_playlist_list(&self) -> Element<'_, DailyRecommendMessage> {
        // 计算可见范围
        let (start_idx, end_idx) = self.calculate_visible_range();

        // 只为可见卡片创建 widget
        let cards: Vec<Element<DailyRecommendMessage>> = self
            .playlists
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(end_idx - start_idx)
            .map(|(_idx, playlist)| {
                let card_data = PlaylistCardData::from(playlist);
                let card_element = create_playlist_card::<DailyRecommendMessage>(card_data.clone());

                button(card_element)
                    .padding(0)
                    .style(|_theme, status| iced::widget::button::Style {
                        background: match status {
                            iced::widget::button::Status::Hovered => {
                                Some(iced::Background::Color(iced::Color::from_rgba(
                                    0.3, 0.6, 1.0, 0.12,
                                )))
                            }
                            _ => None,
                        },
                        border: iced::border::Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: 16.0.into(),
                        },
                        ..Default::default()
                    })
                    .on_press(DailyRecommendMessage::NavigatePlaylist(card_data.id))
                    .into()
            })
            .collect();

        // 计算顶部占位高度（为不可见的卡片留出空间）
        let available_width = self.window_size.width - 200.0 - 80.0;
        let columns = ((available_width) / 200.0).floor().max(1.0) as usize;
        let row_height = 260.0 + 20.0; // 卡片高度 + 间距
        let top_spacing = ((start_idx / columns) as f32) * row_height;

        // 构建带占位的滚动内容
        let scrollable_content = column![
            container(text(""))
                .height(Length::Fixed(top_spacing))
                .width(Length::Fill),
            container(crate::ui::responsive_grid(cards, 180.0, 20, available_width))
                .padding(20)
                .width(Length::Fill),
        ];

        column![
            text(Self::title()).size(32),
            scrollable(scrollable_content)
                .on_scroll(|viewport| DailyRecommendMessage::ViewportChanged(viewport))
                .height(Length::Fill),
        ]
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

}
