use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

use crate::api::model::LyricDetail;
use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

// ─── 预编译正则 ────────────────────────────────────────────────────────────────

static LRC_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\d{2,3}:\d{2}(?:[.:]\d{2,3})?\]").unwrap());

static YRC_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[(\d+),(\d+)\]").unwrap());

static YRC_CHAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d+),(\d+),\d+\)").unwrap());

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

    for raw_line in raw.lines() {
        if !is_lyric_line(raw_line) {
            continue;
        }

        let Some(header_caps) = YRC_HEADER_RE.captures_at(raw_line, 0) else {
            continue;
        };

        let line_start: u64 = header_caps[1].parse().unwrap_or(0);
        let line_duration: u64 = header_caps[2].parse().unwrap_or(0);
        let rest = &raw_line[header_caps.get(0).unwrap().end()..];

        let mut chars: Vec<LyricChar> = Vec::new();
        for cm in YRC_CHAR_RE.captures_iter(rest) {
            let ch_start: u64 = cm[1].parse().unwrap_or(0);
            let ch_duration: u64 = cm[2].parse().unwrap_or(0);
            let m = cm.get(0).unwrap();
            let after_close = m.end();

            let next_paren = rest[after_close..].find('(');
            let ch_text_slice = match next_paren {
                Some(np) => &rest[after_close..after_close + np],
                None => &rest[after_close..],
            };

            let ch_count = ch_text_slice.chars().count();
            if ch_count > 0 {
                let per_dur = ch_duration / ch_count as u64;
                let mut offset = 0u64;
                for ch in ch_text_slice.chars() {
                    chars.push(LyricChar {
                        ch: ch.to_string(),
                        start: ch_start + offset,
                        duration: per_dur.max(1),
                    });
                    offset += per_dur;
                }
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

        let tags: Vec<&str> = LRC_TAG_RE
            .find_iter(raw_line)
            .map(|m| m.as_str())
            .collect();
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
    let t_lines = parse_lrc(t_raw);
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
            .filter(|(i, l)| !used_orig[*i] && l.start <= tl.start && l.start + l.duration > tl.start)
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
                            && line.text.len()
                                > main_lines[best_idx.unwrap()].text.len());

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

    let mut lines = if let Some(yrc) = &lyric.yrc {
        let mut parsed = parse_verbatim_json(yrc);
        parsed.append(&mut parse_yrc(yrc));
        if !parsed.is_empty() {
            parsed
        } else {
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
