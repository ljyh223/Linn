use crate::api::NcmApi;
use crate::models::Playlist;
use std::sync::Arc;

/// 歌单服务 - 业务逻辑层
pub struct PlaylistService {
    api: Arc<NcmApi>,
}

impl PlaylistService {
    pub fn new(api: Arc<NcmApi>) -> Self {
        Self { api }
    }

    /// 获取推荐歌单（不需要登录）
    pub async fn get_recommendations(&self) -> anyhow::Result<Vec<Playlist>> {
        // 使用 top_song_list 获取热门推荐歌单
        self.api.get_hot_playlists("流行", "hot", 0, 20).await
    }

    /// 获取热门歌单（不需要登录）
    pub async fn get_hot_playlists(
        &self,
        category: &str,
        order: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Playlist>> {
        self.api.get_hot_playlists(category, order, offset, limit).await
    }
}
