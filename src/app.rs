use crate::models::Playlist;
use crate::pages::Page;
use crate::services::ImageCache;
use crate::services::PlaylistService;
use crate::ui::{responsive_grid, AsyncImage, Content, PlaylistCardData, Sidebar};
use crate::utils::ImageSize;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Fill, Length, Subscription, Task};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
    FetchRecommendations,
    RecommendationsFetched(Result<Vec<Playlist>, String>),
    ImageLoaded(String, Result<iced::widget::image::Handle, String>),
    WindowResized(iced::Size),
}

pub struct App {
    current_page: Page,
    sidebar: Sidebar,
    content: Content,
    playlist_service: Arc<PlaylistService>,
    image_cache: ImageCache,
    playlists: Vec<Playlist>,
    playlist_images: HashMap<String, AsyncImage>,
    is_loading: bool,
    error_message: Option<String>,
    window_size: iced::Size,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let current_page = Page::DailyRecommend;
        let api = Arc::new(crate::api::NcmApi::default());
        let playlist_service = Arc::new(PlaylistService::new(api));

        let mut app = Self {
            current_page,
            sidebar: Sidebar::new(current_page),
            content: Content::new(current_page),
            playlist_service,
            image_cache: ImageCache::default(),
            playlists: Vec::new(),
            playlist_images: HashMap::new(),
            is_loading: false,
            error_message: None,
            window_size: iced::Size::new(1200.0, 800.0),
        };

        // 自动加载推荐歌单
        let task = app.fetch_recommendations();

        (app, task)
    }
}

impl Default for App {
    fn default() -> Self {
        let current_page = Page::DailyRecommend;
        let api = Arc::new(crate::api::NcmApi::default());
        let playlist_service = Arc::new(PlaylistService::new(api));

        Self {
            current_page,
            sidebar: Sidebar::new(current_page),
            content: Content::new(current_page),
            playlist_service,
            image_cache: ImageCache::default(),
            playlists: Vec::new(),
            playlist_images: HashMap::new(),
            is_loading: false,
            error_message: None,
            window_size: iced::Size::new(1200.0, 800.0),
        }
    }
}

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(page) => {
                self.current_page = page;
                self.sidebar = crate::ui::Sidebar::new(page);
                self.content = crate::ui::Content::new(page);

                if page == Page::DailyRecommend && self.playlists.is_empty() {
                    return self.fetch_recommendations();
                }

                Task::none()
            }

            Message::FetchRecommendations => {
                self.is_loading = true;
                let service = self.playlist_service.clone();

                Task::perform(
                    async move {
                        service
                            .get_recommendations()
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::RecommendationsFetched,
                )
            }

            Message::RecommendationsFetched(result) => {
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

            Message::ImageLoaded(url, result) => {
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

            Message::WindowResized(size) => {
                self.window_size = size;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let sidebar = self.sidebar.view();

        let content = match self.current_page {
            Page::DailyRecommend => self.view_daily_recommend(),
            _ => self.content.view(),
        };

        row![sidebar, content].width(Fill).height(Fill).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| {
            match event {
                iced::Event::Window(iced::window::Event::Resized(size)) => {
                    Some(Message::WindowResized(size))
                }
                _ => None,
            }
        })
    }

    fn view_daily_recommend(&self) -> Element<Message> {
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
            .width(Fill)
            .height(Fill)
            .padding(40)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.08, 0.08, 0.1,
                ))),
                ..Default::default()
            })
            .into()
    }

    fn view_loading(&self) -> Element<Message> {
        column![
            text("为我推荐").size(32),
            text("正在获取推荐歌单...").size(16),
        ]
        .spacing(10)
        .into()
    }

    fn view_error(&self, error: &str) -> Element<Message> {
        column![
            text("为我推荐").size(32),
            text(error.to_string())
                .size(16)
                .style(|_theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.8, 0.3, 0.3)),
                }),
            iced::widget::button("重试").on_press(Message::FetchRecommendations),
        ]
        .spacing(10)
        .into()
    }

    fn view_empty(&self) -> Element<Message> {
        column![
            text("为我推荐").size(32),
            text("每日推荐歌单，根据你的音乐口味量身定制").size(16),
            iced::widget::button("获取推荐").on_press(Message::FetchRecommendations),
        ]
        .spacing(10)
        .into()
    }

    fn view_playlist_list(&self) -> Element<Message> {
        // 直接创建卡片UI，避免生命周期问题
        let cards: Vec<Element<Message>> = self
            .playlists
            .iter()
            .map(|playlist| {
                let card_data = PlaylistCardData::from(playlist);
                let image_state = self
                    .playlist_images
                    .get(&playlist.cover_url)
                    .cloned()
                    .unwrap_or_else(AsyncImage::loading);

                // 直接创建卡片UI
                let card_element = crate::ui::create_playlist_card(card_data, image_state);
                card_element.map(|_| Message::Navigate(Page::DailyRecommend))
            })
            .collect();

        // Calculate available width (subtract sidebar width and padding)
        let sidebar_width = 200.0;
        let padding = 80.0; // 40px padding on each side
        let available_width = self.window_size.width - sidebar_width - padding;

        column![
            text("为我推荐").size(32),
            scrollable(
                container(responsive_grid(cards, 200.0, 20, available_width))
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

    fn fetch_recommendations(&mut self) -> Task<Message> {
        self.is_loading = true;
        let service = self.playlist_service.clone();

        Task::perform(
            async move {
                service
                    .get_recommendations()
                    .await
                    .map_err(|e| e.to_string())
            },
            Message::RecommendationsFetched,
        )
    }

    fn load_playlist_images(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();

        for playlist in &self.playlists {
            let url = playlist.cover_url.clone();
            let cache = self.image_cache.clone();

            if !self.playlist_images.contains_key(&url) {
                self.playlist_images
                    .insert(url.clone(), AsyncImage::loading());

                // 为卡片封面使用合适的图片尺寸 (160px 显示 -> 200x200 加载)
                let sized_url = ImageSize::Small.apply_to_url(&url);

                tasks.push(Task::perform(
                    load_image_from_url(cache, sized_url),
                    move |result| Message::ImageLoaded(url, result),
                ));
            }
        }

        Task::batch(tasks)
    }
}

/// 从 URL 加载图片（支持网易云的尺寸参数优化）
async fn load_image_from_url(
    cache: ImageCache,
    sized_url: String,
) -> Result<iced::widget::image::Handle, String> {
    // 先尝试从缓存加载
    if let Ok(Some(data)) = cache.load_from_cache(&sized_url).await {
        // Apply rounded corners (16px)
        let rounded_data = crate::services::ImageCache::apply_rounded_corners(data, 16)
            .map_err(|e: anyhow::Error| e.to_string())?;
        let handle = iced::widget::image::Handle::from_bytes(rounded_data);
        return Ok(handle);
    }

    // 下载并缓存
    match cache.download_and_cache(&sized_url).await {
        Ok(data) => {
            // Apply rounded corners (16px)
            let rounded_data = crate::services::ImageCache::apply_rounded_corners(data, 16)
                .map_err(|e: anyhow::Error| e.to_string())?;
            let handle = iced::widget::image::Handle::from_bytes(rounded_data);
            Ok(handle)
        }
        Err(e) => Err(e.to_string()),
    }
}
