//! 页面组件
//!
//! 包含主窗口右侧内容区的不同页面组件。

pub mod recommend;
pub mod discover;
pub mod collection;
pub mod favorites;
pub mod playlist_detail;

pub use recommend::Recommend;
pub use discover::{Discover, DiscoverOutput};
pub use collection::Collection;
pub use favorites::Favorites;

pub use playlist_detail::PlaylistDetail;
