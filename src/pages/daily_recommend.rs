use crate::models::Playlist;
use crate::services::ImageCache;
use crate::ui::{AsyncImage, PlaylistCardData};
use crate::utils::ImageSize;
use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length, Task};
use std::collections::HashMap;
use std::sync::Arc;

/// 每日推荐页面的消息
#[derive(Debug, Clone)]
pub enum DailyRecommendMessage {
    FetchRecommendations,
    RecommendationsFetched(Result<Vec<Playlist>, String>),
    ImageLoaded(String, Result<iced::widget::image::Handle, String>),
    NavigatePlaylist(u64), // 新增：点击歌单卡片导航
}

/// 每日推荐页面
pub struct DailyRecommendPage {
    playlist_service: Arc<crate::services::PlaylistService>,
    image_cache: ImageCache,
    playlists: Vec<Playlist>,
    playlist_images: HashMap<String, AsyncImage>,
    is_loading: bool,
    error_message: Option<String>,
    window_size: iced::Size,
}

impl DailyRecommendPage {
    /// 创建新的每日推荐页面
    pub fn new(
        playlist_service: Arc<crate::services::PlaylistService>,
        image_cache: ImageCache,
        window_size: iced::Size,
    ) -> Self {
        Self {
            playlist_service,
            image_cache,
            playlists: Vec::new(),
            playlist_images: HashMap::new(),
            is_loading: false,
            error_message: None,
            window_size,
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
                        return self.load_playlist_images();
                    }
                    Err(error) => {
                        self.error_message = Some(error);
                    }
                }

                Task::none()
            }

            DailyRecommendMessage::ImageLoaded(url, result) => {
                match result {
                    Ok(handle) => {
                        self.playlist_images.insert(url, AsyncImage::loaded(handle));
                    }
                    Err(_) => {
                        self.playlist_images.insert(url, AsyncImage::failed());
                    }
                }
                Task::none()
            }

            DailyRecommendMessage::NavigatePlaylist(_) => {
                // 这个消息由 App 层处理导航
                Task::none()
            }
        }
    }

    /// 渲染页面
    pub fn view(&self) -> Element<DailyRecommendMessage> {
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

    fn view_loading(&self) -> Element<DailyRecommendMessage> {
        column![
            text(Self::title()).size(32),
            text("正在获取推荐歌单...").size(16),
        ]
        .spacing(10)
        .into()
    }

    fn view_error(&self, error: &str) -> Element<DailyRecommendMessage> {
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

    fn view_empty(&self) -> Element<DailyRecommendMessage> {
        column![
            text(Self::title()).size(32),
            text(Self::description()).size(16),
            button("获取推荐").on_press(DailyRecommendMessage::FetchRecommendations),
        ]
        .spacing(10)
        .into()
    }

    fn view_playlist_list(&self) -> Element<DailyRecommendMessage> {
        // 直接创建卡片UI
        let cards: Vec<Element<DailyRecommendMessage>> = self
            .playlists
            .iter()
            .map(|playlist| {
                let card_data = PlaylistCardData::from(playlist);
                let image_state = self
                    .playlist_images
                    .get(&playlist.cover_url)
                    .cloned()
                    .unwrap_or_else(AsyncImage::loading);
                let playlist_id = playlist.id;

                // 创建可点击的卡片 - 直接用 button 包装
                iced::widget::button(
                    crate::ui::create_playlist_card(card_data, image_state)
                        .map(move |_| DailyRecommendMessage::FetchRecommendations)
                )
                    .padding(0)
                    .style(|_theme, _status| iced::widget::button::Style {
                        background: None,
                        border: iced::border::Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: 16.0.into(),
                        },
                        ..Default::default()
                    })
                    .on_press(DailyRecommendMessage::NavigatePlaylist(playlist_id))
                    .into()
            })
            .collect();

        // Calculate available width
        let sidebar_width = 200.0;
        let padding = 80.0;
        let available_width = self.window_size.width - sidebar_width - padding;

        column![
            text(Self::title()).size(32),
            scrollable(
                container(crate::ui::responsive_grid(cards, 200.0, 20, available_width))
                    .padding(20)
                    .width(Length::Fill)
            )
            .height(Length::Fill),
        ]
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn load_playlist_images(&mut self) -> Task<DailyRecommendMessage> {
        let mut tasks = Vec::new();

        for playlist in &self.playlists {
            let url = playlist.cover_url.clone();
            let cache = self.image_cache.clone();

            if !self.playlist_images.contains_key(&url) {
                self.playlist_images
                    .insert(url.clone(), AsyncImage::loading());

                // 为卡片封面使用合适的图片尺寸
                let sized_url = ImageSize::Small.apply_to_url(&url);

                tasks.push(Task::perform(
                    load_image_from_url(cache, sized_url),
                    move |result| DailyRecommendMessage::ImageLoaded(url, result),
                ));
            }
        }

        Task::batch(tasks)
    }
}

/// 从 URL 加载图片
async fn load_image_from_url(
    cache: ImageCache,
    sized_url: String,
) -> Result<iced::widget::image::Handle, String> {
    // 先尝试从缓存加载
    if let Ok(Some(data)) = cache.load_from_cache(&sized_url).await {
        let rounded_data = crate::services::ImageCache::apply_rounded_corners(data, 16)
            .map_err(|e: anyhow::Error| e.to_string())?;
        let handle = iced::widget::image::Handle::from_bytes(rounded_data);
        return Ok(handle);
    }

    // 下载并缓存
    match cache.download_and_cache(&sized_url).await {
        Ok(data) => {
            let rounded_data = crate::services::ImageCache::apply_rounded_corners(data, 16)
                .map_err(|e: anyhow::Error| e.to_string())?;
            let handle = iced::widget::image::Handle::from_bytes(rounded_data);
            Ok(handle)
        }
        Err(e) => Err(e.to_string()),
    }
}
