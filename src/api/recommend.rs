use ncm_api_rs::Query;

use crate::api::{Album, Artist, Playlist, Song, client::client};


pub async fn get_recommend_playlist() -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new();
    match client().recommend_resource(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            if let Some(playlists) = resp.body["recommend"].as_array() {
                for pl in playlists {
                    res.push(Playlist {
                        id: pl["id"].as_u64().unwrap_or(0),
                        name: pl["name"].as_str().unwrap_or("").to_string(),
                        cover_url: pl["picUrl"].as_str().unwrap_or("").to_string(),
                        creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                        creator_id: pl["creator"]["userId"].as_u64().unwrap_or(0),
                        description: pl["copywriter"].as_str().unwrap_or("").to_string(),
                        play_count: pl["playcount"].as_u64().unwrap_or(0),
                    });
                }
            }
            return Ok(res);
        }
        Err(e) => {
            eprintln!("获取推荐歌单失败: {}", e);
            return Err(e.into());
        }
    }
}

pub async fn get_recommend_song() -> anyhow::Result<Vec<Song>> {
    let query = Query::new();
    match client().recommend_songs(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            let songs = resp.body["data"]["dailySongs"].as_array().cloned().unwrap_or_default();
            for song in songs {
                res.push(Song {
                    id: song["id"].as_u64().unwrap_or(0),
                    name: song["name"].as_str().unwrap_or("").to_string(),
                    cover_url: song["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    artists: song["ar"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .map(|artist| Artist {
                            id: artist["id"].as_u64().unwrap_or(0),
                            name: artist["name"].as_str().unwrap_or("").to_string(),
                            avatar: None,
                        })
                        .collect(),
                    album: Album {
                        id: song["al"]["id"].as_u64().unwrap_or(0),
                        name: song["al"]["name"].as_str().unwrap_or("").to_string(),
                        cover_url: song["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    },
                    duration: song["dt"].as_u64().unwrap_or(0),
                })
            }
            Ok(res)
        }
        Err(e) => {
            eprintln!("获取推荐歌曲失败: {}", e);
            Err(e.into())
        }
    }
}
