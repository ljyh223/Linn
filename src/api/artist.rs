use ncm_api_rs::Query;

use crate::api::{Album, Artist, ArtistDetail, Mv, Song, client::client};

pub async fn get_artist_detail(id: u64) -> anyhow::Result<ArtistDetail> {
    let query = Query::new().param("id", &id.to_string());
    match client().artist_detail(&query).await {
        Ok(resp) => {
            let body = &resp.body;

            // 基础检查：确保有 data
            let data = body.get("data").and_then(|v| v.as_object()).unwrap();

            // artist 对象
            let artist = data.get("artist").and_then(|v| v.as_object()).unwrap();
            // user 对象（用于取 signature）
            let user = data.get("user").and_then(|v| v.as_object());

            // identify 对象（用于取 imageDesc）
            let identify = data.get("identify").and_then(|v| v.as_object());

            // 提取 transNames（数组），拼接成单个字符串
            let trans_name = artist
                .get("transNames")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();

            // 提取 alias（数组），拼接成单个字符串
            let alias_text = artist
                .get("alias")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();

            // identifyDesc：优先取 identify.imageDesc；若不存在则取 user.description/detailDescription
            let identify_desc = identify
                .and_then(|i| i.get("imageDesc").and_then(|v| v.as_str()))
                .map(|s| s.to_owned())
                .or_else(|| {
                    user.and_then(|u| u.get("description").and_then(|v| v.as_str()))
                        .map(|s| s.to_owned())
                })
                .or_else(|| {
                    user.and_then(|u| u.get("detailDescription").and_then(|v| v.as_str()))
                        .map(|s| s.to_owned())
                })
                .unwrap_or_default();

            // signature（来自 user）
            let signature = user
                .and_then(|u| u.get("signature").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();

            // briefDesc（来自 artist）
            let brief_desc = artist
                .get("briefDesc")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // description：这里使用 artist.briefDesc（与示例里的 briefDesc 内容一致）
            let description = brief_desc.clone();

            // 音乐/专辑/MV数量
            let music_size = artist
                .get("musicSize")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let album_size = artist
                .get("albumSize")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let mv_size = artist.get("mvSize").and_then(|v| v.as_u64()).unwrap_or(0);

            Ok(ArtistDetail {
                id: artist.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                name: artist
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                avatar: artist
                    .get("avatar")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                description,
                identify_desc,
                alias_text,
                signature,
                brief_desc,
                trans_name,
                music_size,
                album_size,
                mv_size,
            })
        }
        Err(e) => {
            eprintln!("获取歌手信息失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn get_artist_song(id: u64) -> anyhow::Result<Vec<Song>> {
    let query = Query::new().param("id", &id.to_string());
    match client().artists(&query).await {
        Ok(resp) => {
            let hot_songs = resp.body["hotSongs"].as_array().unwrap();

            // 获取 data 对象
            let data = hot_songs
                .iter()
                .map(|s| Song {
                    id: s["id"].as_u64().unwrap(),
                    name: s["name"].as_str().unwrap().to_string(),
                    cover_url: s["al"]["picUrl"].as_str().unwrap().to_string(),
                    artists: s["ar"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|a| Artist {
                            id: a["id"].as_u64().unwrap(),
                            name: a["name"].as_str().unwrap().to_string(),
                            avatar: None,
                        })
                        .collect(),
                    album: Album {
                        id: s["al"]["id"].as_u64().unwrap(),
                        name: s["al"]["name"].as_str().unwrap().to_string(),
                        cover_url: s["al"]["picUrl"].as_str().unwrap().to_string(),
                    },
                    duration: s["dt"].as_u64().unwrap(),
                })
                .collect();

            Ok(data)
        }
        Err(e) => {
            eprintln!("获取歌手歌曲列表失败: {}", e);
            Err(e.into())
        }
    }
}


pub async fn get_artist_album(id: u64) -> anyhow::Result<Vec<Album>> {
    let query = Query::new().param("id", &id.to_string());
    match client().artist_album(&query).await {
        Ok(resp) => {
            let hot_albums = resp.body["hotAlbums"].as_array().unwrap();

            let data = hot_albums
                .iter()
                .map(|a| Album {
                    id: a["id"].as_u64().unwrap(),
                    name: a["name"].as_str().unwrap().to_string(),
                    cover_url: a["picUrl"].as_str().unwrap().to_string(),
                })
                .collect();

            Ok(data)
        }
        Err(e) => {
            eprintln!("获取歌手专辑列表失败: {}", e);
            Err(e.into())
        }
    }
}


pub async fn get_artist_mv(id: u64) -> anyhow::Result<Vec<Mv>> {
    let query = Query::new().param("id", &id.to_string());
    match client().artist_mv(&query).await {
        Ok(resp) => {
            // 注意：根据官方文档 /artist/mv 接口，MV 列表的字段名通常是 "mvs"
            let mvs = resp.body["mvs"]
                .as_array()
                .unwrap();

            let data = mvs
                .iter()
                .map(|m| Mv {
                    id: m["id"].as_u64().unwrap_or(0),
                    name: m["name"].as_str().unwrap_or("").to_string(),
                    // 封面优先使用 16:9，没有就用 imgurl
                    cover: m["imgurl16v9"]
                        .as_str()
                        .unwrap_or_else(|| m["imgurl"].as_str().unwrap_or(""))
                        .to_string(),
                    duration: m["duration"].as_u64().unwrap_or(0),
                })
                .collect();

            Ok(data)
        }
        Err(e) => {
            eprintln!("获取歌手 MV 列表失败: {}", e);
            Err(e.into())
        }
    }
}
