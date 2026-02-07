//! 可复用的 UI 组件
//!
//! 包含应用中可复用的 UI 组件，如导航栏、播放栏等。

pub mod list_detail;
pub mod navigation;
pub mod player_bar;

pub use async_image::AsyncImage;
pub use list_detail::{
    ListDetail, ListDetailMsg, ListDetailOutput, DetailData, DetailConfig, DetailType, DetailTab,
};
pub use navigation::{Navigation, NavigationOutput};
pub use player_bar::{PlayerBar, PlayerBarOutput};
