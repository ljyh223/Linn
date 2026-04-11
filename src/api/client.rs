use once_cell::sync::Lazy;
use std::sync::RwLock;
use ncm_api_rs::{ApiClient, Query, create_client};

use crate::api::{Album, Artist, Playlist, PlaylistDetail, Song};

static CLIENT: Lazy<RwLock<Option<ApiClient>>> = Lazy::new(|| {
    RwLock::new(None)
});

pub fn init_client(cookie: String) {
    let client = create_client(Some(cookie));

    let mut guard = CLIENT.write().unwrap();
    *guard = Some(client);
}

fn client() -> ApiClient {
    CLIENT
        .read()
        .unwrap()
        .as_ref()
        .expect("NCM client not initialized")
        .clone()
}


pub async fn get_recommned_playlist() -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new();
    match client().recommend_resource(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            if let Some(playlists) = resp.body["recommend"].as_array() {
                for pl in playlists {
                    res.push(Playlist {
                        id: pl["id"].as_i64().unwrap_or(0),
                        name: pl["name"].as_str().unwrap_or("").to_string(),
                        cover_url: pl["picUrl"].as_str().unwrap_or("").to_string(),
                        creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                        creator_id: pl["creator"]["userId"].as_i64().unwrap_or(0),
                        description: pl["copywriter"].as_str().unwrap_or("").to_string(),
                        play_count: pl["playcount"].as_i64().unwrap_or(0),
                    });
                }
            
            }
            return Ok(res);
        },
        Err(e) => {
            eprintln!("获取推荐歌单失败: {}", e);
            return Err(e.into());
        },
    }
}

pub async fn get_playlist_detail(id: i64) -> anyhow::Result<PlaylistDetail> {
    let query = Query::new().param("id", &id.to_string());
    match client().playlist_detail(&query).await {
        Ok(resp) => {
            let pl = resp.body["playlist"].as_object().unwrap();
            let tracks = pl["tracks"].as_array().cloned().unwrap_or_default();
            let mut track_list = Vec::new();
            for track in tracks {
                let artists = track["ar"].as_array().cloned().unwrap_or_default();
                let artist_list = artists.iter().map(|artist| Artist {
                    id: artist["id"].as_i64().unwrap_or(0),
                    name: artist["name"].as_str().unwrap_or("").to_string(),
                    cover_url: artist["picUrl"].as_str().unwrap_or("").to_string(),
                }).collect::<Vec<_>>();
                track_list.push(Song {
                    id: track["id"].as_i64().unwrap_or(0),
                    name: track["name"].as_str().unwrap_or("").to_string(),
                    cover_url: track["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    artists: artist_list, // Placeholder - you would need to populate this with actual artist data
                    album: Album {
                        id: track["al"]["id"].as_i64().unwrap_or(0),
                        name: track["al"]["name"].as_str().unwrap_or("").to_string(),
                        cover_url: track["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    },
                    duration: track["dt"].as_i64().unwrap_or(0),
                })
            }
            return Ok(PlaylistDetail {
                id: pl["id"].as_i64().unwrap_or(0),
                name: pl["name"].as_str().unwrap_or("").to_string(),
                cover_url: pl["picUrl"].as_str().unwrap_or("").to_string(),
                creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                creator_id: pl["creator"]["userId"].as_i64().unwrap_or(0),
                description: pl["copywriter"].as_str().unwrap_or("").to_string(),
                play_count: pl["playcount"].as_i64().unwrap_or(0),
                tracks: Vec::new(), // Placeholder - you would need to populate this with actual track data
            })
        }
        Err(e) => {
            eprintln!("获取歌单详情失败: {}", e);
            return Err(e.into());
        }
    }
}


#[test]
fn test(){
    init_client("".to_string());
    let _ = get_playlist_detail(8656494498);
    let _ = get_recommned_playlist();
}