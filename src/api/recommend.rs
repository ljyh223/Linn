use std::{fs, path::PathBuf};

use chrono::{Duration, Local, Timelike};
use ncm_api_rs::Query;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::APP_NAME;
use crate::api::{
    Album, ApiClientExt, Artist, HomeBlock, HomeBlockType, HomeSection, Playlist, Song, UserInfo,
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

pub async fn get_home_block() -> anyhow::Result<Vec<HomeSection>> {
    if let Some(pages) = load_home_cache() {
        log::info!("使用首页推荐本地缓存");
        return Ok(parse_home_resource_pages(&pages));
    }

    let first_page = get_home_resource_page("0", true, &[]).await?;
    let mut pages = vec![first_page.raw];
    if first_page.has_more {
        match get_home_resource_page(
            &first_page.cursor.to_string(),
            false,
            &first_page.loaded_position_codes,
        )
        .await
        {
            Ok(second_page) => pages.push(second_page.raw),
            Err(error) => log::warn!("加载首页推荐第二页失败，将只展示第一页: {error}"),
        }
    }

    save_home_cache(&pages);
    Ok(parse_home_resource_pages(&pages))
}

/// 获取新版首页资源流的一页。`loaded_position_codes` 应传入此前已渲染的 positionCode，
/// 以便服务端返回后续模块。
pub async fn get_home_resource_page(
    cursor: &str,
    is_first_screen: bool,
    loaded_position_codes: &[String],
) -> anyhow::Result<HomeResourcePage> {
    // 首屏抓包要求空字符串而非 JSON 空数组；后续页才传 positionCode 的 JSON 数组。
    let loaded_position_codes = encode_loaded_position_codes(loaded_position_codes)?;
    let query = Query::new()
        .param("cursor", cursor)
        .param(
            "is_first_screen",
            if is_first_screen { "true" } else { "false" },
        )
        .param("loaded_position_codes", &loaded_position_codes);

    let response = client_ext().home_recommend_resource_show(&query).await?;
    if response.status != 200 {
        anyhow::bail!("首页推荐接口返回状态 {}", response.status);
    }

    Ok(HomeResourcePage {
        cursor: response.body["data"]["cursor"]
            .as_i64()
            .or_else(|| {
                response.body["data"]["cursor"]
                    .as_str()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or_default(),
        has_more: response.body["data"]["hasMore"].as_bool().unwrap_or(false),
        loaded_position_codes: response.body["data"]["blocks"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|block| block["positionCode"].as_str().map(ToOwned::to_owned))
            .collect(),
        raw: response.body,
    })
}

fn encode_loaded_position_codes(loaded_position_codes: &[String]) -> anyhow::Result<String> {
    // 首屏抓包要求空字符串而非 JSON 空数组；后续页才传 positionCode 的 JSON 数组。
    if loaded_position_codes.is_empty() {
        Ok(String::new())
    } else {
        Ok(serde_json::to_string(loaded_position_codes)?)
    }
}

#[derive(Debug)]
pub struct HomeResourcePage {
    pub cursor: i64,
    pub has_more: bool,
    pub loaded_position_codes: Vec<String>,
    raw: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct HomeResourceCache {
    period: String,
    pages: Vec<Value>,
}

fn home_cache_period() -> String {
    let now = Local::now();
    let date = if now.hour() < 6 {
        now.date_naive() - Duration::days(1)
    } else {
        now.date_naive()
    };
    date.format("%F").to_string()
}

fn home_cache_path() -> PathBuf {
    let user_id = UserInfo::load_from_disk()
        .map(|user| user.id)
        .unwrap_or_default();
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
        .join(format!("home-recommend-{user_id}.json"))
}

fn load_home_cache() -> Option<Vec<Value>> {
    let content = fs::read_to_string(home_cache_path()).ok()?;
    let cache: HomeResourceCache = serde_json::from_str(&content).ok()?;
    (cache.period == home_cache_period()).then_some(cache.pages)
}

fn save_home_cache(pages: &[Value]) {
    let path = home_cache_path();
    let Some(parent) = path.parent() else { return };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let cache = HomeResourceCache {
        period: home_cache_period(),
        pages: pages.to_vec(),
    };
    match serde_json::to_vec(&cache) {
        Ok(bytes) => {
            if let Err(error) = fs::write(path, bytes) {
                log::warn!("写入首页推荐缓存失败: {error}");
            }
        }
        Err(error) => log::warn!("序列化首页推荐缓存失败: {error}"),
    }
}

fn parse_home_resource_pages(pages: &[Value]) -> Vec<HomeSection> {
    pages
        .iter()
        .flat_map(|page| {
            page["data"]["blocks"]
                .as_array()
                .into_iter()
                .flatten()
                .cloned()
        })
        .filter_map(|block| parse_home_resource_block(&block))
        .collect()
}

fn parse_home_resource_block(block: &Value) -> Option<HomeSection> {
    let Some(position_code) = block["positionCode"].as_str() else {
        return None;
    };
    let data = if position_code == "PAGE_RECOMMEND_SHORTCUT" {
        block["dslData"].get("data")?
    } else {
        select_special_field(&block["dslData"])?
    };

    let blocks = match position_code {
        "PAGE_RECOMMEND_DAILY_RECOMMEND" => data["resources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(daily_resource_to_home_block)
            .collect(),
        "PAGE_RECOMMEND_RADAR"
        | "PAGE_RECOMMEND_SPECIAL_CLOUD_VILLAGE_PLAYLIST"
        | "PAGE_RECOMMEND_MIXED_ARTIST_PLAYLIST"
        | "PAGE_RECOMMEND_RANK"
        | "PAGE_RECOMMEND_MY_SHEET"
        | "PAGE_RECOMMEND_COMBINATION"
        | "PAGE_RECOMMEND_FEELING_PLAYLIST_LOCATION"
        | "PAGE_RECOMMEND_SCENE_PLAYLIST_LOCATION"
        | "PAGE_RECOMMEND_MONTH_YEAR_PLAYLIST"
        | "PAGE_RECOMMEND_NEW_SONG_AND_ALBUM" => collection_resource_blocks(data),
        // 快捷入口会混合最近常听歌单、专辑和每日推荐；播客 position 不在分支内，
        // 因而不会进入首页。
        "PAGE_RECOMMEND_SHORTCUT" => collection_resource_blocks(data),
        "PAGE_RECOMMEND_PRIVATE_RCMD_SONG" | "PAGE_RECOMMEND_RED_SIMILAR_SONG" => {
            song_recommendation_block(data)
        }
        _ => return None, // 包括全部播客 position。
    };
    (!blocks.is_empty()).then(|| HomeSection {
        position_code: position_code.to_string(),
        title: section_title(block, data),
        blocks,
    })
}

fn section_title(block: &Value, data: &Value) -> String {
    block["dslData"]["dslShowTitle"]
        .as_str()
        .or_else(|| data["title"].as_str())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(
            || match block["positionCode"].as_str().unwrap_or_default() {
                "PAGE_RECOMMEND_DAILY_RECOMMEND" => "每日推荐",
                "PAGE_RECOMMEND_RADAR" => "雷达歌单",
                "PAGE_RECOMMEND_SHORTCUT" => "最近常听",
                "PAGE_RECOMMEND_SCENE_PLAYLIST_LOCATION" => "场景歌单",
                "PAGE_RECOMMEND_COMBINATION" => "推荐组合",
                "PAGE_RECOMMEND_SPECIAL_CLOUD_VILLAGE_PLAYLIST" => "云村歌单",
                "PAGE_RECOMMEND_RANK" => "排行榜",
                "PAGE_RECOMMEND_MONTH_YEAR_PLAYLIST" => "月度歌单",
                "PAGE_RECOMMEND_NEW_SONG_AND_ALBUM" => "新歌新碟",
                "PAGE_RECOMMEND_PRIVATE_RCMD_SONG" => "为你推荐",
                "PAGE_RECOMMEND_RED_SIMILAR_SONG" => "相似歌曲",
                _ => "推荐内容",
            },
        )
        .to_string()
}

/// Android 的 `selectSpecialField`：模块字段名由服务端实验决定，因此优先取直接
/// `blockResource`，否则选取最长的对象字段，并再解开其内部的 `blockResource`。
fn select_special_field(dsl_data: &Value) -> Option<&Value> {
    if dsl_data["blockResource"].is_object() {
        return dsl_data.get("blockResource");
    }

    let candidate = dsl_data
        .as_object()?
        .iter()
        .filter(|(_, value)| value.is_object())
        .max_by_key(|(key, _)| key.len())?
        .1;

    if candidate["blockResource"].is_object() {
        candidate.get("blockResource")
    } else {
        Some(candidate)
    }
}

fn daily_resource_to_home_block(resource: &Value) -> Option<HomeBlock> {
    let type_ = match resource["resourceType"].as_str()? {
        "dailySongs" => HomeBlockType::Daily,
        "star" => HomeBlockType::Playlist(resource_id(resource)?),
        "fm" => HomeBlockType::Fm,
        "similarSong" => HomeBlockType::Queue(song_ids(resource)),
        "similarArtist" => HomeBlockType::Artist(
            resource["resourceExtInfo"]["artists"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|artist| artist["id"].as_u64())
                .collect(),
        ),
        "playList" => HomeBlockType::Playlist(resource_id(resource)?),
        _ => return None,
    };

    Some(home_block_from_resource(resource, type_))
}

fn collection_resource_blocks(data: &Value) -> Vec<HomeBlock> {
    data["resources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|resource| {
            let id = resource_id(resource)?;
            let type_ = match resource["resourceType"].as_str() {
                Some("album") => HomeBlockType::Album(id),
                Some("dailySongs") => HomeBlockType::Daily,
                // EAPI 对歌单使用 playList 或 list，榜单也可直接按歌单打开。
                Some("playList") | Some("list") | None => HomeBlockType::Playlist(id),
                _ => return None,
            };
            Some(home_block_from_resource(resource, type_))
        })
        .collect()
}

fn song_recommendation_block(data: &Value) -> Vec<HomeBlock> {
    let song_ids = data["content"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|group| group["items"].as_array().into_iter().flatten())
        .filter_map(|item| resource_id(item))
        .collect::<Vec<_>>();
    if song_ids.is_empty() {
        return Vec::new();
    }

    vec![HomeBlock {
        type_: HomeBlockType::Queue(song_ids),
        title: data["header"]["title"]
            .as_str()
            .unwrap_or("推荐歌曲")
            .to_string(),
        sub_title: String::new(),
        cover: String::new(),
        color: String::new(),
    }]
}

fn home_block_from_resource(resource: &Value, type_: HomeBlockType) -> HomeBlock {
    HomeBlock {
        type_,
        title: resource["singleLineTitle"]
            .as_str()
            .or_else(|| resource["title"].as_str())
            .unwrap_or_default()
            .to_string(),
        sub_title: resource["subTitle"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        cover: resource["coverImg"]
            .as_str()
            .or_else(|| resource["coverUrl"].as_str())
            .unwrap_or_default()
            .to_string(),
        color: String::new(),
    }
}

fn resource_id(resource: &Value) -> Option<u64> {
    resource["resourceId"]
        .as_u64()
        .or_else(|| {
            resource["resourceId"]
                .as_str()
                .and_then(|id| id.parse().ok())
        })
        .or_else(|| resource["extInfo"]["playlist"]["id"].as_u64())
}

fn song_ids(resource: &Value) -> Vec<u64> {
    resource["resourceId"]
        .as_str()
        .and_then(|ids| serde_json::from_str::<Vec<String>>(ids).ok())
        .into_iter()
        .flatten()
        .filter_map(|id| id.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn follows_android_position_dispatch_and_special_field_selection() {
        let response = json!({
            "data": { "blocks": [
                {"positionCode": "PAGE_RECOMMEND_RANK", "dslData": {"rank_module_with_a_long_name": {
                    "resources": [{"resourceId": "42", "title": "榜单", "coverImg": "cover"}]
                }}},
                {"positionCode": "PAGE_RECOMMEND_DAILY_RECOMMEND", "dslData": {"blockResource": {
                    "resources": [
                        {"resourceType": "dailySongs", "resourceId": "daily", "title": "每日推荐"},
                        {"resourceType": "playList", "resourceId": "7", "title": "歌单"}
                    ]
                }}},
                {"positionCode": "PAGE_RECOMMEND_RED_SIMILAR_SONG", "dslData": {"song_module": {
                    "header": {"title": "相似歌曲"},
                    "content": {"items": [{"items": [{"resourceId": "9"}, {"resourceId": "10"}]}]}
                }}},
                {"positionCode": "PAGE_RECOMMEND_PODCAST", "dslData": {"blockResource": {
                    "resources": [{"resourceId": "100", "title": "不应展示的播客"}]
                }}}
            ]}
        });

        let sections = parse_home_resource_pages(&[response]);
        assert_eq!(sections.len(), 3, "播客模块不能进入首页");
        assert_eq!(sections[0].position_code, "PAGE_RECOMMEND_RANK");
        assert_eq!(sections[0].title, "排行榜");
        assert!(matches!(
            sections[0].blocks[0].type_,
            HomeBlockType::Playlist(42)
        ));
        assert_eq!(sections[1].title, "每日推荐");
        assert!(matches!(sections[1].blocks[0].type_, HomeBlockType::Daily));
        assert!(matches!(
            sections[1].blocks[1].type_,
            HomeBlockType::Playlist(7)
        ));
        assert!(
            matches!(sections[2].blocks[0].type_, HomeBlockType::Queue(ref ids) if ids == &[9, 10])
        );
    }

    #[test]
    fn first_page_uses_empty_loaded_positions_and_next_page_uses_json_array() {
        assert_eq!(encode_loaded_position_codes(&[]).unwrap(), "");
        assert_eq!(
            encode_loaded_position_codes(&["PAGE_RECOMMEND_RADAR".to_string()]).unwrap(),
            r#"["PAGE_RECOMMEND_RADAR"]"#
        );
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
