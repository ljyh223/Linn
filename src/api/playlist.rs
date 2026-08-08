use ncm_api_rs::Query;

use crate::api::{Album, Artist, PlaylistDetail, Song, client::client};

fn parse_song(value: &serde_json::Value) -> Song {
    use crate::api::pic_url_from_id;

    let artists = value["ar"].as_array().cloned().unwrap_or_default();
    let alnum = value["al"].as_object().cloned().unwrap_or_default();
    let pic_url = alnum
        .get("picUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let pic_id = alnum.get("picId").and_then(|v| v.as_u64()).unwrap_or(0);
    let cover_url = if pic_url.is_empty() && pic_id != 0 {
        pic_url_from_id(pic_id)
    } else {
        pic_url
    };
    let artist_list = artists
        .iter()
        .map(|artist| Artist {
            id: artist["id"].as_u64().unwrap_or(0),
            name: artist["name"].as_str().unwrap_or("").to_string(),
            avatar: None,
            // cover_url: artist["picUrl"].as_str().unwrap_or("").to_string(),
        })
        .collect::<Vec<_>>();

    Song {
        id: value["id"].as_u64().unwrap_or(0),
        name: value["name"].as_str().unwrap_or("").to_string(),
        cover_url: cover_url.clone(),
        artists: artist_list,
        album: Album {
            id: alnum.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            name: alnum
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            cover_url,
        },
        duration: value["dt"].as_u64().unwrap_or(0),
    }
}

// 分页获取歌单歌曲：直接用已知的 track_ids 切片 + 只请求一次歌曲详情接口，
// 避免每次分页都重新请求歌单详情（/playlist/track/all 内部会先拉一次完整详情）
pub async fn get_playlist_track_all(
    track_ids: &[u64],
    offset: usize,
    limit: usize,
) -> anyhow::Result<Vec<Song>> {
    let end = (offset + limit).min(track_ids.len());
    if offset >= end {
        return Ok(Vec::new());
    }
    let batch = &track_ids[offset..end];
    let ids = batch
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let query = Query::new().param("ids", &ids);

    match client().song_detail(&query).await {
        Ok(resp) => {
            let songs = resp.body["songs"].as_array().cloned().unwrap_or_default();
            Ok(songs.iter().map(parse_song).collect())
        }
        Err(e) => {
            eprintln!("分页获取歌单歌曲失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn get_playlist_detail(id: u64) -> anyhow::Result<PlaylistDetail> {
    let query = Query::new().param("id", &id.to_string());

    match client().playlist_detail(&query).await {
        Ok(resp) => {
            let pl = resp.body["playlist"].as_object().unwrap();
            let tracks = pl["tracks"].as_array().cloned().unwrap_or_default();
            let track_ids = pl["trackIds"].as_array().cloned().unwrap_or_default();
            let track_list: Vec<Song> = tracks.iter().map(parse_song).collect();
            let track_id_list: Vec<u64> = track_ids
                .iter()
                .map(|ids| ids["id"].as_u64().unwrap_or(0))
                .collect();
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
