use crate::api::NcmApi;
use crate::models::{PlaylistDetail, Song, SortField, SortOrder};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 歌曲服务 - 业务逻辑层（带缓存）
pub struct SongService {
    api: Arc<NcmApi>,
    cache: Arc<RwLock<HashMap<u64, PlaylistDetail>>>,
}

impl SongService {
    /// 创建新的歌曲服务
    pub fn new(api: Arc<NcmApi>) -> Self {
        Self {
            api,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取歌单歌曲（带缓存）
    pub async fn get_playlist_songs(
        &self,
        playlist_id: u64,
    ) -> anyhow::Result<PlaylistDetail> {
        // 先检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(detail) = cache.get(&playlist_id) {
                return Ok(detail.clone());
            }
        }

        // 缓存未命中，从 API 获取
        let detail = self.api.get_playlist_detail(playlist_id).await?;

        // 写入缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(playlist_id, detail.clone());
        }

        Ok(detail)
    }

    /// 清除指定歌单的缓存
    pub async fn clear_cache(&self, playlist_id: u64) {
        let mut cache = self.cache.write().await;
        cache.remove(&playlist_id);
    }

    /// 清除所有缓存
    pub async fn clear_all_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 本地排序歌曲（原地排序）
    pub fn sort_songs(songs: &mut [Song], field: SortField, order: SortOrder) {
        if field == SortField::Default || order == SortOrder::Default {
            return; // 默认排序，不做处理
        }

        songs.sort_by(|a, b| {
            let cmp = match field {
                SortField::Title => {
                    // 按歌名排序（支持拼音）
                    a.name.cmp(&b.name)
                }
                SortField::Artist => {
                    // 按歌手名称排序
                    let a_artists = a.artists_string();
                    let b_artists = b.artists_string();
                    a_artists.cmp(&b_artists)
                }
                SortField::Album => {
                    // 按专辑名称排序
                    a.album.name.cmp(&b.album.name)
                }
                SortField::Duration => {
                    // 按时长排序
                    a.duration.cmp(&b.duration)
                }
                SortField::Size => {
                    // 按文件大小排序
                    a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0))
                }
                SortField::CreateTime => {
                    // 按创建时间排序（暂无此字段，使用 ID 替代）
                    a.id.cmp(&b.id)
                }
                SortField::Default => {
                    std::cmp::Ordering::Equal
                }
            };

            match order {
                SortOrder::Asc => cmp,
                SortOrder::Desc => cmp.reverse(),
                SortOrder::Default => std::cmp::Ordering::Equal,
            }
        });
    }

    /// 排序歌单详情的歌曲（返回新的排序后的详情）
    pub fn sort_playlist_detail(
        detail: &PlaylistDetail,
        field: SortField,
        order: SortOrder,
    ) -> PlaylistDetail {
        let mut sorted_detail = detail.clone();
        Self::sort_songs(&mut sorted_detail.songs, field, order);
        sorted_detail
    }
}
