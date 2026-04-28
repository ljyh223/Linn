use std::sync::OnceLock;
use std::time::Duration;

use moka::future::Cache;
use ncm_api_rs::Query;

use crate::api::{Album, Artist, Song, SoundQuality, client::client};

static URL_CACHE: OnceLock<Cache<(u64, String), String>> = OnceLock::new();

fn url_cache() -> &'static Cache<(u64, String), String> {
    URL_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(500)
            .time_to_idle(Duration::from_secs(25 * 60))
            .build()
    })
}

pub async fn get_song_url(id: u64, quality: SoundQuality) -> anyhow::Result<String> {
    let key = (id, quality.to_string());
    if let Some(url) = url_cache().get(&key).await {
        return Ok(url);
    }

    let query = Query::new()
        .param("id", &id.to_string())
        .param("level", &quality.to_string());

    match client().song_url_v1(&query).await {
        Ok(resp) => {
            if let Some(url) = resp.body["data"][0]["url"].as_str() {
                let url = url.to_string();
                url_cache().insert(key, url.clone()).await;
                Ok(url)
            } else {
                Err(anyhow::anyhow!("未找到歌曲URL"))
            }
        }
        Err(e) => {
            eprintln!("获取歌曲URL失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn get_song_detail(ids: Vec<u64>) -> anyhow::Result<Vec<Song>> {
    let query = Query::new().param(
        "ids",
        ids.iter()
            .map(u64::to_string)
            .collect::<Vec<String>>()
            .join(", ")
            .as_str(),
    );

    match client().song_detail(&query).await {
        Ok(resp) => {
            let songs = resp.body["songs"].as_array().unwrap();
            let mut song_lsit = Vec::new();
            for song in songs {
                song_lsit.push(Song {
                    id: song["id"].as_u64().unwrap_or(0),
                    name: song["name"].as_str().unwrap_or("").to_string(),
                    cover_url: song["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    artists: song["ar"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|artist| Artist {
                            id: artist["id"].as_u64().unwrap_or(0),
                            name: artist["name"].as_str().unwrap_or("").to_string(),
                            avatar: None,
                        })
                        .collect::<Vec<Artist>>(),
                    album: Album {
                        id: song["al"]["id"].as_u64().unwrap_or(0),
                        name: song["al"]["name"].as_str().unwrap_or("").to_string(),
                        cover_url: song["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    },
                    duration: song["dt"].as_u64().unwrap_or(0),
                });
            }
            Ok(song_lsit)
        }
        Err(e) => {
            eprintln!("获取歌曲URL失败: {}", e);
            return Err(e.into());
        }
    }
}

