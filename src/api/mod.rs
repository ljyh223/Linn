//! API 模块
//!
//! 封装与网易云音乐 API 的交互。

pub mod client;
pub mod model;

pub use client::*;
pub use model::*;