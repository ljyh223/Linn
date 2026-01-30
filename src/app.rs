use crate::pages::{
    DailyRecommendPage, DailyRecommendMessage, DiscoverPage, FavoritesPage, LikedSongsPage,
    Page, PlaylistSongsPage, PlaylistSongsMessage,
};
use crate::services::{ImageCache, PlaylistService, SongService};
use crate::ui::{Content, Sidebar};
use iced::{Element, Subscription, Task};
use std::sync::Arc;

/// 应用消息
#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
    WindowResized(iced::Size),
    // 每日推荐页面消息
    DailyRecommend(DailyRecommendMessage),
    // 歌单详情页面消息
    PlaylistSongs(PlaylistSongsMessage),
}

/// 主应用结构
pub struct App {
    current_page: Page,
    sidebar: Sidebar,
    content: Content,

    // 服务
    playlist_service: Arc<PlaylistService>,
    song_service: Arc<SongService>,
    image_cache: ImageCache,

    // 页面实例
    daily_recommend_page: DailyRecommendPage,
    discover_page: DiscoverPage,
    liked_songs_page: LikedSongsPage,
    favorites_page: FavoritesPage,
    playlist_songs_page: PlaylistSongsPage,

    // 窗口大小
    window_size: iced::Size,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let current_page = Page::DailyRecommend;
        let api = Arc::new(crate::api::NcmApi::default());
        let playlist_service = Arc::new(PlaylistService::new(api.clone()));
        let song_service = Arc::new(SongService::new(api));
        let image_cache = ImageCache::default();
        let window_size = iced::Size::new(1200.0, 800.0);

        let daily_recommend_page =
            DailyRecommendPage::new(playlist_service.clone(), image_cache.clone(), window_size);
        let discover_page = DiscoverPage::new();
        let liked_songs_page = LikedSongsPage::new();
        let favorites_page = FavoritesPage::new();
        let playlist_songs_page = PlaylistSongsPage::new(song_service.clone(), image_cache.clone(), window_size);

        let mut app = Self {
            current_page,
            sidebar: Sidebar::new(current_page),
            content: Content::new(current_page),
            playlist_service,
            song_service,
            image_cache,
            daily_recommend_page,
            discover_page,
            liked_songs_page,
            favorites_page,
            playlist_songs_page,
            window_size,
        };

        // 自动加载推荐歌单（仅首次）
        let task = app.fetch_recommendations_if_needed();

        (app, task)
    }

    /// 获取推荐歌单（仅在数据未加载时）
    fn fetch_recommendations_if_needed(&mut self) -> Task<Message> {
        if !self.daily_recommend_page.is_data_loaded() {
            self.daily_recommend_page
                .fetch_recommendations()
                .map(Message::DailyRecommend)
        } else {
            Task::none()
        }
    }

    /// 获取推荐歌单
    fn fetch_recommendations(&mut self) -> Task<Message> {
        self.daily_recommend_page
            .fetch_recommendations()
            .map(Message::DailyRecommend)
    }
}

impl Default for App {
    fn default() -> Self {
        let current_page = Page::DailyRecommend;
        let api = Arc::new(crate::api::NcmApi::default());
        let playlist_service = Arc::new(PlaylistService::new(api.clone()));
        let song_service = Arc::new(SongService::new(api));
        let image_cache = ImageCache::default();
        let window_size = iced::Size::new(1200.0, 800.0);

        Self {
            current_page,
            sidebar: Sidebar::new(current_page),
            content: Content::new(current_page),
            playlist_service: playlist_service.clone(),
            song_service: song_service.clone(),
            image_cache: image_cache.clone(),
            daily_recommend_page: DailyRecommendPage::new(
                playlist_service,
                image_cache.clone(),
                window_size,
            ),
            discover_page: DiscoverPage::new(),
            liked_songs_page: LikedSongsPage::new(),
            favorites_page: FavoritesPage::new(),
            playlist_songs_page: PlaylistSongsPage::new(song_service, image_cache.clone(), window_size.clone()),
            window_size,
        }
    }
}

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(page) => {
                // 处理歌单导航
                if let Page::PlaylistDetail(playlist_id) = page {
                    // 先更新页面状态
                    self.current_page = Page::PlaylistDetail(playlist_id);
                    self.sidebar = crate::ui::Sidebar::new(Page::PlaylistDetail(playlist_id));
                    self.content = crate::ui::Content::new(Page::PlaylistDetail(playlist_id));

                    // 返回获取歌曲的任务
                    return self
                        .playlist_songs_page
                        .fetch_songs(playlist_id)
                        .map(Message::PlaylistSongs);
                }

                self.current_page = page;
                self.sidebar = crate::ui::Sidebar::new(page);
                self.content = crate::ui::Content::new(page);

                // 如果导航到每日推荐且数据未加载，自动加载
                if page == Page::DailyRecommend {
                    return self.fetch_recommendations_if_needed();
                }

                Task::none()
            }

            Message::WindowResized(size) => {
                self.window_size = size;
                self.daily_recommend_page.set_window_size(size);
                self.playlist_songs_page.set_window_size(size);
                Task::none()
            }

            Message::DailyRecommend(msg) => {
                // 检查是否是导航消息
                if let crate::pages::DailyRecommendMessage::NavigatePlaylist(playlist_id) = msg {
                    return Task::done(Message::Navigate(Page::PlaylistDetail(playlist_id)));
                }

                self.daily_recommend_page
                    .update(msg)
                    .map(Message::DailyRecommend)
            }

            Message::PlaylistSongs(msg) => {
                self.playlist_songs_page
                    .update(msg)
                    .map(Message::PlaylistSongs)
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let sidebar = self.sidebar.view();

        let content = match self.current_page {
            Page::DailyRecommend => self
                .daily_recommend_page
                .view()
                .map(Message::DailyRecommend),
            Page::Discover => self
                .discover_page
                .view()
                .map(|_| Message::Navigate(Page::Discover)),
            Page::LikedSongs => self
                .liked_songs_page
                .view()
                .map(|_| Message::Navigate(Page::LikedSongs)),
            Page::Favorites => self
                .favorites_page
                .view()
                .map(|_| Message::Navigate(Page::Favorites)),
            Page::PlaylistDetail(_) => self
                .playlist_songs_page
                .view()
                .map(Message::PlaylistSongs),
        };

        iced::widget::row![sidebar, content]
            .width(iced::Fill)
            .height(iced::Fill)
            .into()
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
}
