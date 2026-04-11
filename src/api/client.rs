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
            println!("状态: {}", resp.status);
            // println!("歌单详情响应: {:?}", resp.body);
            let pl = resp.body["playlist"].as_object().unwrap();
            // println!("歌单对象: {:?}", pl);
            let tracks = pl["tracks"].as_array().cloned().unwrap_or_default();
            // println!("歌曲列表: {:?}", tracks);
            let mut track_list = Vec::new();
            for track in tracks {
                println!("处理歌曲: {}", track);
                let artists = track["ar"].as_array().cloned().unwrap_or_default();
                let alnum = track["al"].as_object().cloned().unwrap_or_default();
                let artist_list = artists.iter().map(|artist| Artist {
                    id: artist["id"].as_i64().unwrap_or(0),
                    name: artist["name"].as_str().unwrap_or("").to_string(),
                    cover_url: String::new(),
                    // cover_url: artist["picUrl"].as_str().unwrap_or("").to_string(),
                }).collect::<Vec<_>>();
                
                track_list.push(Song {
                    id: track["id"].as_i64().unwrap_or(0),
                    name: track["name"].as_str().unwrap_or("").to_string(),
                    cover_url: alnum["picUrl"].as_str().unwrap_or("").to_string(),
                    artists: artist_list,
                    album: Album {
                        id: alnum["id"].as_i64().unwrap_or(0),
                        name: alnum["name"].as_str().unwrap_or("").to_string(),
                        cover_url: alnum["picUrl"].as_str().unwrap_or("").to_string(),
                    },
                    duration: track["dt"].as_i64().unwrap_or(0),
                })
            }
            return Ok(PlaylistDetail {
                id: pl["id"].as_i64().unwrap_or(0),
                name: pl["name"].as_str().unwrap_or("").to_string(),
                cover_url: pl["coverImgUrl"].as_str().unwrap_or("").to_string(),
                creator_name: pl["creator"]["nickname"].as_str().unwrap_or("").to_string(),
                creator_id: pl["creator"]["userId"].as_i64().unwrap_or(0),
                description: pl["description"].as_str().unwrap_or("").to_string(),
                play_count: pl["playCount"].as_i64().unwrap_or(0),
                tracks: track_list,
            })
        }
        Err(e) => {
            eprintln!("获取歌单详情失败: {}", e);
            return Err(e.into());
        }
    }
}


#[tokio::test]
async fn test() {
    init_client("MUSIC_A_T=1628302039878; MUSIC_R_T=1628302040015; MUSIC_R_U=00236B77FCA4628CDCF272DA9C15003D5625364D3CF83134A651ECA863E4E450AB1F9FE6AD2D3FAE60EC080DA0E16D8F5AFB8A871D7F8D775B2F64C3C883E111C03C8821B4449EFA1D677C5EE50978A86B; NMTID=00OJQaCFkk3ieiCe0XTuG1gnl2rEPAAAAGdbBUBAw; __csrf=a1a3d17aaafeeb01ea60cf5871667fba; MUSIC_U=005D16BA9075E5D9A048A0F7962D4C91FA1AEB0F61D1E69F3B7734E57AB813091A11E5A04165D52A6419E40048509923D5460F18BEC14FBC83A56E77BD27DD892328AD08D8C12C824EDF0F154EDA47FF32A41C75257DAFBA7C7A22BC4C4482C94B17828718E6822BFCC4DA07043E3F71C640F4D2F41DFF9B3A67DA615CD38A1E7BC135089E8EE5E73438EA3FAC2AB441092900C3F5ECD6CB8A09B0456597D32B228064168E04FF074199C19A9EC7164CAEA611C99C611E5043EE29A744FD8B4D0D6BBF41045385C1744FFF5FA06C10169DD11B83FE5E4AA6F87284B6EEF2C915DB7856DA8CE1FEC337A6EC6662A2195F9B328B04B592587866C2B4F86BB89022F0DAEABE29DD8FEF0C61D92BE4D12CA312FE3833C9B8D053E326BA5B44EA8BAFA65BA396D7BFA87FF174733D3F44D8A40A3AA931E258985D6552C8CF0F97F30BA1E91BCB9F94AF5FFF958F608F3BB4A762450222DD094C55262805A809B9430734DD501D646E7561F43F5E69C812F85231; Max-Age=2147483647; Expires=Mon, 26 Apr 2094 11:07:36 GMT; Path=/wapi/clientlog".to_string());

    let playlist_detail = get_playlist_detail(8656494498).await;
    match playlist_detail {
        Ok(detail) => {
            println!("Playlist Detail: {:?}", detail);
        }
        Err(e) => {
            eprintln!("Error fetching playlist detail: {}", e);
        }
    }

    let recommend_playlist = get_recommned_playlist().await;
    match recommend_playlist {
        Ok(playlists) => {
            println!("Recommended Playlists: {:?}", playlists);
        }
        Err(e) => {
            eprintln!("Error fetching recommended playlists: {}", e);
        }
    }
}