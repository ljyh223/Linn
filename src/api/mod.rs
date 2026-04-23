//! API 模块
//!
//! 封装与网易云音乐 API 的交互。

pub mod client;
pub mod model;
pub mod utils;
pub mod user;
pub mod playlist;
pub mod song;
pub mod album;
pub mod artist;
pub mod recommend;


pub use client::init_client;
use client::client;
pub use model::*;
pub use utils::*;
pub use user::*;
pub use playlist::*;
pub use song::*;
pub use album::*;
pub use artist::*;
pub use recommend::*;