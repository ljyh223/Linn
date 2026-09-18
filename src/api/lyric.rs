use moka::future::Cache;
use ncm_api_rs::Query;
use once_cell::sync::Lazy;
use std::future::Future;

use crate::api::{
    LyricDetail, Song, amll::fetch_amll_ttml, client::client, qqmusic::fetch_qq_lyric_for_song,
};
use crate::lyrics::{LyricsSource, ttml::parse_ttml};
use crate::utils::ttml::is_ttml;

#[derive(Debug, Clone)]
pub struct SelectedLyrics {
    pub detail: LyricDetail,
    pub source: LyricsSource,
}

const MAX_LYRIC_CACHE_ENTRIES: u64 = 128;
static LYRIC_CACHE: Lazy<Cache<u64, SelectedLyrics>> = Lazy::new(|| {
    Cache::builder()
        .max_capacity(MAX_LYRIC_CACHE_ENTRIES)
        .build()
});

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
            Ok(LyricDetail {
                lyric,
                tlyric,
                yrc,
                ytlrc,
                is_pure_music,
            })
        }
        Err(e) => {
            log::warn!("[lyrics][ncm] request failed song_id={id} error={e}");
            Err(e.into())
        }
    }
}

/// Fetch lyrics using the configured source priority. The song metadata is
/// required to search QQ Music because the two services use unrelated IDs.
pub async fn get_lyric_for_song(song: &Song) -> anyhow::Result<SelectedLyrics> {
    if let Some(cached) = LYRIC_CACHE.get(&song.id).await {
        log::info!("[lyrics] cache hit song_id={}", song.id);
        return Ok(cached);
    }

    let song = song.clone();
    get_or_fetch_cached(&LYRIC_CACHE, song.id, async move {
        fetch_lyrics_uncached(&song).await
    })
    .await
}

async fn get_or_fetch_cached<F>(
    cache: &Cache<u64, SelectedLyrics>,
    song_id: u64,
    fetch: F,
) -> anyhow::Result<SelectedLyrics>
where
    F: Future<Output = anyhow::Result<SelectedLyrics>>,
{
    cache
        .try_get_with(song_id, async move {
            fetch.await.map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.as_str().to_owned()))
}

async fn fetch_lyrics_uncached(song: &Song) -> anyhow::Result<SelectedLyrics> {
    // Keep every request concurrent, but stop waiting once the priority order
    // makes the result definitive. A completed low-priority source still may
    // not bypass a higher-priority request that is in flight.
    let amll = async {
        match fetch_amll_ttml(song.id).await {
            Ok(Some(ttml)) => validated_amll(ttml),
            Ok(None) => {
                log::info!("[lyrics][amll] unavailable song_id={}", song.id);
                None
            }
            Err(error) => {
                log::warn!(
                    "[lyrics][amll] request failed song_id={} error={error}",
                    song.id
                );
                None
            }
        }
    };
    let ncm = async {
        match get_lryic(song.id).await {
            Ok(lyric) => Some(lyric),
            Err(error) => {
                log::warn!(
                    "[lyrics][ncm] request failed song_id={} error={error}",
                    song.id
                );
                None
            }
        }
    };
    let qq = async {
        match fetch_qq_lyric_for_song(song).await {
            Ok(lyric) => Some(lyric),
            Err(error) => {
                log::warn!(
                    "[lyrics][qq] request failed ncm_song_id={} error={error}",
                    song.id
                );
                None
            }
        }
    };

    let Some((source_label, selected)) = resolve_source_futures(amll, ncm, qq).await else {
        log::warn!("[lyrics] no usable source song_id={}", song.id);
        return Err(anyhow::anyhow!("歌词不存在"));
    };

    log::info!(
        "[lyrics] selected source={} song_id={}",
        source_label,
        song.id
    );
    Ok(selected)
}

enum SourceState<T> {
    Pending,
    Ready(Option<T>),
}

enum SourceResolution {
    Pending,
    Complete(Option<(&'static str, SelectedLyrics)>),
}

fn resolve_ready_sources(
    amll: &SourceState<SelectedLyrics>,
    ncm: &SourceState<LyricDetail>,
    qq: &SourceState<LyricDetail>,
) -> SourceResolution {
    match amll {
        SourceState::Pending => return SourceResolution::Pending,
        SourceState::Ready(Some(lyrics)) => {
            return SourceResolution::Complete(Some(("amll_ttml", lyrics.clone())));
        }
        SourceState::Ready(None) => {}
    }

    let ncm = match ncm {
        SourceState::Pending => return SourceResolution::Pending,
        SourceState::Ready(lyrics) => lyrics,
    };
    if let Some(lyric) = ncm.as_ref().filter(|lyric| lyric_has_yrc(lyric)) {
        return SourceResolution::Complete(Some((
            "ncm_yrc",
            SelectedLyrics {
                source: LyricsSource::Yrc,
                detail: lyric.clone(),
            },
        )));
    }

    let qq = match qq {
        SourceState::Pending => return SourceResolution::Pending,
        SourceState::Ready(lyrics) => lyrics,
    };
    SourceResolution::Complete(select_lyrics(None, ncm.clone(), qq.clone()))
}

async fn resolve_source_futures<AF, NF, QF>(
    amll: AF,
    ncm: NF,
    qq: QF,
) -> Option<(&'static str, SelectedLyrics)>
where
    AF: Future<Output = Option<SelectedLyrics>>,
    NF: Future<Output = Option<LyricDetail>>,
    QF: Future<Output = Option<LyricDetail>>,
{
    tokio::pin!(amll, ncm, qq);
    let mut amll_state = SourceState::Pending;
    let mut ncm_state = SourceState::Pending;
    let mut qq_state = SourceState::Pending;

    loop {
        tokio::select! {
            result = &mut amll, if matches!(amll_state, SourceState::Pending) => {
                amll_state = SourceState::Ready(result);
            }
            result = &mut ncm, if matches!(ncm_state, SourceState::Pending) => {
                ncm_state = SourceState::Ready(result);
            }
            result = &mut qq, if matches!(qq_state, SourceState::Pending) => {
                qq_state = SourceState::Ready(result);
            }
        }

        match resolve_ready_sources(&amll_state, &ncm_state, &qq_state) {
            SourceResolution::Pending => {}
            SourceResolution::Complete(selected) => return selected,
        }
    }
}

fn validated_amll(ttml: String) -> Option<SelectedLyrics> {
    match parse_ttml(&ttml) {
        Ok(document) if !document.is_empty() => Some(SelectedLyrics {
            source: LyricsSource::Ttml,
            detail: LyricDetail {
                lyric: Some(ttml),
                tlyric: None,
                is_pure_music: false,
                yrc: None,
                ytlrc: None,
            },
        }),
        Ok(_) => {
            log::warn!("[lyrics][amll] ignored a TTML document with no lyric lines");
            None
        }
        Err(error) => {
            log::warn!("[lyrics][amll] ignored malformed TTML: {error}");
            None
        }
    }
}

fn select_lyrics(
    amll: Option<String>,
    ncm: Option<LyricDetail>,
    qq: Option<LyricDetail>,
) -> Option<(&'static str, SelectedLyrics)> {
    if let Some(ttml) = amll {
        if let Some(lyrics) = validated_amll(ttml) {
            return Some(("amll_ttml", lyrics));
        }
    }

    let selected = if let Some(lyric) = ncm.as_ref().filter(|lyric| lyric_has_yrc(lyric)) {
        ("ncm_yrc", LyricsSource::Yrc, lyric.clone())
    } else if let Some(lyric) = qq.as_ref().filter(|lyric| lyric_has_yrc(lyric)) {
        ("qq_qrc", LyricsSource::Qrc, lyric.clone())
    } else if let Some(lyric) = ncm.as_ref().filter(|lyric| lyric_has_plain(lyric)) {
        ("ncm_lrc", LyricsSource::Lrc, lyric.clone())
    } else if let Some(lyric) = qq.as_ref().filter(|lyric| lyric_has_plain(lyric)) {
        ("qq_lrc", LyricsSource::Lrc, lyric.clone())
    } else {
        ("unknown", LyricsSource::Unknown, ncm.or(qq)?)
    };
    Some((
        selected.0,
        SelectedLyrics {
            source: selected.1,
            detail: selected.2,
        },
    ))
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
        .try_fold(json, |value, key| value.get(*key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::pending;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;
    use tokio::sync::oneshot;

    fn plain(text: &str) -> LyricDetail {
        LyricDetail {
            lyric: Some(format!("[00:01.00]{text}")),
            tlyric: None,
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        }
    }

    fn word_timed(text: &str) -> LyricDetail {
        LyricDetail {
            lyric: Some(format!("[00:01.00]{text}")),
            tlyric: None,
            yrc: Some(format!("[1000,1000](1000,1000,0){text}")),
            ytlrc: None,
            is_pure_music: false,
        }
    }

    #[test]
    fn malformed_amll_does_not_hide_a_valid_lrc_fallback() {
        let (label, selected) = select_lyrics(
            Some("<tt xmlns=\"http://www.w3.org/ns/ttml\"><broken>".into()),
            Some(plain("still visible")),
            None,
        )
        .unwrap();

        assert_eq!(label, "ncm_lrc");
        assert_eq!(selected.source, LyricsSource::Lrc);
        assert_eq!(
            selected.detail.lyric.as_deref(),
            Some("[00:01.00]still visible")
        );
    }

    #[test]
    fn ttml_without_timed_lines_does_not_hide_a_valid_lrc_fallback() {
        let (label, selected) = select_lyrics(
            Some("<tt xmlns=\"http://www.w3.org/ns/ttml\"><body/></tt>".into()),
            Some(plain("fallback")),
            None,
        )
        .unwrap();

        assert_eq!(label, "ncm_lrc");
        assert_eq!(selected.source, LyricsSource::Lrc);
    }

    #[test]
    fn validated_amll_remains_the_highest_priority_source() {
        let ttml = include_str!("../../tests/fixtures/lyrics/karaoke.ttml");
        let (label, selected) =
            select_lyrics(Some(ttml.into()), Some(plain("fallback")), None).unwrap();

        assert_eq!(label, "amll_ttml");
        assert_eq!(selected.source, LyricsSource::Ttml);
        assert_eq!(selected.detail.lyric.as_deref(), Some(ttml));
    }

    #[tokio::test]
    async fn concurrent_consumers_share_one_in_flight_fetch() {
        let cache = Cache::new(4);
        let fetch_count = Arc::new(AtomicUsize::new(0));
        let make_fetch = || {
            let fetch_count = fetch_count.clone();
            async move {
                fetch_count.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(SelectedLyrics {
                    detail: plain("coalesced"),
                    source: LyricsSource::Lrc,
                })
            }
        };

        let (sidebar, fullscreen) = tokio::join!(
            get_or_fetch_cached(&cache, 42, make_fetch()),
            get_or_fetch_cached(&cache, 42, make_fetch()),
        );

        assert_eq!(sidebar.unwrap().source, LyricsSource::Lrc);
        assert_eq!(fullscreen.unwrap().source, LyricsSource::Lrc);
        assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn valid_amll_returns_without_waiting_for_lower_priority_sources() {
        let ttml = include_str!("../../tests/fixtures/lyrics/karaoke.ttml").to_owned();
        let amll = validated_amll(ttml).unwrap();

        let result = tokio::time::timeout(
            Duration::from_secs(1),
            resolve_source_futures(
                async { Some(amll) },
                pending::<Option<LyricDetail>>(),
                pending::<Option<LyricDetail>>(),
            ),
        )
        .await
        .expect("valid AMLL must not wait for lower-priority providers")
        .unwrap();

        assert_eq!(result.0, "amll_ttml");
        assert_eq!(result.1.source, LyricsSource::Ttml);
    }

    #[tokio::test]
    async fn lower_priority_result_cannot_bypass_pending_amll() {
        let (release, wait) = oneshot::channel();
        let task = tokio::spawn(resolve_source_futures(
            async move {
                wait.await.unwrap();
                None
            },
            async { Some(word_timed("ncm")) },
            async { Some(word_timed("qq")) },
        ));

        tokio::task::yield_now().await;
        assert!(!task.is_finished());
        release.send(()).unwrap();

        let result = task.await.unwrap().unwrap();
        assert_eq!(result.0, "ncm_yrc");
        assert_eq!(result.1.detail.yrc, word_timed("ncm").yrc);
    }

    #[tokio::test]
    async fn ncm_word_timing_does_not_wait_for_qq_after_amll_is_absent() {
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            resolve_source_futures(
                async { None },
                async { Some(word_timed("ncm")) },
                pending::<Option<LyricDetail>>(),
            ),
        )
        .await
        .expect("NCM word timing must make a lower-priority QQ result irrelevant")
        .unwrap();

        assert_eq!(result.0, "ncm_yrc");
        assert_eq!(result.1.source, LyricsSource::Yrc);
    }

    #[tokio::test]
    async fn ncm_plain_lyric_waits_for_possible_qq_word_timing() {
        let (release, wait) = oneshot::channel();
        let task = tokio::spawn(resolve_source_futures(
            async { None },
            async { Some(plain("ncm plain")) },
            async move {
                wait.await.unwrap();
                Some(word_timed("qq timed"))
            },
        ));

        tokio::task::yield_now().await;
        assert!(!task.is_finished());
        release.send(()).unwrap();

        let result = task.await.unwrap().unwrap();
        assert_eq!(result.0, "qq_qrc");
        assert_eq!(result.1.source, LyricsSource::Qrc);
    }
}
