use netease_cloud_music_api::MusicApi;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 网易云音乐 API 封装
#[derive(Clone)]
pub struct NcmApi {
    api: Arc<Mutex<MusicApi>>,
}

impl NcmApi {
    pub fn new() -> Self {
        Self {
            api: Arc::new(Mutex::new(MusicApi::default())),
        }
    }

    /// 获取热门歌单（不需要登录）
    pub async fn get_hot_playlists(
        &self,
        category: &str,
        order: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<crate::models::Playlist>> {
        let api = self.api.lock().await;
        let songlists = api
            .top_song_list(category, order, offset as u16, limit as u16)
            .await?;

        let playlists = songlists
            .into_iter()
            .map(|sl| crate::models::Playlist {
                id: sl.id,
                name: sl.name,
                cover_url: sl.cover_img_url,
                creator: sl.author,
            })
            .collect();

        Ok(playlists)
    }

    /// 获取每日推荐歌单（需要登录）
    pub async fn recommend_resource(&self) -> anyhow::Result<Vec<crate::models::Playlist>> {
        let api = self.api.lock().await;
        let songlists = api.recommend_resource().await?;

        let playlists = songlists
            .into_iter()
            .map(|sl| crate::models::Playlist {
                id: sl.id,
                name: sl.name,
                cover_url: sl.cover_img_url,
                creator: sl.author,
            })
            .collect();

        Ok(playlists)
    }

    /// 登录
    pub async fn login(&self, email: String, password: String) -> anyhow::Result<bool> {
        let api = self.api.lock().await;
        let result = api.login(email, password).await?;
        Ok(result.code == 200)
    }
}

impl Default for NcmApi {
    fn default() -> Self {
        Self::new()
    }
}
