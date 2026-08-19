use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

use crate::api::model::LyricDetail;
use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};
use crate::utils::ttml::{is_ttml, parse_ttml};

// ─── 预编译正则 ────────────────────────────────────────────────────────────────

static LRC_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\d{2,3}:\d{2}(?:[.:]\d{2,3})?\]").unwrap());

static YRC_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(\d+),(\d+)\]").unwrap());

// QRC payloads exist in both `(start,duration)` and
// `(start,duration,phoneme-index)` forms. QQ currently returns the former for
// many songs, while some NCM/YRC payloads use the latter.
static YRC_CHAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d+),(\d+)(?:,\d+)?\)").unwrap());

// ─── 版权信息过滤 ─────────────────────────────────────────────────────────────

const COPYRIGHT_PATTERNS: &[&str] = &["著作权", "QQ音乐", "腾讯音乐", "未经许可", "不得转载"];

fn is_copyright_line(text: &str) -> bool {
    COPYRIGHT_PATTERNS.iter().any(|pat| text.contains(pat))
}

fn is_lyric_line(line: &str) -> bool {
    let stripped = line.trim();
    if stripped.is_empty() {
        return false;
    }
    if stripped.starts_with('{') {
        return false;
    }
    if stripped.starts_with("//") {
        return false;
    }
    true
}

// ─── JSON 逐字格式解析 ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonVerbatimLine {
    t: u64,
    c: Vec<JsonVerbatimChar>,
}

#[derive(Debug, Deserialize)]
struct JsonVerbatimChar {
    tx: Option<String>,
    #[serde(default)]
    li: Option<String>,
    #[serde(default)]
    or: Option<String>,
}

pub fn parse_verbatim_json(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();

    for raw_line in raw.lines() {
        let raw_line = raw_line.trim();
        if raw_line.is_empty() || !raw_line.starts_with('{') {
            continue;
        }
        let Ok(vl) = serde_json::from_str::<JsonVerbatimLine>(raw_line) else {
            continue;
        };
        if vl.c.is_empty() {
            continue;
        }

        let mut chars: Vec<LyricChar> = Vec::new();
        let mut offset = 0u64;
        for jc in &vl.c {
            let tx = jc.tx.as_deref().unwrap_or("");
            if tx.is_empty() {
                continue;
            }
            for ch in tx.chars() {
                chars.push(LyricChar {
                    ch: ch.to_string(),
                    start: vl.t + offset,
                    duration: 0,
                });
                offset = offset.saturating_add(1);
            }
        }
        if chars.is_empty() {
            continue;
        }

        let text: String = chars.iter().map(|c| c.ch.as_str()).collect();
        let duration = offset.max(1);

        lines.push(LyricLine {
            start: vl.t,
            duration,
            text,
            kind: LyricLineKind::Verbatim(chars),
            translation: None,
        });
    }

    lines
}

// ─── YRC 解析 ─────────────────────────────────────────────────────────────────

pub fn parse_yrc(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();

    // QQ wraps QRC in XML and frequently serializes newlines as numeric XML
    // entities. Normalize them here as well as in the QQ decoder so this
    // parser remains safe for NCM/other callers that provide the raw payload.
    let normalized = raw
        .replace("&#10;", "\n")
        .replace("&#xA;", "\n")
        .replace("&#x0A;", "\n")
        .replace("&#13;", "\r")
        .replace("&#xD;", "\r")
        .replace("&#x0D;", "\r");

    for raw_line in normalized.lines() {
        let raw_line = raw_line.trim();
        if !is_lyric_line(raw_line) {
            continue;
        }

        let Some(header_caps) = YRC_HEADER_RE.captures_at(raw_line, 0) else {
            continue;
        };

        let line_start: u64 = header_caps[1].parse().unwrap_or(0);
        let line_duration: u64 = header_caps[2].parse().unwrap_or(0);
        let rest = &raw_line[header_caps.get(0).unwrap().end()..];

        // QRC has been observed in both forms:
        //   (start,duration)text   (time marker before text)
        //   text(start,duration)   (time marker after text; used by the
        //   reference Python parser). Split around every marker and choose
        //   the association from whether there is text before the first one.
        let matches: Vec<_> = YRC_CHAR_RE.find_iter(rest).collect();
        let mut timed_segments: Vec<(u64, u64, String)> = Vec::new();
        let marker_before_text = matches
            .first()
            .is_some_and(|marker| rest[..marker.start()].trim().is_empty());

        for (index, marker) in matches.iter().enumerate() {
            let captures = YRC_CHAR_RE
                .captures(marker.as_str())
                .expect("YRC marker was produced by the same regex");
            let ch_start: u64 = captures[1].parse().unwrap_or(0);
            let ch_duration: u64 = captures[2].parse().unwrap_or(0);

            let text = if marker_before_text {
                let start = marker.end();
                let end = matches
                    .get(index + 1)
                    .map_or(rest.len(), |next| next.start());
                &rest[start..end]
            } else {
                let start = if index == 0 {
                    0
                } else {
                    matches[index - 1].end()
                };
                &rest[start..marker.start()]
            };

            // Spaces are meaningful lyric characters (especially in English
            // QRC). Do not trim each segment: doing so turns `Hello world`
            // into `Helloworld` and also drops explicitly timed space tokens.
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            let ch_count = text.chars().count();
            if ch_count == 0 {
                continue;
            }
            timed_segments.push((ch_start, ch_duration, text.to_string()));
        }

        // Some QQ QRC variants store character starts relative to the line
        // (`(0,300)字...`), while others store absolute song positions. The
        // relative form is unambiguous when every character fits within the
        // line duration and starts before the line start.
        let relative_timing = line_start > 0
            && !timed_segments.is_empty()
            && timed_segments.iter().all(|(start, duration, _)| {
                *start < line_start && start.saturating_add(*duration) <= line_duration
            });

        let mut chars: Vec<LyricChar> = Vec::new();
        for (ch_start, ch_duration, text) in timed_segments {
            let ch_start = if relative_timing {
                line_start.saturating_add(ch_start)
            } else {
                ch_start
            };
            let ch_count = text.chars().count();
            let per_dur = (ch_duration / ch_count as u64).max(1);
            for (offset, ch) in text.chars().enumerate() {
                chars.push(LyricChar {
                    ch: ch.to_string(),
                    start: ch_start + offset as u64 * per_dur,
                    duration: per_dur,
                });
            }
        }

        if chars.is_empty() {
            continue;
        }

        let text: String = chars.iter().map(|c| c.ch.as_str()).collect();

        lines.push(LyricLine {
            start: line_start,
            duration: line_duration,
            text,
            kind: LyricLineKind::Verbatim(chars),
            translation: None,
        });
    }

    lines
}

// ─── LRC 解析 ─────────────────────────────────────────────────────────────────

fn parse_lrc_timestamp(s: &str) -> Option<u64> {
    // 格式: mm:ss.xx 或 mm:ss.xxx 或 mm:ss
    let colon = s.find(':')?;
    let mm = s[..colon].trim().parse::<u64>().ok()?;

    let rest = &s[colon + 1..];
    let (ss_str, ms_str) = if let Some(dot) = rest.find(&['.', ':'][..]) {
        (&rest[..dot], &rest[dot + 1..])
    } else {
        (rest, "0")
    };

    let ss = ss_str.trim().parse::<u64>().ok()?;

    let ms = match ms_str.len() {
        0 => 0,
        1 => ms_str.parse::<u64>().ok()? * 100,
        2 => ms_str.parse::<u64>().ok()? * 10,
        _ => ms_str[..3].parse::<u64>().ok()?,
    };

    Some(mm * 60_000 + ss * 1_000 + ms)
}

pub fn parse_lrc(raw: &str) -> Vec<LyricLine> {
    let mut entries: Vec<(u64, String)> = Vec::new();

    for raw_line in raw.lines() {
        if !is_lyric_line(raw_line) {
            continue;
        }

        let tags: Vec<&str> = LRC_TAG_RE.find_iter(raw_line).map(|m| m.as_str()).collect();
        if tags.is_empty() {
            continue;
        }

        let text = LRC_TAG_RE.replace_all(raw_line, "").trim().to_string();
        if text.is_empty() {
            continue;
        }
        if text.starts_with("//") {
            continue;
        }
        if is_copyright_line(&text) {
            continue;
        }

        for tag in &tags {
            let inner = &tag[1..tag.len() - 1];
            if let Some(ms) = parse_lrc_timestamp(inner) {
                entries.push((ms, text.clone()));
            }
        }
    }

    entries.sort_by_key(|(t, _)| *t);

    let n = entries.len();
    let mut lines = Vec::with_capacity(n);

    for i in 0..n {
        let (start, text) = entries[i].clone();
        let duration = if i + 1 < n {
            entries[i + 1].0.saturating_sub(start)
        } else {
            5000
        };

        lines.push(LyricLine {
            start,
            duration,
            text,
            kind: LyricLineKind::Plain,
            translation: None,
        });
    }

    lines
}

// ─── 翻译配对 ─────────────────────────────────────────────────────────────────
//
// 策略：
//   1. 优先匹配翻译时间戳落在主歌词行 [start, start+duration) 区间内的行
//   2. 若无区间匹配，退化为最近邻（差值 < MATCH_THRESHOLD_MS）
//   3. 最近邻时有并列按原文长度优先
//   4. 每行原文最多匹配一条翻译（1:1），避免多条翻译挤在一行

const MATCH_THRESHOLD_MS: u64 = 3000;

pub fn inject_translations(main_lines: &mut Vec<LyricLine>, t_raw: &str) {
    // QQ's translated QRC is word-timed too; use its line timing when it is
    // not an ordinary LRC payload.
    let mut t_lines = parse_lrc(t_raw);
    if t_lines.is_empty() {
        t_lines = parse_yrc(t_raw);
    }
    if t_lines.is_empty() || main_lines.is_empty() {
        return;
    }

    let usable: Vec<_> = t_lines.iter().filter(|tl| !tl.text.is_empty()).collect();
    if usable.is_empty() {
        return;
    }

    let mut used_orig: Vec<bool> = vec![false; main_lines.len()];

    for tl in usable {
        // 策略 1: 区间匹配
        let interval_idx = main_lines
            .iter()
            .enumerate()
            .filter(|(i, l)| {
                !used_orig[*i] && l.start <= tl.start && l.start + l.duration > tl.start
            })
            .map(|(i, _)| i)
            .last();

        match interval_idx {
            Some(idx) => {
                main_lines[idx].translation = Some(tl.text.clone());
                used_orig[idx] = true;
            }
            None => {
                // 策略 2: 最近邻
                let mut best_idx: Option<usize> = None;
                let mut best_diff = u64::MAX;

                for (i, line) in main_lines.iter().enumerate() {
                    if used_orig[i] {
                        continue;
                    }
                    let diff = if line.start >= tl.start {
                        line.start - tl.start
                    } else {
                        tl.start - line.start
                    };

                    if diff > MATCH_THRESHOLD_MS {
                        continue;
                    }

                    let is_better = diff < best_diff
                        || (diff == best_diff
                            && best_idx.is_some()
                            && line.text.len() > main_lines[best_idx.unwrap()].text.len());

                    if is_better {
                        best_diff = diff;
                        best_idx = Some(i);
                    }
                }

                if let Some(idx) = best_idx {
                    main_lines[idx].translation = Some(tl.text.clone());
                    used_orig[idx] = true;
                }
            }
        }
    }
}

// ─── 入口：从 LyricDetail 解析并返回最终行列表 ────────────────────────────────

pub fn parse_lyric(lyric: &LyricDetail) -> Option<Vec<LyricLine>> {
    if lyric.is_pure_music {
        return None;
    }

    if let Some(raw) = lyric.lyric.as_deref().filter(|raw| is_ttml(raw)) {
        return parse_ttml(raw).ok().filter(|lines| !lines.is_empty());
    }

    let mut lines = if let Some(yrc) = &lyric.yrc {
        let mut parsed = parse_verbatim_json(yrc);
        let mut qrc = parse_yrc(yrc);
        log::debug!(
            "[lyrics][parse] yrc_bytes={} qrc_lines={} qrc_word_chars={} json_lines={}",
            yrc.len(),
            qrc.len(),
            qrc.iter()
                .map(|line| match &line.kind {
                    LyricLineKind::Verbatim(chars) => chars.len(),
                    LyricLineKind::Plain => 0,
                })
                .sum::<usize>(),
            parsed.len()
        );
        parsed.append(&mut qrc);
        if !parsed.is_empty() {
            parsed
        } else {
            let header_count = YRC_HEADER_RE.find_iter(yrc).count();
            let marker_count = YRC_CHAR_RE.find_iter(yrc).count();
            log::warn!(
                "[lyrics][parse] QRC produced zero lines bytes={} headers={} markers={}",
                yrc.len(),
                header_count,
                marker_count
            );
            eprintln!(
                "[lyrics] QRC parse zero lines bytes={} headers={} markers={}",
                yrc.len(),
                header_count,
                marker_count
            );
            parse_mixed(&lyric.lyric)
        }
    } else {
        parse_mixed(&lyric.lyric)
    };

    if lines.is_empty() {
        return None;
    }

    // 注入翻译：优先 ytlrc，其次 tlyric
    let t_source = lyric.ytlrc.as_deref().or(lyric.tlyric.as_deref());
    if let Some(t_raw) = t_source {
        if !t_raw.is_empty() {
            inject_translations(&mut lines, t_raw);
        }
    }

    Some(lines)
}

fn parse_mixed(raw: &Option<String>) -> Vec<LyricLine> {
    let raw = match raw.as_deref() {
        Some(r) if !r.is_empty() => r,
        _ => return Vec::new(),
    };
    let mut lines = parse_verbatim_json(raw);
    lines.append(&mut parse_lrc(raw));
    lines.sort_by_key(|l| l.start);
    lines
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "lyric_parse_tests.rs"]
mod tests;
