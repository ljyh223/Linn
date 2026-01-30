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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_songs() -> Vec<Song> {
        vec![
            Song {
                id: 1,
                name: "Zulu".to_string(),
                artists: vec![Artist { id: 1, name: "Artist Z".to_string() }],
                album: Album { id: 1, name: "Album B".to_string() },
                duration: 300000,
                size: Some(5000000),
                copyright_id: 0,
                cover_url: String::new(),
            },
            Song {
                id: 2,
                name: "Alpha".to_string(),
                artists: vec![Artist { id: 2, name: "Artist A".to_string() }],
                album: Album { id: 2, name: "Album A".to_string() },
                duration: 180000,
                size: Some(3000000),
                copyright_id: 0,
                cover_url: String::new(),
            },
            Song {
                id: 3,
                name: "Beta".to_string(),
                artists: vec![Artist { id: 3, name: "Artist B".to_string() }],
                album: Album { id: 3, name: "Album C".to_string() },
                duration: 240000,
                size: Some(4000000),
                copyright_id: 0,
                cover_url: String::new(),
            },
        ]
    }

    #[test]
    fn test_sort_by_title_asc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Title, SortOrder::Asc);
        assert_eq!(songs[0].name, "Alpha");
        assert_eq!(songs[1].name, "Beta");
        assert_eq!(songs[2].name, "Zulu");
    }

    #[test]
    fn test_sort_by_title_desc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Title, SortOrder::Desc);
        assert_eq!(songs[0].name, "Zulu");
        assert_eq!(songs[1].name, "Beta");
        assert_eq!(songs[2].name, "Alpha");
    }

    #[test]
    fn test_sort_by_artist_asc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Artist, SortOrder::Asc);
        assert_eq!(songs[0].artists[0].name, "Artist A");
        assert_eq!(songs[1].artists[0].name, "Artist B");
        assert_eq!(songs[2].artists[0].name, "Artist Z");
    }

    #[test]
    fn test_sort_by_album_asc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Album, SortOrder::Asc);
        assert_eq!(songs[0].album.name, "Album A");
        assert_eq!(songs[1].album.name, "Album B");
        assert_eq!(songs[2].album.name, "Album C");
    }

    #[test]
    fn test_sort_by_duration_asc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Duration, SortOrder::Asc);
        assert_eq!(songs[0].duration, 180000);
        assert_eq!(songs[1].duration, 240000);
        assert_eq!(songs[2].duration, 300000);
    }

    #[test]
    fn test_sort_by_duration_desc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Duration, SortOrder::Desc);
        assert!(songs[0].duration > songs[1].duration);
        assert!(songs[1].duration > songs[2].duration);
    }

    #[test]
    fn test_sort_by_size_asc() {
        let mut songs = create_test_songs();
        SongService::sort_songs(&mut songs, SortField::Size, SortOrder::Asc);
        assert_eq!(songs[0].size, Some(3000000));
        assert_eq!(songs[1].size, Some(4000000));
        assert_eq!(songs[2].size, Some(5000000));
    }

    #[test]
    fn test_sort_default_does_nothing() {
        let mut songs = create_test_songs();
        let original_order: Vec<u64> = songs.iter().map(|s| s.id).collect();

        SongService::sort_songs(&mut songs, SortField::Default, SortOrder::Asc);

        let new_order: Vec<u64> = songs.iter().map(|s| s.id).collect();
        assert_eq!(original_order, new_order);
    }

    #[test]
    fn test_sort_order_default_does_nothing() {
        let mut songs = create_test_songs();
        let original_order: Vec<u64> = songs.iter().map(|s| s.id).collect();

        SongService::sort_songs(&mut songs, SortField::Title, SortOrder::Default);

        let new_order: Vec<u64> = songs.iter().map(|s| s.id).collect();
        assert_eq!(original_order, new_order);
    }
}
