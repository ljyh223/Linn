use ncm_api_rs::Query;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::api::{
    LyricDetail, Song, amll::fetch_amll_ttml, client::client, qqmusic::fetch_qq_lyric_for_song,
};
use crate::utils::ttml::is_ttml;

static LYRIC_CACHE: Lazy<Mutex<HashMap<u64, LyricDetail>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
const MAX_LYRIC_CACHE_ENTRIES: usize = 128;

pub async fn get_lryic(id: u64) -> anyhow::Result<LyricDetail> {
    log::debug!("[lyrics][ncm] requesting song_id={id}");
    let query = Query::new().param("id", &id.to_string());

    match client().lyric_new(&query).await {
        Ok(resp) => {
            let json = resp.body;
            let lyric = get_str(&json, &["lrc", "lyric"]);
            let tlyric = get_str(&json, &["tlyric", "lyric"]);
            let yrc = get_str(&json, &["yrc", "lyric"]);
            let ytlrc = get_str(&json, &["ytlrc", "lyric"]);
            let is_pure_music = json["isPure"].as_bool().unwrap_or(false);
            log::info!(
                "[lyrics][ncm] song_id={id} lyric={} tlyric={} yrc={} ytlrc={} pure={is_pure_music}",
                lyric.as_ref().is_some_and(|v| !v.trim().is_empty()),
                tlyric.as_ref().is_some_and(|v| !v.trim().is_empty()),
                yrc.as_ref().is_some_and(|v| !v.trim().is_empty()),
                ytlrc.as_ref().is_some_and(|v| !v.trim().is_empty())
            );
            return Ok(LyricDetail {
                lyric,
                tlyric,
                yrc,
                ytlrc,
                is_pure_music,
            });
        }
        Err(e) => {
            eprintln!("获取歌词失败: {}", e);
            return Err(e.into());
        }
    }
}

/// Fetch lyrics using the configured source priority. The song metadata is
/// required to search QQ Music because the two services use unrelated IDs.
pub async fn get_lyric_for_song(song: &Song) -> anyhow::Result<LyricDetail> {
    if let Some(cached) = LYRIC_CACHE.lock().await.get(&song.id).cloned() {
        log::info!("[lyrics] cache hit song_id={}", song.id);
        eprintln!("[lyrics] lyric cache hit song_id={}", song.id);
        return Ok(cached);
    }

    // Fetch every source concurrently. Selection happens only after all three
    // requests have completed, so a slower high-priority source cannot be
    // bypassed by an earlier low-priority response.
    let (amll_result, ncm_result, qq_result) = tokio::join!(
        fetch_amll_ttml(song.id),
        get_lryic(song.id),
        fetch_qq_lyric_for_song(song),
    );

    let amll = match amll_result {
        Ok(Some(ttml)) => Some(ttml),
        Ok(None) => {
            log::info!("[lyrics][amll] unavailable song_id={}", song.id);
            eprintln!("[lyrics] AMLL TTML unavailable song_id={}", song.id);
            None
        }
        Err(error) => {
            log::warn!(
                "[lyrics][amll] request failed song_id={} error={error}",
                song.id
            );
            eprintln!(
                "[lyrics] AMLL request failed song_id={} error={error}",
                song.id
            );
            None
        }
    };
    let ncm = match ncm_result {
        Ok(lyric) => Some(lyric),
        Err(error) => {
            log::warn!(
                "[lyrics][ncm] request failed song_id={} error={error}",
                song.id
            );
            eprintln!(
                "[lyrics] NCM request failed song_id={} error={error}",
                song.id
            );
            None
        }
    };
    let qq = match qq_result {
        Ok(lyric) => Some(lyric),
        Err(error) => {
            log::warn!(
                "[lyrics][qq] request failed ncm_song_id={} error={error}",
                song.id
            );
            eprintln!(
                "[lyrics] QQ request failed ncm_song_id={} error={error}",
                song.id
            );
            None
        }
    };

    let (source, selected) = if let Some(ttml) = amll {
        (
            "amll_ttml",
            LyricDetail {
                lyric: Some(ttml),
                tlyric: None,
                is_pure_music: false,
                yrc: None,
                ytlrc: None,
            },
        )
    } else if let Some(lyric) = ncm.as_ref().filter(|lyric| lyric_has_yrc(lyric)) {
        ("ncm_yrc", lyric.clone())
    } else if let Some(lyric) = qq.as_ref().filter(|lyric| lyric_has_yrc(lyric)) {
        ("qq_qrc", lyric.clone())
    } else if let Some(lyric) = ncm.as_ref().filter(|lyric| lyric_has_plain(lyric)) {
        ("ncm_lrc", lyric.clone())
    } else if let Some(lyric) = qq.as_ref().filter(|lyric| lyric_has_plain(lyric)) {
        ("qq_lrc", lyric.clone())
    } else {
        log::warn!("[lyrics] no usable source song_id={}", song.id);
        eprintln!("[lyrics] no usable source song_id={}", song.id);
        return ncm.or(qq).ok_or_else(|| anyhow::anyhow!("歌词不存在"));
    };

    log::info!("[lyrics] selected source={} song_id={}", source, song.id);
    eprintln!("[lyrics] selected source={} song_id={}", source, song.id);
    let mut cache = LYRIC_CACHE.lock().await;
    if cache.len() >= MAX_LYRIC_CACHE_ENTRIES && !cache.contains_key(&song.id) {
        if let Some(evicted_id) = cache.keys().next().copied() {
            cache.remove(&evicted_id);
        }
    }
    cache.insert(song.id, selected.clone());
    Ok(selected)
}

fn nonempty(value: Option<&String>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

fn lyric_has_yrc(lyric: &LyricDetail) -> bool {
    !lyric.is_pure_music && lyric.yrc.as_deref().is_some_and(has_word_timing)
}

fn has_word_timing(raw: &str) -> bool {
    raw.lines().any(|line| {
        let line = line.trim_start();
        (line.starts_with('[') && line.contains(',') && line.contains('('))
            || line.starts_with('{') && line.contains("\"t\"")
    })
}

fn lyric_has_plain(lyric: &LyricDetail) -> bool {
    !lyric.is_pure_music
        && lyric.lyric.as_deref().is_some_and(|text| !is_ttml(text))
        && nonempty(lyric.lyric.as_ref())
}

fn get_str(json: &serde_json::Value, path: &[&str]) -> Option<String> {
    path.iter()
        .fold(Some(json), |acc, key| acc?.get(*key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
