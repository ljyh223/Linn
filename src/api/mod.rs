//! API 模块
//!
//! 封装与网易云音乐 API 的交互。

pub mod client;
pub mod model;

pub use client::{init_client, get_recommend_playlist, get_playlist_detail, get_song_url, get_song_detail};
pub use model::{Playlist, PlaylistDetail, Song, Artist, Album, SoundQuality};