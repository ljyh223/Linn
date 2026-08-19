//! API 模块
//!
//! 封装与网易云音乐 API 的交互。

pub mod album;
pub mod amll;
pub mod artist;
pub mod client;
pub mod comment;
pub mod custom_api;
pub mod explore;
pub mod lyric;
pub mod model;
pub mod mv;
pub mod playlist;
pub mod qqmusic;
pub mod recommend;
pub mod search;
pub mod song;
pub mod user;
pub mod utils;

pub use album::*;
pub use artist::*;
pub use client::init_client;
pub use comment::*;
pub use custom_api::*;
pub use explore::*;
pub use lyric::*;
pub use model::*;
pub use mv::*;
pub use playlist::*;
pub use recommend::*;
pub use search::*;
pub use song::*;
pub use user::*;
