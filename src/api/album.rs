use ncm_api_rs::Query;

use crate::api::{Album, AlbumDetail, Artist, Song, client::client};


pub async fn get_album_detail(id: u64) -> anyhow::Result<AlbumDetail> {
    let query = Query::new().param("id", &id.to_string());

    match client().album(&query).await {
        Ok(resp) => {
            let album = resp.body["album"].as_object().unwrap();
            let songs = resp.body["songs"].as_array().unwrap();
            let cover_url = album["picUrl"].as_str().unwrap_or("").to_string();
            let artists = album["artists"]
                .as_array()
                .unwrap()
                .iter()
                .map(|artist| Artist {
                    id: artist["id"].as_u64().unwrap(),
                    name: artist["name"].as_str().unwrap().to_string(),
                    avatar: None,
                })
                .collect::<Vec<Artist>>();
            let tracks = songs
                .iter()
                .map(|s| Song {
                    id: s["id"].as_u64().unwrap_or(0),
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    cover_url: cover_url.clone(),
                    artists: artists.clone(),
                    album: Album {
                        id: album["id"].as_u64().unwrap_or(0),
                        name: album["name"].as_str().unwrap_or("").to_string(),
                        cover_url: cover_url.clone(),
                    },
                    duration: s["dt"].as_u64().unwrap_or(0),
                })
                .collect::<Vec<Song>>();

            Ok(AlbumDetail {
                id: album["id"].as_u64().unwrap_or(0),
                name: album["name"].as_str().unwrap_or("").to_string(),
                description: album["description"].as_str().unwrap_or("").to_string(),
                cover_url,
                artists: artists,
                tracks: tracks,
            })
        }
        Err(e) => {
            eprintln!("获取专辑详情失败: {}", e);
            return Err(e.into());
        }
    }
}



pub async fn album_subscribe(id: u64, subscribe: bool) -> anyhow::Result<()> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("t", if subscribe { "1" } else { "0" });

    match client().album_sub(&query).await {
        Ok(resp) => {
            eprintln!("收藏/取消收藏专辑：{:?}", resp.body);
            Ok(())
        },
        Err(e) => {
            eprintln!("订阅专辑失败: {}", e);
            Err(e.into())
        }
    }
}


