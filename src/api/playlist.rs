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

// 收藏/取消收藏歌单
pub async fn playlist_subscribe(id: u64, subscribe: bool) -> anyhow::Result<()> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("t", if subscribe { "1" } else { "0" });

    match client().playlist_subscribe(&query).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("操作歌单失败: {}", e);
            Err(e.into())
        }
    }
}

// 新建歌单
pub async fn playlist_create(name: &str) -> anyhow::Result<u64> {
    let query = Query::new().param("name", name);

    match client().playlist_create(&query).await {
        Ok(resp) => {
            let id = resp.body["id"].as_u64().unwrap_or(0);
            Ok(id)
        }
        Err(e) => {
            eprintln!("新建歌单失败: {}", e);
            Err(e.into())
        }
    }
}

// 删除歌单
pub async fn playlist_delete(id: u64) -> anyhow::Result<()> {
    let query = Query::new().param("id", &id.to_string());

    match client().playlist_delete(&query).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("删除歌单失败: {}", e);
            Err(e.into())
        }
    }
}

// 喜欢音乐
pub async fn like_song(id: u64, like: bool) -> anyhow::Result<()> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("like", if like { "true" } else { "false" });

    match client().like(&query).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("操作喜欢失败: {}", e);
            Err(e.into())
        }
    }
}

// 是否喜欢音乐
pub async fn is_like_song(id: u64) -> anyhow::Result<bool> {
    let query = Query::new().param("ids", &format!("[{}]", id));

    match client().song_like_check(&query).await {
        Ok(resp) => {
            let liek_ids = resp.body["ids"].as_array().cloned().unwrap_or_default();
            Ok(liek_ids.len() > 0)
        }
        Err(e) => {
            eprintln!("获取是否喜欢失败: {}", e);
            Err(e.into())
        }
    }
}

// 添加歌曲到歌单
pub async fn playlist_track_add(pid: u64, track_id: u64) -> anyhow::Result<()> {
    let query = Query::new()
        .param("op", "add")
        .param("pid", &pid.to_string())
        .param("tracks", &track_id.to_string());

    match client().playlist_tracks(&query).await {
        Ok(resp) => {
            eprintln!("返回结果：{:?}", resp.body);
            eprintln!("添加歌曲到歌单成功");
            Ok(())
        },
        Err(e) => {
            eprintln!("添加歌曲到歌单失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn playlist_track_del(pid: u64, track_id: u64) -> anyhow::Result<()> {
    let query = Query::new()
        .param("op", "del")
        .param("pid", &pid.to_string())
        .param("tracks", &track_id.to_string());

    match client().playlist_tracks(&query).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("歌单删除歌曲失败: {}", e);
            Err(e.into())
        }
    }
}
