use ncm_api_rs::Query;

use crate::api::{LyricDetail, client::client};

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
