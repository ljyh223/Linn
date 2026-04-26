use ncm_api_rs::Query;

use crate::api::{Album, Artist, Song, SoundQuality, client::client};

pub async fn get_song_url(id: u64, quality: SoundQuality) -> anyhow::Result<String> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("level", &quality.to_string());

    match client().song_url_v1(&query).await {
        Ok(resp) => {
            if let Some(url) = resp.body["data"][0]["url"].as_str() {
                return Ok(url.to_string());
            } else {
                return Err(anyhow::anyhow!("未找到歌曲URL"));
            }
        }
        Err(e) => {
            eprintln!("获取歌曲URL失败: {}", e);
            return Err(e.into());
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

