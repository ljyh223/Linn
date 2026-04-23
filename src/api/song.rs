use ncm_api_rs::Query;

use crate::api::{SoundQuality, client::client};

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