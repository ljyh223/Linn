use ncm_api_rs::{ApiClient, Query, create_client};
use once_cell::sync::Lazy;
use std::{any, sync::RwLock};

use crate::api::{Album, Artist, Playlist, PlaylistDetail, Song, SoundQuality, model::{AlbumDetail, LyricDetail, UserInfo}};

static CLIENT: Lazy<RwLock<Option<ApiClient>>> = Lazy::new(|| RwLock::new(None));

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

pub async fn get_reconmend_song() -> anyhow::Result<Vec<Song>> {
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
                            cover_url: String::new(),
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
                        cover_url: String::new(),
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

pub async fn get_song_url(id: u64, quality: SoundQuality) -> anyhow::Result<String> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("level", &quality.to_string());

    match client().song_url_v1(&query).await {
        Ok(resp) => {
            if let Some(url) = resp.body["data"][0]["url"].as_str() {
                return Ok(url.to_string());
            } else {
                return Err(anyhow::anyhow!("未找到歌曲URL"));
            }
        }
        Err(e) => {
            eprintln!("获取歌曲URL失败: {}", e);
            return Err(e.into());
        }
    }
}

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
                    cover_url: "".to_string(),
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

pub async fn get_song_detail(ids: Vec<u64>) -> anyhow::Result<Vec<Song>> {
    let query = Query::new().param(
        "ids",
        ids.iter()
            .map(u64::to_string)
            .collect::<Vec<String>>()
            .join(", ")
            .as_str(),
    );

    match client().song_detail(&query).await {
        Ok(resp) => {
            let songs = resp.body["songs"].as_array().unwrap();
            let mut song_lsit = Vec::new();
            for song in songs {
                song_lsit.push(Song {
                    id: song["id"].as_u64().unwrap_or(0),
                    name: song["name"].as_str().unwrap_or("").to_string(),
                    cover_url: song["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    artists: song["ar"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|artist| Artist {
                            id: artist["id"].as_u64().unwrap_or(0),
                            name: artist["name"].as_str().unwrap_or("").to_string(),
                            cover_url: String::new(),
                        })
                        .collect::<Vec<Artist>>(),
                    album: Album {
                        id: song["al"]["id"].as_u64().unwrap_or(0),
                        name: song["al"]["name"].as_str().unwrap_or("").to_string(),
                        cover_url: song["al"]["picUrl"].as_str().unwrap_or("").to_string(),
                    },
                    duration: song["dt"].as_u64().unwrap_or(0),
                });
            }
            Ok(song_lsit)
        }
        Err(e) => {
            eprintln!("获取歌曲URL失败: {}", e);
            return Err(e.into());
        }
    }
}


pub async fn get_lryic(id: u64) -> anyhow::Result<LyricDetail> {
    let query = Query::new().param("id", &id.to_string());

    match client().lyric_new(&query).await {
        Ok(resp) => {
            let json = resp.body;
            let lyric  = get_str(&json, &["lrc", "lyric"]);
            let tlyric = get_str(&json, &["tlyric", "lyric"]);
            let yrc    = get_str(&json, &["yrc", "lyric"]);
            let is_pure_music = json["isPure"].as_bool().unwrap_or(false);
            return Ok(LyricDetail { lyric: lyric, tlyric: tlyric, yrc: yrc, is_pure_music});
        }
        Err(e) => {
            eprintln!("获取歌词失败: {}", e);
            return Err(e.into());
        }
    }
}
fn get_str(json: &serde_json::Value, path: &[&str]) -> Option<String> {
    path.iter()
        .fold(Some(json), |acc, key| acc?.get(*key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

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

#[tokio::test]
async fn test_init_client() {
    init_client("MUSIC_A_T=1628302039878; MUSIC_R_T=1628302040015; MUSIC_R_U=00236B77FCA4628CDCF272DA9C15003D5625364D3CF83134A651ECA863E4E450AB1F9FE6AD2D3FAE60EC080DA0E16D8F5AFB8A871D7F8D775B2F64C3C883E111C03C8821B4449EFA1D677C5EE50978A86B; NMTID=00OJQaCFkk3ieiCe0XTuG1gnl2rEPAAAAGdbBUBAw; __csrf=a1a3d17aaafeeb01ea60cf5871667fba; MUSIC_U=005D16BA9075E5D9A048A0F7962D4C91FA1AEB0F61D1E69F3B7734E57AB813091A11E5A04165D52A6419E40048509923D5460F18BEC14FBC83A56E77BD27DD892328AD08D8C12C824EDF0F154EDA47FF32A41C75257DAFBA7C7A22BC4C4482C94B17828718E6822BFCC4DA07043E3F71C640F4D2F41DFF9B3A67DA615CD38A1E7BC135089E8EE5E73438EA3FAC2AB441092900C3F5ECD6CB8A09B0456597D32B228064168E04FF074199C19A9EC7164CAEA611C99C611E5043EE29A744FD8B4D0D6BBF41045385C1744FFF5FA06C10169DD11B83FE5E4AA6F87284B6EEF2C915DB7856DA8CE1FEC337A6EC6662A2195F9B328B04B592587866C2B4F86BB89022F0DAEABE29DD8FEF0C61D92BE4D12CA312FE3833C9B8D053E326BA5B44EA8BAFA65BA396D7BFA87FF174733D3F44D8A40A3AA931E258985D6552C8CF0F97F30BA1E91BCB9F94AF5FFF958F608F3BB4A762450222DD094C55262805A809B9430734DD501D646E7561F43F5E69C812F85231; Max-Age=2147483647; Expires=Mon, 26 Apr 2094 11:07:36 GMT; Path=/wapi/clientlog".to_string());

    // test_playlist_detail().await;
    // test_recommend_playlist().await;
    // test_song_url().await;
    // test_song_detail().await;

    // test_album_detail().await;
    // test_recommend_songs().await;
    // test_lyric().await;
    test_user_info().await;

}


async fn test_playlist_detail() {
    match get_playlist_detail(8656494498).await {
        Ok(detail) => println!("Playlist Detail: {:?}", detail),
        Err(e) => eprintln!("Error fetching playlist detail: {}", e),
    }
}

async fn test_recommend_playlist() {
    match get_recommend_playlist().await {
        Ok(playlists) => println!("Recommended Playlists: {:?}", playlists),
        Err(e) => eprintln!("Error fetching recommended playlists: {}", e),
    }
}

async fn test_song_url() {
    match get_song_url(1969519579, SoundQuality::Standard).await {
        Ok(url) => println!("Song URL: {}", url),
        Err(e) => eprintln!("Error fetching song URL: {}", e),
    }
}

async fn test_song_detail() {
    match get_song_detail(vec![3363002263, 2003647821]).await {
        Ok(songs) => println!("Songs: {:?}", songs),
        Err(e) => eprintln!("Error fetching song details: {}", e),
    }
}

async fn test_album_detail() {
    match get_album_detail(32311).await {
        Ok(detail) => println!("Album Detail: {:?}", detail),
        Err(e) => eprintln!("Error fetching album detail: {}", e),
    }
}

async fn test_recommend_songs() {
    match get_reconmend_song().await {
        Ok(songs) => println!("Recommended Songs: {:?}", songs),
        Err(e) => eprintln!("Error fetching recommended songs: {}", e),
    }
}

async fn test_lyric() {
    match get_lryic(1824020871).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }

    match get_lryic(1831761295).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }

    match get_lryic(1923203310).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }

    match get_lryic(1966112058).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }
}


async fn test_user_info() {
    match get_user_info().await {
        Ok(info) => println!("User Info: {:?}", info),
        Err(e) => eprintln!("Error fetching user info: {}", e),
    }
}