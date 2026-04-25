use ncm_api_rs::Query;

use crate::api::{Album, Playlist, UserCounts, UserDetails, UserInfo, client::client};

pub async fn get_user_info() -> anyhow::Result<UserInfo> {
    let query = Query::new();
    match client().user_account(&query).await {
        Ok(resp) => {
            let user = resp.body["profile"].as_object().unwrap();
            Ok(UserInfo {
                id: user["userId"].as_u64().unwrap_or(0),
                name: user["nickname"].as_str().unwrap_or("").to_string(),
                avatar_url: user["avatarUrl"].as_str().unwrap_or("").to_string(),
            })
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            return Err(e.into());
        }
    }
}

pub async fn get_user_subcount() -> anyhow::Result<UserCounts> {
    let query = Query::new();
    match client().user_subcount(&query).await {
        Ok(resp) => {
            // let subcount = ;
            Ok(serde_json::from_value(resp.body.clone())?)
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            return Err(e.into());
        }
    }
}

pub async fn get_user_sub_album() -> anyhow::Result<Vec<Album>> {
    let query = Query::new();
    match client().album_sublist(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            if let Some(albums) = resp.body["data"].as_array() {
                for album in albums {
                    res.push(Album {
                        id: album["id"].as_u64().unwrap(),
                        name: album["name"].as_str().unwrap().to_string(),
                        cover_url: album["picUrl"].as_str().unwrap().to_string(),
                    })
                }
            }
            Ok(res)
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            Err(e.into())
        }
    }
}
pub async fn get_user_playlist(uid: u64) -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new().param("uid", &uid.to_string());
    match client().user_playlist(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            if let Some(playlists) = resp.body["playlist"].as_array() {
                for pl in playlists {
                    res.push(Playlist {
                        id: pl["id"].as_u64().unwrap_or(0),
                        name: pl["name"].as_str().unwrap_or("").to_string(),
                        cover_url: pl["coverImgUrl"].as_str().unwrap_or("").to_string(),
                        creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                        creator_id: pl["creator"]["userId"].as_u64().unwrap_or(0),
                        description: pl["description"].as_str().unwrap_or("").to_string(),
                        play_count: pl["playCount"].as_u64().unwrap_or(0),
                    });
                }
            }
            return Ok(res);
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            return Err(e.into());
        }
    }
}

pub async fn get_user_playlist_created(uid: u64) -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new().param("uid", &uid.to_string());
    match client().user_playlist_create(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            if let Some(playlists) = resp.body["data"]["playlist"].as_array() {
                for pl in playlists {
                    res.push(Playlist {
                        id: pl["id"].as_u64().unwrap_or(0),
                        name: pl["name"].as_str().unwrap_or("").to_string(),
                        cover_url: pl["coverImgUrl"].as_str().unwrap_or("").to_string(),
                        creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                        creator_id: pl["creator"]["userId"].as_u64().unwrap_or(0),
                        description: pl["description"].as_str().unwrap_or("").to_string(),
                        play_count: pl["playCount"].as_u64().unwrap_or(0),
                    });
                }
            } else {
                eprintln!("获取用户创建歌单失败: {}", resp.body);
            }
            Ok(res)
        }

        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn get_user_playlist_collected(uid: u64) -> anyhow::Result<Vec<Playlist>> {
    let query = Query::new().param("uid", &uid.to_string());
    match client().user_playlist_collect(&query).await {
        Ok(resp) => {
            let mut res = Vec::new();
            if let Some(playlists) = resp.body["data"]["playlist"].as_array() {
                for pl in playlists {
                    res.push(Playlist {
                        id: pl["id"].as_u64().unwrap_or(0),
                        name: pl["name"].as_str().unwrap_or("").to_string(),
                        cover_url: pl["coverImgUrl"].as_str().unwrap_or("").to_string(),
                        creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                        creator_id: pl["creator"]["userId"].as_u64().unwrap_or(0),
                        description: pl["description"].as_str().unwrap_or("").to_string(),
                        play_count: pl["playCount"].as_u64().unwrap_or(0),
                    });
                }
            } else {
                eprintln!("获取用户收藏歌单失败: {}", resp.body);
            }
            Ok(res)
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            Err(e.into())
        }
    }
}

pub async fn get_user_detail(uid: u64) -> anyhow::Result<UserDetails> {
    let query = Query::new().param("uid", &uid.to_string());
    match client().user_detail(&query).await {
        Ok(resp) => {
            let user = resp.body["profile"].as_object().unwrap();
            // eprintln!("User Detail JSON: {:?}", resp.body);
            Ok(UserDetails {
                id: user["userId"].as_u64().unwrap_or(0),
                name: user["nickname"].as_str().unwrap_or("").to_string(),
                avatar_url: user["avatarUrl"].as_str().unwrap_or("").to_string(),
                follows: user["follows"].as_u64().unwrap_or(0).to_string(),
                followeds: user["followeds"].as_u64().unwrap_or(0).to_string(),
                vip_type: user["vipType"].as_u64().unwrap_or(0).to_string(),
                level: resp.body["level"].as_u64().unwrap_or(0).to_string(),
            })
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            Err(e.into())
        }
    }
}
