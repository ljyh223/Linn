use ncm_api_rs::Query;

use crate::api::{Album, Artist, Playlist, Song, client::client};

/// 网易云图片 URL 生成：picId 无法直接使用，需要先加密成 CDN 路径再拼 URL。
/// 加密方式：id 字符串与固定 key 逐字节异或 -> md5 -> base64（/ 变 _，+ 变 -），
/// 主机号 p1~p4 由 id 最后一位决定。
pub fn pic_url_from_id(pic_id: u64) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use md5::{Digest, Md5};

    let key = b"3go8&$8*3*3h0k(2)2".to_vec();
    let id_bytes = pic_id.to_string().into_bytes();
    let xored: Vec<u8> = id_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect();

    let digest = Md5::digest(&xored);
    let encrypted = STANDARD.encode(digest).replace('/', "_").replace('+', "-");

    let seed = (pic_id % 10) as u32;
    let p = match seed {
        0..=2 => 1,
        3..=4 => 2,
        5..=7 => 3,
        _ => 4,
    };

    format!("https://p{p}.music.126.net/{encrypted}/{pic_id}.jpg")
}

/// 搜索结果封装，total 为总匹配数（用于分页）
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchResult<T> {
    pub total: u64,
    pub items: Vec<T>,
}

/// 搜索类型 code（cloudsearch 接口）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchType {
    Song,     // 1
    Album,    // 10
    Artist,   // 100
    Playlist, // 1000
}

impl SearchType {
    pub fn code(self) -> i64 {
        match self {
            Self::Song => 1,
            Self::Album => 10,
            Self::Artist => 100,
            Self::Playlist => 1000,
        }
    }
}

async fn search_body(
    keywords: &str,
    search_type: SearchType,
    limit: u64,
    offset: u64,
) -> anyhow::Result<serde_json::Value> {
    let query = Query::new()
        .param("keywords", keywords)
        .param("type", &search_type.code().to_string())
        .param("limit", &limit.to_string())
        .param("offset", &offset.to_string());

    match crate::api::client::client().cloudsearch(&query).await {
        Ok(resp) => Ok(resp.body),
        Err(e) => {
            eprintln!("搜索失败: {}", e);
            Err(e.into())
        }
    }
}

fn parse_song(value: &serde_json::Value) -> Song {
    let alnum = value["al"].as_object().cloned().unwrap_or_default();
    let pic_url = alnum.get("picUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let pic_id = alnum.get("picId").and_then(|v| v.as_u64()).unwrap_or(0);
    let cover_url = if pic_url.is_empty() && pic_id != 0 {
        pic_url_from_id(pic_id)
    } else {
        pic_url
    };
    Song {
        id: value["id"].as_u64().unwrap_or(0),
        name: value["name"].as_str().unwrap_or("").to_string(),
        cover_url: cover_url.clone(),
        artists: value["ar"]
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
            id: alnum.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            name: alnum.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cover_url,
        },
        duration: value["dt"].as_u64().unwrap_or(0),
    }
}

fn parse_playlist(value: &serde_json::Value) -> Playlist {
    Playlist {
        id: value["id"].as_u64().unwrap_or(0),
        name: value["name"].as_str().unwrap_or("").to_string(),
        cover_url: value["coverImgUrl"].as_str().unwrap_or("").to_string(),
        creator_name: value["creator"]["nickname"].as_str().unwrap_or("").to_string(),
        creator_id: value["creator"]["userId"].as_u64().unwrap_or(0),
        description: value["description"].as_str().unwrap_or("").to_string(),
        play_count: value["playCount"].as_u64().unwrap_or(0),
    }
}

fn parse_artist(value: &serde_json::Value) -> Artist {
    Artist {
        id: value["id"].as_u64().unwrap_or(0),
        name: value["name"].as_str().unwrap_or("").to_string(),
        avatar: value["picUrl"]
            .as_str()
            .or_else(|| value["img1v1Url"].as_str())
            .map(|s| s.to_string()),
    }
}

fn parse_album(value: &serde_json::Value) -> Album {
    let pic_url = value["picUrl"].as_str().unwrap_or("").to_string();
    let pic_id = value["picId"].as_u64().unwrap_or(0);
    let cover_url = if pic_url.is_empty() && pic_id != 0 {
        pic_url_from_id(pic_id)
    } else {
        pic_url
    };
    Album {
        id: value["id"].as_u64().unwrap_or(0),
        name: value["name"].as_str().unwrap_or("").to_string(),
        cover_url,
    }
}

/// 搜索单曲
pub async fn search_songs(
    keywords: &str,
    limit: u64,
    offset: u64,
) -> anyhow::Result<SearchResult<Song>> {
    let body = search_body(keywords, SearchType::Song, limit, offset).await?;
    let result = &body["result"];
    Ok(SearchResult {
        total: result["songCount"].as_u64().unwrap_or(0),
        items: result["songs"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(parse_song)
            .collect(),
    })
}

/// 搜索歌单
pub async fn search_playlists(
    keywords: &str,
    limit: u64,
    offset: u64,
) -> anyhow::Result<SearchResult<Playlist>> {
    let body = search_body(keywords, SearchType::Playlist, limit, offset).await?;
    let result = &body["result"];
    Ok(SearchResult {
        total: result["playlistCount"].as_u64().unwrap_or(0),
        items: result["playlists"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(parse_playlist)
            .collect(),
    })
}

/// 搜索歌手
pub async fn search_artists(
    keywords: &str,
    limit: u64,
    offset: u64,
) -> anyhow::Result<SearchResult<Artist>> {
    let body = search_body(keywords, SearchType::Artist, limit, offset).await?;
    let result = &body["result"];
    Ok(SearchResult {
        total: result["artistCount"].as_u64().unwrap_or(0),
        items: result["artists"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(parse_artist)
            .collect(),
    })
}

/// 搜索专辑
pub async fn search_albums(
    keywords: &str,
    limit: u64,
    offset: u64,
) -> anyhow::Result<SearchResult<Album>> {
    let body = search_body(keywords, SearchType::Album, limit, offset).await?;
    let result = &body["result"];
    Ok(SearchResult {
        total: result["albumCount"].as_u64().unwrap_or(0),
        items: result["albums"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(parse_album)
            .collect(),
    })
}

/// 搜索建议：按类型分组返回，类型信息由分组本身携带
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchSuggest {
    pub songs: Vec<Song>,
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
}

/// 搜索建议（search_suggest），无需指定类型
pub async fn search_suggest(keywords: &str) -> anyhow::Result<SearchSuggest> {
    let query = Query::new().param("keywords", keywords);

    match crate::api::client::client().search_suggest(&query).await {
        Ok(resp) => {
            let result = &resp.body["result"];
            let mut suggest = SearchSuggest::default();

            suggest.songs = result["songs"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|song| {
                    let album = &song["album"];
                    let pic_url = album["picUrl"].as_str().unwrap_or("").to_string();
                    let pic_id = album["picId"].as_u64().unwrap_or(0);
                    let cover_url = if pic_url.is_empty() && pic_id != 0 {
                        pic_url_from_id(pic_id)
                    } else {
                        pic_url
                    };
                    Song {
                        id: song["id"].as_u64().unwrap_or(0),
                        name: song["name"].as_str().unwrap_or("").to_string(),
                        cover_url: cover_url.clone(),
                        artists: song["artists"]
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
                            id: album["id"].as_u64().unwrap_or(0),
                            name: album["name"].as_str().unwrap_or("").to_string(),
                            cover_url,
                        },
                        duration: song["duration"].as_u64().unwrap_or(0),
                    }
                })
                .collect();

            suggest.artists = result["artists"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_artist)
                .collect();

            suggest.albums = result["albums"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_album)
                .collect();

            Ok(suggest)
        }
        Err(e) => {
            eprintln!("获取搜索建议失败: {}", e);
            Err(e.into())
        }
    }
}

#[allow(dead_code)]
pub(crate) async fn test_suggest_structure() {
    use ncm_api_rs::Query;
    let query = Query::new().param("keywords", "周杰伦");
    let mut client = crate::api::client::client();
    let resp = client.search_suggest(&query).await.unwrap();
    println!(
        "===== suggest 结构 =====\n{}",
        serde_json::to_string_pretty(&resp.body).unwrap()
    );
}

#[allow(dead_code)]
pub(crate) async fn test_multimatch_structure() {
    use ncm_api_rs::Query;
    for kw in ["周杰伦", "晴天", "七里香 专辑"] {
        let query = Query::new().param("keywords", kw);
        let mut client = crate::api::client::client();
        let resp = client.search_multimatch(&query).await.unwrap();
        let result = resp.body["result"].as_object().cloned().unwrap_or_default();
        let mut keys: Vec<(&String, usize)> = result
            .iter()
            .filter(|(k, _)| *k != "orders")
            .map(|(k, v)| (k, v.as_array().map_or(0, |a| a.len())))
            .collect();
        keys.sort();
        println!(
            "===== multimatch '{}' =====",
            kw
        );
        println!(
            "keys: {}",
            keys
                .iter()
                .map(|(k, n)| format!("{}[{}]", k, n))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let orders = result
            .get("orders")
            .and_then(|o| o.as_array())
            .map(|o| {
                o.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        println!("orders: {}", orders);
    }
}