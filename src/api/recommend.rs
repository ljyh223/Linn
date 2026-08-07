use ncm_api_rs::Query;

use crate::api::{
    Album, ApiClientExt, Artist, HomeBlock, HomeBlockType, Playlist, Song,
    client::{client, client_ext},
};

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
            let songs = resp.body["data"]["dailySongs"]
                .as_array()
                .cloned()
                .unwrap_or_default();
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

pub async fn get_home_block() -> anyhow::Result<Vec<HomeBlock>> {
    let query = Query::new();
    match client_ext().home_recommend_resource(&query).await {
        Ok(resp) => {
            // eprintln!("获取首页块成功: {}", resp.body);
            let res = resp.body["data"]["items"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let mut blocks = Vec::new();
            for b in res {
                let res_type = b["resourceType"].as_str().unwrap_or("");
                let module_type = b["moduleType"].as_str().unwrap_or("");
                let sub_resource_type = b["subResourceType"].as_str().unwrap_or("");
                let block = HomeBlock {
                    type_: if res_type == "dailySongs" && sub_resource_type == "dailySong" {
                        HomeBlockType::Daily
                    } else if res_type == "dailySongs" && sub_resource_type == "style_dailySong" {
                        HomeBlockType::DailyCategory {
                            tag_id: b["extData"]["tagId"].as_u64().unwrap(),
                            category_id: b["extData"]["categoryId"].as_u64().unwrap(),
                            song_id: b["extData"]["rcmdData"]
                                .as_array()
                                .cloned()
                                .unwrap_or_default()
                                .iter()
                                .map(|n| n["itemId"].as_str().unwrap().to_string().parse().unwrap())
                                .collect(),
                        }
                    } else if res_type == "playList"
                        && (module_type == "mood"
                            || module_type == "new_song_album"
                            || module_type == "radar"
                            || module_type == "artist_playlist")
                    {
                        // eprintln!("获取首页块类型: {}", module_type);
                        // eprintln!("获取首页块: {}", b);
                        HomeBlockType::Playlist(
                            b["resourceId"]
                                .as_str()
                                .unwrap()
                                .to_string()
                                .parse()
                                .unwrap(),
                        )
                    } else if res_type == "star" {
                        HomeBlockType::Playlist(b["extData"]["playlist"]["id"].as_u64().unwrap())
                    } else if res_type == "fm" {
                        HomeBlockType::Fm
                    } else if res_type == "similarSong" {
                        HomeBlockType::Queue(
                            b.get("resourceId")
                                .and_then(|v| v.as_str())
                                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                                .map(|ids| {
                                    ids.into_iter()
                                        .filter_map(|id| id.parse::<u64>().ok())
                                        .collect()
                                })
                                .unwrap_or_default(),
                        )
                    } else if res_type == "similarArtist" {
                        HomeBlockType::Artist(
                            b["resourceExtInfo"]["artists"]
                                .as_array()
                                .cloned()
                                .unwrap_or_default()
                                .iter()
                                .map(|a| a["id"].as_u64().unwrap())
                                .collect(),
                        )
                    } else {
                        HomeBlockType::Unknown
                    },
                    title: b["title"].as_str().unwrap_or("").to_string(),
                    sub_title: b["simplifiedTitle"].as_str().unwrap_or("").to_string(),
                    cover: b["coverUrl"].as_str().unwrap_or("").to_string(),
                    color: String::new(),
                };

                blocks.push(block);
            }
            Ok(blocks)
        }
        Err(e) => {
            eprintln!("获取首页歌单失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn get_home_category_daily_song_list(
    ids: Vec<u64>,
    category_id: u64,
    tag_id: u64,
) -> anyhow::Result<Vec<Song>> {
    let query = Query::new()
        .param(
            "song_ids",
            ids.iter()
                .map(u64::to_string)
                .collect::<Vec<String>>()
                .join(",")
                .as_str(),
        )
        .param("category_id", &category_id.to_string())
        .param("tag_id", &tag_id.to_string());

    match client_ext().home_category_daily_song_list(&query).await {
        Ok(resp) => {
            // eprintln!("获取首页歌单成功: {}", resp.body);
            let songs = resp.body["data"]["dailySongs"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|song| Song {
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
                .collect();

            Ok(songs)
        }

        Err(e) => {
            eprintln!("获取首页歌单失败: {}", e);
            Err(e.into())
        }
    }
}
