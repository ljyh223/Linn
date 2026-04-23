use ncm_api_rs::Query;

use crate::api::{Album, Artist, PlaylistDetail, Song, client::client};


pub async fn get_playlist_detail(id: u64) -> anyhow::Result<PlaylistDetail> {
    let query = Query::new().param("id", &id.to_string());

    match client().playlist_detail(&query).await {
        Ok(resp) => {
            let pl = resp.body["playlist"].as_object().unwrap();
            let tracks = pl["tracks"].as_array().cloned().unwrap_or_default();
            let track_ids = pl["trackIds"].as_array().cloned().unwrap_or_default();
            let mut track_list = Vec::new();
            let mut track_id_list = Vec::new();
            for track in tracks {
                let artists = track["ar"].as_array().cloned().unwrap_or_default();
                let alnum = track["al"].as_object().cloned().unwrap_or_default();
                let artist_list = artists
                    .iter()
                    .map(|artist| Artist {
                        id: artist["id"].as_u64().unwrap_or(0),
                        name: artist["name"].as_str().unwrap_or("").to_string(),
                        avatar: None,
                        // cover_url: artist["picUrl"].as_str().unwrap_or("").to_string(),
                    })
                    .collect::<Vec<_>>();

                track_list.push(Song {
                    id: track["id"].as_u64().unwrap_or(0),
                    name: track["name"].as_str().unwrap_or("").to_string(),
                    cover_url: alnum["picUrl"].as_str().unwrap_or("").to_string(),
                    artists: artist_list,
                    album: Album {
                        id: alnum["id"].as_u64().unwrap_or(0),
                        name: alnum["name"].as_str().unwrap_or("").to_string(),
                        cover_url: alnum["picUrl"].as_str().unwrap_or("").to_string(),
                    },
                    duration: track["dt"].as_u64().unwrap_or(0),
                })
            }

            for ids in track_ids {
                track_id_list.push(ids["id"].as_u64().unwrap_or(0));
            }
            Ok(PlaylistDetail {
                id: pl["id"].as_u64().unwrap_or(0),
                name: pl["name"].as_str().unwrap_or("").to_string(),
                cover_url: pl["coverImgUrl"].as_str().unwrap_or("").to_string(),
                creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                creator_id: pl["creator"]["userId"].as_u64().unwrap_or(0),
                description: pl["description"].as_str().unwrap_or("").to_string(),
                play_count: pl["playCount"].as_u64().unwrap_or(0),
                tracks: track_list,
                track_ids: track_id_list,
            })
        }
        Err(e) => {
            eprintln!("获取歌单详情失败: {}", e);
            return Err(e.into());
        }
    }
}