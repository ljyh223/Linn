//! 网易云音乐 API 封装
//!
//! 基于 netease-cloud-music-api 封装的异步 API 客户端。

use netease_cloud_music_api::MusicApi;
use once_cell::sync::Lazy;

/// 全局网易云音乐 API 客户端
pub static NCM_API: Lazy<MusicApi> = Lazy::new(|| {
    MusicApi::default()
});

/// 网易云音乐 API 封装结构
pub struct NcmApi;

impl NcmApi {
    /// 创建新的 API 客户端
    pub fn new() -> MusicApi {
        MusicApi::default()
    }

    /// 获取全局 API 客户端
    pub fn global() -> &'static MusicApi {
        &NCM_API
    }
}

// TODO: 添加更多 API 封装方法
// - 获取推荐歌单
// - 获取歌单详情
// - 搜索音乐
// - 获取歌曲URL
// - 获取歌词
