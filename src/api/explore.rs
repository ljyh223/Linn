//! 发现页（Explore）相关接口

use ncm_api_rs::Query;

use crate::api::{Album, Artist, Mv, Playlist, Song, client::client};

fn parse_song(value: &serde_json::Value) -> Song {
    Song {
        id: value["id"].as_u64().unwrap_or(0),
        name: value["name"].as_str().unwrap_or("").to_string(),
        cover_url: value["al"]["picUrl"]
            .as_str()
            .or_else(|| value["album"]["picUrl"].as_str())
            .unwrap_or("")
            .to_string(),
        artists: value["ar"]
            .as_array()
            .filter(|arr| !arr.is_empty())
            .or_else(|| value["artists"].as_array())
            .map(|arr| {
                arr.iter()
                    .map(|a| Artist {
                        id: a["id"].as_u64().unwrap_or(0),
                        name: a["name"].as_str().unwrap_or("").to_string(),
                        avatar: None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        album: Album {
            id: value["al"]["id"]
                .as_u64()
                .or_else(|| value["album"]["id"].as_u64())
                .unwrap_or(0),
            name: value["al"]["name"]
                .as_str()
                .or_else(|| value["album"]["name"].as_str())
                .unwrap_or("")
                .to_string(),
            cover_url: value["al"]["picUrl"]
                .as_str()
                .or_else(|| value["album"]["picUrl"].as_str())
                .unwrap_or("")
                .to_string(),
        },
        duration: value["dt"]
            .as_u64()
            .or_else(|| value["duration"].as_u64())
            .unwrap_or(0),
    }
}

/// 排行榜列表（含更新频率），只保留经典全局榜
pub async fn get_toplist() -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new();
    match client().toplist_detail(&query).await {
        Ok(resp) => {
            let list = resp.body["list"].as_array().cloned().unwrap_or_default();
            let mut result = Vec::new();
            for item in &list {
                let id = item["id"].as_u64().unwrap_or(0);
                if id == 0 {
                    continue;
                }
                result.push(Playlist {
                    id,
                    name: item["name"].as_str().unwrap_or("").to_string(),
                    cover_url: item["coverImgUrl"]
                        .as_str()
                        .or_else(|| item["coverUrl"].as_str())
                        .unwrap_or("")
                        .to_string(),
                    creator_name: String::new(),
                    creator_id: 0,
                    description: item["updateFrequency"].as_str().unwrap_or("").to_string(),
                    play_count: 0,
                });
            }
            Ok(result)
        }
        Err(e) => {
            eprintln!("获取排行榜失败: {e}");
            Err(e.into())
        }
    }
}

/// 获取单个榜单的歌曲列表
pub async fn get_toplist_songs(id: u64) -> anyhow::Result<Vec<Song>> {
    let query = Query::new().param("id", &id.to_string());
    match client().top_list(&query).await {
        Ok(resp) => {
            let tracks = resp.body["playlist"]["tracks"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            Ok(tracks.iter().map(parse_song).collect())
        }
        Err(e) => {
            eprintln!("获取榜单歌曲失败: {e}");
            Err(e.into())
        }
    }
}

/// 新歌速递
pub async fn get_new_songs() -> anyhow::Result<Vec<Song>> {
    let query = Query::new().param("type", "0");
    match client().top_song(&query).await {
        Ok(resp) => {
            let songs = resp.body["data"].as_array().cloned().unwrap_or_default();
            Ok(songs.iter().map(parse_song).collect())
        }
        Err(e) => {
            eprintln!("获取新歌速递失败: {e}");
            Err(e.into())
        }
    }
}

/// 新碟上架
pub async fn get_new_albums() -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new().param("area", "ALL").param("limit", "50");
    match client().top_album(&query).await {
        Ok(resp) => {
            let albums = resp.body["monthData"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let mut result = Vec::new();
            for item in &albums {
                let id = item["id"].as_u64().unwrap_or(0);
                if id == 0 {
                    continue;
                }
                result.push(Playlist {
                    id,
                    name: item["name"].as_str().unwrap_or("").to_string(),
                    cover_url: item["picUrl"].as_str().unwrap_or("").to_string(),
                    creator_name: item["artist"]["name"].as_str().unwrap_or("").to_string(),
                    creator_id: 0,
                    description: String::new(),
                    play_count: 0,
                });
            }
            Ok(result)
        }
        Err(e) => {
            eprintln!("获取新碟上架失败: {e}");
            Err(e.into())
        }
    }
}

/// 最新 MV
pub async fn get_new_mvs() -> anyhow::Result<Vec<Mv>> {
    let query = Query::new().param("limit", "12");
    match client().mv_first(&query).await {
        Ok(resp) => {
            let list = resp.body["data"].as_array().cloned().unwrap_or_default();
            let mut result = Vec::new();
            for item in &list {
                let id = item["id"].as_u64().unwrap_or(0);
                if id == 0 {
                    continue;
                }
                result.push(Mv {
                    id,
                    name: item["name"].as_str().unwrap_or("").to_string(),
                    cover: item["cover"].as_str().unwrap_or("").to_string(),
                    duration: item["duration"].as_u64().unwrap_or(0),
                    play_count: item["playCount"].as_u64().unwrap_or(0),
                });
            }
            Ok(result)
        }
        Err(e) => {
            eprintln!("获取最新 MV 失败: {e}");
            Err(e.into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::init_client;
    use relm4::gtk::gio::prelude::SettingsExt;

    #[tokio::test]
    async fn test_explore_apis() {
        use crate::APPLICATION_ID;
        unsafe {
            let _ = std::process::Command::new("glib-compile-schemas")
                .arg("data")
                .status();
            std::env::set_var("GSETTINGS_SCHEMA_DIR", "data");
        }
        let settings = relm4::gtk::gio::Settings::new(APPLICATION_ID);
        init_client(settings.string("cookie").to_string());

        let toplists = get_toplist().await.unwrap();
        println!("toplists={}", toplists.len());
        assert!(!toplists.is_empty(), "toplist 解析为空");
        assert!(toplists[0].cover_url.contains("http"));

        let songs = get_toplist_songs(toplists[0].id).await.unwrap();
        println!(
            "toplist_songs={} first={:?}",
            songs.len(),
            songs.first().map(|s| (&s.name, &s.cover_url))
        );
        assert!(!songs.is_empty());

        let news = get_new_songs().await.unwrap();
        println!(
            "new_songs={} first={:?}",
            news.len(),
            news.first().map(|s| (&s.name, &s.cover_url))
        );
        assert!(!news.is_empty());
        assert!(news[0].cover_url.contains("http"));

        let albums = get_new_albums().await.unwrap();
        println!(
            "new_albums={} first={:?}",
            albums.len(),
            albums
                .first()
                .map(|a| (&a.name, &a.creator_name, &a.cover_url))
        );
        assert!(!albums.is_empty());

        let mvs = get_new_mvs().await.unwrap();
        println!(
            "new_mvs={} first={:?}",
            mvs.len(),
            mvs.first().map(|m| (&m.name, m.play_count))
        );
        assert!(!mvs.is_empty());
    }
}
