use netease_cloud_music_api::MusicApi;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import models from our crate
use crate::models::{Song, Artist, Album, PlaylistDetail};

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

    /// 获取歌单详情（包含歌曲）
    pub async fn get_playlist_detail(&self, id: u64) -> anyhow::Result<PlaylistDetail> {
        let api = self.api.lock().await;

        // 使用 song_list_detail API
        let detail = api.song_list_detail(id).await?;

        let songs: Vec<Song> = detail
            .songs
            .into_iter()
            .map(|s| Song {
                id: s.id,
                name: s.name,
                artists: vec![Artist {
                    id: 0, // singer may not have id
                    name: s.singer,
                }],
                album: Album {
                    id: s.album_id,
                    name: s.album,
                },
                duration: s.duration,
                size: None, // API doesn't provide size in list detail
                copyright_id: 0, // TODO: Handle SongCopyright type
                cover_url: s.pic_url, // 使用歌单封面作为默认封面
            })
            .collect();

        Ok(PlaylistDetail {
            id: detail.id,
            name: detail.name,
            cover_url: detail.cover_img_url,
            description: detail.description,
            songs,
        })
    }

    /// 获取歌曲详情
    pub async fn get_songs_detail(&self, ids: &[u64]) -> anyhow::Result<Vec<Song>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let api = self.api.lock().await;
        let songs_info = api.songs_detail(ids).await?;

        Ok(songs_info
            .into_iter()
            .map(|s| Song {
                id: s.id,
                name: s.name,
                artists: vec![Artist {
                    id: 0, // singer may not have id
                    name: s.singer,
                }],
                album: Album {
                    id: s.album_id,
                    name: s.album,
                },
                duration: s.duration,
                size: None, // API doesn't provide size in detail
                copyright_id: 0, // TODO: Handle SongCopyright type
                cover_url: String::new(), // Album pic not directly available
            })
            .collect())
    }
}

impl Default for NcmApi {
    fn default() -> Self {
        Self::new()
    }
}
