use std::sync::OnceLock;
use std::time::Duration;

use moka::future::Cache;
use ncm_api_rs::Query;

use crate::api::{Artist, Mv, MvDetail, client::client, get_artist_mv};

static MV_URL_CACHE: OnceLock<Cache<u64, String>> = OnceLock::new();

fn mv_url_cache() -> &'static Cache<u64, String> {
    MV_URL_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(200)
            .time_to_idle(Duration::from_secs(60 * 60))
            .build()
    })
}

/// 获取 MV 播放地址
pub async fn get_mv_url(id: u64) -> anyhow::Result<String> {
    if let Some(url) = mv_url_cache().get(&id).await {
        return Ok(url);
    }

    let query = Query::new()
        .param("id", &id.to_string())
        .param("r", "1080");

    match client().mv_url(&query).await {
        Ok(resp) => {
            if let Some(url) = resp.body["data"]["url"].as_str() {
                let url = url.to_string();
                mv_url_cache().insert(id, url.clone()).await;
                Ok(url)
            } else {
                Err(anyhow::anyhow!("该 MV 需要 VIP 或暂无播放资源"))
            }
        }
        Err(e) => {
            eprintln!("获取 MV URL 失败: {}", e);
            Err(e.into())
        }
    }
}

/// 获取 MV 详情
pub async fn get_mv_detail(id: u64) -> anyhow::Result<MvDetail> {
    let query = Query::new().param("mvid", &id.to_string());
    match client().mv_detail(&query).await {
        Ok(resp) => {
            let data = resp.body["data"].as_object().unwrap();

            Ok(MvDetail {
                id: data.get("id").and_then(|v| v.as_u64()).unwrap_or(id),
                name: data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                cover: data
                    .get("cover")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                play_count: data
                    .get("playCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                brief_desc: data
                    .get("briefDesc")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                artists: data
                    .get("artists")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|a| Artist {
                                id: a["id"].as_u64().unwrap_or(0),
                                name: a["name"].as_str().unwrap_or("").to_string(),
                                avatar: None,
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        }
        Err(e) => {
            eprintln!("获取 MV 详情失败: {}", e);
            Err(e.into())
        }
    }
}

/// 获取相关 MV
///
/// 网易云 `/weapi/discovery/simiMV` 接口已失效（忽略 mvid 一律返回固定列表），
/// 这里退化为取「当前 MV 的歌手」的其他热门 MV 作为相关推荐。
pub async fn get_simi_mv(id: u64) -> anyhow::Result<Vec<Mv>> {
    let detail = get_mv_detail(id).await?;
    let artist_id = detail.artists.first().map(|a| a.id).unwrap_or(0);
    if artist_id == 0 {
        return Ok(vec![]);
    }

    let mvs = get_artist_mv(artist_id).await?;
    Ok(mvs.into_iter().filter(|m| m.id != id).take(12).collect())
}
