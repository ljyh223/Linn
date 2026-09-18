use std::{sync::Arc, sync::LazyLock};

use regex::Regex;
use serde::Deserialize;

use super::{LyricModelError, LyricsDocument, LyricsLine, LyricsSource, TimedSpan, TimingQuality};
use crate::lyrics::ttml::{is_ttml, parse_ttml};

static LRC_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[(\d{2,3}):(\d{2})(?:[.:](\d{1,3}))?\]").unwrap());
static WORD_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[(\d+),(\d+)\]").unwrap());
static WORD_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d+),(\d+)(?:,\d+)?\)").unwrap());

const COPYRIGHT_PATTERNS: &[&str] = &["著作权", "QQ音乐", "腾讯音乐", "未经许可", "不得转载"];
const TRANSLATION_MATCH_LIMIT_MS: u64 = 3_000;

#[derive(Debug, Clone, Copy)]
pub struct RawLyrics<'a> {
    pub lyric: Option<&'a str>,
    pub translation: Option<&'a str>,
    pub word_synced: Option<&'a str>,
    pub word_translation: Option<&'a str>,
    pub word_source: LyricsSource,
    pub is_pure_music: bool,
}

/// Select and parse the best representation without exposing provider formats
/// to presentation code.
pub fn parse_document(input: RawLyrics<'_>) -> anyhow::Result<Option<LyricsDocument>> {
    if input.is_pure_music {
        return Ok(None);
    }
    if let Some(raw) = input.lyric.filter(|raw| is_ttml(raw)) {
        let document = parse_ttml(raw)?;
        return Ok((!document.is_empty()).then_some(document));
    }

    let word_document = match input.word_source {
        LyricsSource::Yrc | LyricsSource::Qrc => input
            .word_synced
            .map(|raw| parse_word_synced(raw, input.word_source))
            .transpose()?,
        _ => None,
    };
    let metadata_document = input.word_synced.map(parse_json_lines).transpose()?;
    let plain_document = input.lyric.map(parse_lrc).transpose()?;

    let mut document = match word_document.filter(|document| !document.is_empty()) {
        Some(word) => append_metadata(word, metadata_document.as_ref())?,
        None => match plain_document.filter(|document| !document.is_empty()) {
            Some(plain) => append_metadata(plain, metadata_document.as_ref())?,
            None => match metadata_document.filter(|document| !document.is_empty()) {
                Some(metadata) => metadata,
                None => return Ok(None),
            },
        },
    };

    let translation_raw = input.word_translation.or(input.translation);
    if let Some(raw) = translation_raw.filter(|raw| !raw.trim().is_empty()) {
        let mut translations = parse_lrc(raw)?;
        if translations.is_empty() {
            translations = parse_word_synced(raw, input.word_source)?;
        }
        document = merge_translations(&document, &translations)?;
    }
    Ok(Some(document))
}

fn append_metadata(
    document: LyricsDocument,
    metadata: Option<&LyricsDocument>,
) -> Result<LyricsDocument, LyricModelError> {
    let Some(metadata) = metadata.filter(|metadata| !metadata.is_empty()) else {
        return Ok(document);
    };
    let mut lines = document.lines.to_vec();
    lines.extend(metadata.lines.iter().cloned());
    lines.sort_by_key(|line| line.start_ms);
    LyricsDocument::new(document.source, lines)
}

/// Parse ordinary line-synchronised LRC without manufacturing sub-line data.
pub fn parse_lrc(raw: &str) -> Result<LyricsDocument, LyricModelError> {
    let mut entries = Vec::<(u64, String)>::new();
    for raw_line in raw.lines() {
        let tags: Vec<_> = LRC_TAG.captures_iter(raw_line).collect();
        if tags.is_empty() {
            continue;
        }
        let text = LRC_TAG.replace_all(raw_line, "").trim().to_owned();
        if text.is_empty()
            || text.starts_with("//")
            || COPYRIGHT_PATTERNS
                .iter()
                .any(|pattern| text.contains(pattern))
        {
            continue;
        }
        for tag in tags {
            if let Some(timestamp) = lrc_timestamp(&tag) {
                entries.push((timestamp, text.clone()));
            }
        }
    }
    entries.sort_by_key(|entry| entry.0);

    let mut lines = Vec::with_capacity(entries.len());
    for (index, (start_ms, text)) in entries.iter().enumerate() {
        let end_ms = entries
            .get(index + 1)
            .map_or(start_ms.saturating_add(5_000), |next| next.0)
            .max(*start_ms);
        lines.push(LyricsLine::new(
            *start_ms,
            end_ms,
            Arc::<str>::from(text.as_str()),
            None,
            TimingQuality::Line,
            vec![],
        )?);
    }
    LyricsDocument::new(LyricsSource::Lrc, lines)
}

/// Parse YRC/QRC-style word timing.
///
/// Each provider marker remains one span even when its text contains several
/// Unicode characters. A renderer may reveal the geometry within that span,
/// but the data layer never claims the provider sent per-character timing.
pub fn parse_word_synced(
    raw: &str,
    source: LyricsSource,
) -> Result<LyricsDocument, LyricModelError> {
    debug_assert!(matches!(source, LyricsSource::Yrc | LyricsSource::Qrc));
    let normalized = normalize_xml_line_breaks(raw);
    let mut lines = Vec::new();

    for raw_line in normalized.lines().map(str::trim) {
        let Some(header) = WORD_LINE.captures(raw_line) else {
            continue;
        };
        let line_start = header[1].parse::<u64>().unwrap_or(0);
        let declared_duration = header[2].parse::<u64>().unwrap_or(0);
        let rest = &raw_line[header.get(0).unwrap().end()..];
        let markers: Vec<_> = WORD_MARKER.find_iter(rest).collect();
        if markers.is_empty() {
            continue;
        }
        let marker_before_text = rest[..markers[0].start()].trim().is_empty();

        let mut segments = Vec::<(u64, u64, &str)>::new();
        for (index, marker) in markers.iter().enumerate() {
            let captures = WORD_MARKER.captures(marker.as_str()).unwrap();
            let start = captures[1].parse::<u64>().unwrap_or(0);
            let duration = captures[2].parse::<u64>().unwrap_or(0);
            if duration == 0 {
                continue;
            }
            let text = if marker_before_text {
                let end = markers
                    .get(index + 1)
                    .map_or(rest.len(), |next| next.start());
                &rest[marker.end()..end]
            } else {
                let start = index
                    .checked_sub(1)
                    .map_or(0, |previous| markers[previous].end());
                &rest[start..marker.start()]
            };
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            if !text.is_empty() {
                segments.push((start, duration, text));
            }
        }
        if segments.is_empty() {
            continue;
        }

        let relative = line_start > 0
            && segments.iter().all(|(start, duration, _)| {
                *start < line_start && start.saturating_add(*duration) <= declared_duration
            });

        let mut text = String::new();
        let mut spans = Vec::with_capacity(segments.len());
        for (start, duration, segment_text) in segments {
            let start_ms = if relative {
                line_start.saturating_add(start)
            } else {
                start
            };
            let range_start = text.len();
            text.push_str(segment_text);
            let range_end = text.len();
            spans.push(TimedSpan::new(
                range_start..range_end,
                start_ms,
                start_ms.saturating_add(duration),
            ));
        }

        let timed_end = spans
            .iter()
            .map(|span| span.end_ms)
            .max()
            .unwrap_or(line_start);
        let line_end = line_start.saturating_add(declared_duration).max(timed_end);
        let Ok(line) = LyricsLine::new(
            line_start,
            line_end,
            text,
            None,
            TimingQuality::Source,
            spans,
        ) else {
            // A malformed provider line must not invalidate every usable line
            // in the document. It is skipped rather than silently clamped.
            continue;
        };
        lines.push(line);
    }
    lines.sort_by_key(|line| line.start_ms);
    LyricsDocument::new(source, lines)
}

/// Parse NCM's JSON credit/metadata lines as line-synchronised text.
///
/// This format contains only a line timestamp. The legacy parser generated a
/// 1 ms offset and zero duration for each character, which made metadata look
/// like karaoke. V2 explicitly keeps it line-only.
pub fn parse_json_lines(raw: &str) -> Result<LyricsDocument, LyricModelError> {
    let mut entries = Vec::<(u64, String)>::new();
    for raw_line in raw.lines().map(str::trim) {
        if !raw_line.starts_with('{') {
            continue;
        }
        let Ok(line) = serde_json::from_str::<JsonLine>(raw_line) else {
            continue;
        };
        let text: String = line
            .content
            .into_iter()
            .filter_map(|part| part.text)
            .collect();
        if !text.is_empty() {
            entries.push((line.start_ms, text));
        }
    }
    entries.sort_by_key(|entry| entry.0);

    let mut lines = Vec::with_capacity(entries.len());
    for (index, (start_ms, text)) in entries.iter().enumerate() {
        let end_ms = entries
            .get(index + 1)
            .map_or(start_ms.saturating_add(5_000), |next| next.0)
            .max(*start_ms);
        lines.push(LyricsLine::new(
            *start_ms,
            end_ms,
            text.as_str(),
            None,
            TimingQuality::Line,
            vec![],
        )?);
    }
    LyricsDocument::new(LyricsSource::Unknown, lines)
}

/// Return a new document with one-to-one translations attached.
///
/// A translation timestamp inside a source line wins. Otherwise the nearest
/// unused line within three seconds is selected. Keeping this operation in the
/// domain layer makes both Cairo and OpenGL consume exactly the same pairing.
pub fn merge_translations(
    document: &LyricsDocument,
    translations: &LyricsDocument,
) -> Result<LyricsDocument, LyricModelError> {
    if document.is_empty() || translations.is_empty() {
        return Ok(document.clone());
    }

    let mut lines = document.lines.to_vec();
    let mut used = vec![false; lines.len()];
    for translation in translations
        .lines
        .iter()
        .filter(|line| !line.text.is_empty())
    {
        let interval_match = lines
            .iter()
            .enumerate()
            .filter(|(index, line)| {
                !used[*index]
                    && line.start_ms <= translation.start_ms
                    && translation.start_ms < line.end_ms
            })
            .map(|(index, _)| index)
            .next_back();

        let target = interval_match.or_else(|| {
            lines
                .iter()
                .enumerate()
                .filter(|(index, _)| !used[*index])
                .filter_map(|(index, line)| {
                    let difference = line.start_ms.abs_diff(translation.start_ms);
                    (difference <= TRANSLATION_MATCH_LIMIT_MS).then_some((index, difference))
                })
                .min_by_key(|(index, difference)| {
                    (*difference, usize::MAX - lines[*index].text.len())
                })
                .map(|(index, _)| index)
        });

        if let Some(index) = target {
            lines[index].translation = Some(translation.text.clone());
            used[index] = true;
        }
    }
    LyricsDocument::new(document.source, lines)
}

fn normalize_xml_line_breaks(raw: &str) -> String {
    raw.replace("&#10;", "\n")
        .replace("&#xA;", "\n")
        .replace("&#x0A;", "\n")
        .replace("&#13;", "\r")
        .replace("&#xD;", "\r")
        .replace("&#x0D;", "\r")
}

fn lrc_timestamp(captures: &regex::Captures<'_>) -> Option<u64> {
    let minutes = captures[1].parse::<u64>().ok()?;
    let seconds = captures[2].parse::<u64>().ok()?;
    let fraction = captures.get(3).map_or("", |value| value.as_str());
    let millis = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u64>().ok()? * 100,
        2 => fraction.parse::<u64>().ok()? * 10,
        _ => fraction[..3].parse::<u64>().ok()?,
    };
    Some(minutes * 60_000 + seconds * 1_000 + millis)
}

#[derive(Debug, Deserialize)]
struct JsonLine {
    #[serde(rename = "t")]
    start_ms: u64,
    #[serde(rename = "c")]
    content: Vec<JsonPart>,
}

#[derive(Debug, Deserialize)]
struct JsonPart {
    #[serde(rename = "tx")]
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_fixture_is_strictly_line_timed() {
        let document = parse_lrc(include_str!("../../tests/fixtures/lyrics/basic.lrc")).unwrap();
        assert_eq!(document.lines.len(), 2);
        assert_eq!(document.lines[0].start_ms, 1_000);
        assert_eq!(document.lines[0].end_ms, 4_250);
        assert_eq!(document.lines[0].timing_quality, TimingQuality::Line);
        assert!(document.lines[0].spans.is_empty());
    }

    #[test]
    fn yrc_fixture_preserves_multi_character_source_segments() {
        let document = parse_word_synced(
            include_str!("../../tests/fixtures/lyrics/word-timed.yrc"),
            LyricsSource::Yrc,
        )
        .unwrap();
        let line = &document.lines[0];
        assert_eq!(&*line.text, "Hello 世界");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(&line.text[line.spans[0].text_range.clone()], "Hello ");
        assert_eq!(&line.text[line.spans[1].text_range.clone()], "世界");
    }

    #[test]
    fn qrc_fixture_supports_markers_after_text_and_relative_time() {
        let document = parse_word_synced(
            include_str!("../../tests/fixtures/lyrics/word-timed.qrc"),
            LyricsSource::Qrc,
        )
        .unwrap();
        let line = &document.lines[0];
        assert_eq!(&*line.text, "Keep moving");
        assert_eq!(line.spans[0].start_ms, 10_000);
        assert_eq!(line.spans[1].start_ms, 10_600);
    }

    #[test]
    fn json_fixture_never_becomes_fake_karaoke() {
        let document =
            parse_json_lines(include_str!("../../tests/fixtures/lyrics/metadata.jsonl")).unwrap();
        assert_eq!(&*document.lines[0].text, "作词: Linn");
        assert_eq!(document.lines[0].timing_quality, TimingQuality::Line);
        assert!(document.lines[0].spans.is_empty());
    }

    #[test]
    fn translations_are_interval_matched_one_to_one() {
        let main = parse_lrc("[00:01.00]one\n[00:04.00]two").unwrap();
        let translations = parse_lrc("[00:01.10]一\n[00:04.20]二").unwrap();
        let merged = merge_translations(&main, &translations).unwrap();
        assert_eq!(merged.lines[0].translation.as_deref(), Some("一"));
        assert_eq!(merged.lines[1].translation.as_deref(), Some("二"));
        assert!(main.lines.iter().all(|line| line.translation.is_none()));
    }

    #[test]
    fn distant_translation_is_not_attached() {
        let main = parse_lrc("[00:10.00]one").unwrap();
        let translations = parse_lrc("[00:01.00]一").unwrap();
        let merged = merge_translations(&main, &translations).unwrap();
        assert_eq!(merged.lines[0].translation, None);
    }

    fn raw<'a>(lyric: Option<&'a str>, word_synced: Option<&'a str>) -> RawLyrics<'a> {
        RawLyrics {
            lyric,
            translation: None,
            word_synced,
            word_translation: None,
            word_source: LyricsSource::Yrc,
            is_pure_music: false,
        }
    }

    #[test]
    fn unified_entry_prefers_real_word_timing_and_merges_metadata() {
        let word = concat!(
            "{\"t\":0,\"c\":[{\"tx\":\"作词: Linn\"}]}\n",
            "[1000,2000](1000,1000,0)歌(2000,1000,0)词"
        );
        let document = parse_document(raw(Some("[00:01.00]fallback"), Some(word)))
            .unwrap()
            .unwrap();
        assert_eq!(document.source, LyricsSource::Yrc);
        assert_eq!(document.lines.len(), 2);
        assert_eq!(document.lines[1].timing_quality, TimingQuality::Source);
    }

    #[test]
    fn metadata_only_word_payload_does_not_hide_lrc() {
        let metadata = "{\"t\":0,\"c\":[{\"tx\":\"作词: Linn\"}]}";
        let document = parse_document(raw(Some("[00:01.00]actual lyric"), Some(metadata)))
            .unwrap()
            .unwrap();
        assert_eq!(document.source, LyricsSource::Lrc);
        assert!(
            document
                .lines
                .iter()
                .any(|line| &*line.text == "actual lyric")
        );
    }

    #[test]
    fn unified_entry_prefers_ttml() {
        let ttml = include_str!("../../tests/fixtures/lyrics/karaoke.ttml");
        let document = parse_document(raw(Some(ttml), Some("[1000,1000](1000,1000)wrong")))
            .unwrap()
            .unwrap();
        assert_eq!(document.source, LyricsSource::Ttml);
        assert_eq!(&*document.lines[0].text, "Hello 世界");
    }

    #[test]
    fn unified_entry_returns_none_for_pure_music() {
        let mut input = raw(Some("[00:01.00]ignored"), None);
        input.is_pure_music = true;
        assert_eq!(parse_document(input).unwrap(), None);
    }

    #[test]
    fn unknown_word_payload_cannot_enter_the_yrc_qrc_parser() {
        let input = RawLyrics {
            lyric: Some("[00:01.00]plain fallback"),
            translation: None,
            word_synced: Some("[broken](payload)"),
            word_translation: None,
            word_source: LyricsSource::Unknown,
            is_pure_music: false,
        };

        let document = parse_document(input).unwrap().unwrap();
        assert_eq!(document.source, LyricsSource::Lrc);
        assert_eq!(&*document.lines[0].text, "plain fallback");
    }
}
