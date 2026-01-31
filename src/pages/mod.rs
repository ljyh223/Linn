pub mod page;
pub mod daily_recommend;
pub mod discover;
pub mod liked_songs;
pub mod favorites;
pub mod playlist_songs;

pub use page::Page;

// 导出页面 struct 和消息
pub use daily_recommend::{DailyRecommendPage, DailyRecommendMessage};
pub use discover::DiscoverPage;
pub use liked_songs::LikedSongsPage;
pub use favorites::FavoritesPage;
pub use playlist_songs::{PlaylistSongsPage, PlaylistSongsMessage};

use crate::ui::components::Icons; 
