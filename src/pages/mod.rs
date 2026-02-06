//! 页面组件
//!
//! 包含主窗口右侧内容区的不同页面组件。

pub mod recommend;
pub mod discover;
pub mod collection;
pub mod favorites;

pub use recommend::Recommend;
pub use discover::Discover;
pub use collection::Collection;
pub use favorites::Favorites;
