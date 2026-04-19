// lyric_parse.rs
// 负责：yrc 逐字解析、lrc 普通解析、翻译行配对


use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

// ─── YRC 解析 ─────────────────────────────────────────────────────────────────
//
// YRC 格式示例（网易云逐字歌词）：
// [1740,2160](1740,480,0)遥(2220,300,0)远(2520,300,0)的(2820,360,0)东(3180,360,0)方
//
// 行头：[行起始ms, 行时长ms]
// 字：(字起始ms, 字时长ms, 0)汉字

pub fn parse_yrc(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();

    for raw_line in raw.lines() {
        let raw_line = raw_line.trim();
        if raw_line.is_empty() {
            continue;
        }

        // 解析行头 [start,duration]
        let Some(line_header_end) = raw_line.find(']') else {
            continue;
        };
        let header = &raw_line[1..line_header_end]; // "start,duration"
        let rest = &raw_line[line_header_end + 1..];

        let (line_start, line_dur) = parse_pair(header).unwrap_or((0, 0));

        // 解析逐字 (start,duration,0)字
        let mut chars: Vec<LyricChar> = Vec::new();
        let mut cursor = rest;

        while let Some(open) = cursor.find('(') {
            let Some(close) = cursor.find(')') else { break; };
            if close < open {
                break;
            }
            let char_meta = &cursor[open + 1..close];
            let (ch_start, ch_dur) = parse_pair(char_meta).unwrap_or((0, 0));

            // 字符紧跟在 ')' 之后，到下一个 '(' 或末尾
            let after_close = &cursor[close + 1..];
            let ch_text = match after_close.find('(') {
                Some(next_open) => &after_close[..next_open],
                None => after_close,
            };

            // ch_text 可能包含多个 Unicode 字符（部分格式会把空格也包进来）
            // 逐个拆成单字，共享同一时间戳（简单平均）
            let ch_count = ch_text.chars().count();
            if ch_count > 0 {
                let per_dur = ch_dur / ch_count as u64;
                let mut offset = 0u64;
                for ch in ch_text.chars() {
                    chars.push(LyricChar {
                        ch: ch.to_string(),
                        start: ch_start + offset,
                        duration: per_dur.max(1),
                    });
                    offset += per_dur;
                }
            }

            cursor = &cursor[close + 1..];
        }

        if chars.is_empty() {
            continue;
        }

        let text: String = chars.iter().map(|c| c.ch.as_str()).collect();

        lines.push(LyricLine {
            start: line_start,
            duration: line_dur,
            text,
            kind: LyricLineKind::Verbatim(chars),
            translation: None,
        });
    }

    lines
}

// ─── LRC 解析 ─────────────────────────────────────────────────────────────────
//
// 普通 LRC 格式：[mm:ss.xx]歌词文本
// 支持多时间戳同行：[00:01.00][00:30.00]歌词

pub fn parse_lrc(raw: &str) -> Vec<LyricLine> {
    let mut entries: Vec<(u64, String)> = Vec::new();

    for raw_line in raw.lines() {
        let raw_line = raw_line.trim();
        if raw_line.is_empty() {
            continue;
        }

        let mut cursor = raw_line;
        let mut timestamps: Vec<u64> = Vec::new();

        // 收集所有时间戳标签
        while cursor.starts_with('[') {
            let Some(close) = cursor.find(']') else { break; };
            let tag = &cursor[1..close];
            if let Some(ms) = parse_lrc_timestamp(tag) {
                timestamps.push(ms);
            }
            cursor = &cursor[close + 1..];
        }

        let text = cursor.trim().to_string();

        // 跳过元数据行（ti:, ar:, al: 等）和空行
        if text.is_empty() || timestamps.is_empty() {
            continue;
        }

        for ts in timestamps {
            entries.push((ts, text.clone()));
        }
    }

    // 按时间排序
    entries.sort_by_key(|(t, _)| *t);

    // 推算每行的 duration = 下一行 start - 本行 start，最后一行给默认值
    let n = entries.len();
    let mut lines = Vec::with_capacity(n);

    for i in 0..n {
        let (start, text) = entries[i].clone();
        let duration = if i + 1 < n {
            entries[i + 1].0.saturating_sub(start)
        } else {
            5000 // 最后一行默认 5 秒
        };

        lines.push(LyricLine {
            start,
            duration,
            text: text.clone(),
            kind: LyricLineKind::Plain,
            translation: None,
        });
    }

    lines
}

// ─── 翻译配对 ─────────────────────────────────────────────────────────────────
//
// 策略：时间戳最近邻匹配。
// 对每一条翻译行，找主歌词中 start 最接近的行，如果差值 < MATCH_THRESHOLD_MS 则配对。
// 一条主歌词行只配对一条翻译（取最近的）。

const MATCH_THRESHOLD_MS: u64 = 800;

/// 将翻译行配对注入到主歌词行的 translation 字段。
/// 原地修改 main_lines。
pub fn inject_translations(main_lines: &mut Vec<LyricLine>, tlyric_raw: &str) {
    let tl_lines = parse_lrc(tlyric_raw);
    if tl_lines.is_empty() {
        return;
    }

    // 预先收集主歌词的时间戳，做最近邻匹配
    // 用贪心：遍历翻译行，每行找最近的主歌词行，已配对的不再重复
    // （翻译行通常比主歌词少或相等，不做双向去重，允许多条翻译映射到同一行）

    for tl in &tl_lines {
        // 找与 tl.start 最接近的主歌词行
        let best = main_lines
            .iter_mut()
            .min_by_key(|l| (l.start as i64 - tl.start as i64).unsigned_abs());

        if let Some(line) = best {
            let diff = (line.start as i64 - tl.start as i64).unsigned_abs();
            if diff < MATCH_THRESHOLD_MS {
                // 如果已有翻译（多条翻译时间戳相近），用换行拼接
                match &mut line.translation {
                    Some(existing) => {
                        existing.push('\n');
                        existing.push_str(&tl.text);
                    }
                    None => {
                        line.translation = Some(tl.text.clone());
                    }
                }
            }
        }
    }
}

// ─── 入口：从 Lyric 结构体解析并返回最终行列表 ────────────────────────────────

use crate::api::model::LyricDetail;

/// 解析 API 返回的 Lyric，优先使用 yrc 逐字歌词，fallback 到 lyric。
/// 翻译自动配对注入。
/// 返回 None 表示纯音乐或无歌词。
pub fn parse_lyric(lyric: &LyricDetail) -> Option<Vec<LyricLine>> {
    if lyric.is_pure_music {
        return None;
    }

    // 优先 yrc
    let mut lines = if let Some(yrc) = &lyric.yrc {
        let parsed = parse_yrc(yrc);
        if !parsed.is_empty() {
            parsed
        } else {
            // yrc 字段存在但解析为空，fallback
            lyric.lyric.as_deref().map(parse_lrc).unwrap_or_default()
        }
    } else {
        lyric.lyric.as_deref().map(parse_lrc).unwrap_or_default()
    };

    if lines.is_empty() {
        return None;
    }

    // 注入翻译
    if let Some(tlyric) = &lyric.tlyric {
        if !tlyric.is_empty() {
            inject_translations(&mut lines, tlyric);
        }
    }

    Some(lines)
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 解析 "a,b[,c...]" 格式，返回前两个数字
fn parse_pair(s: &str) -> Option<(u64, u64)> {
    let mut parts = s.splitn(3, ',');
    let a = parts.next()?.trim().parse::<u64>().ok()?;
    let b = parts.next()?.trim().parse::<u64>().ok()?;
    Some((a, b))
}

/// 解析 LRC 时间戳 "mm:ss.xx" 或 "mm:ss.xxx"，返回毫秒
fn parse_lrc_timestamp(s: &str) -> Option<u64> {
    // 格式：mm:ss.xx 或 mm:ss.xxx
    let colon = s.find(':')?;
    let mm = s[..colon].trim().parse::<u64>().ok()?;

    let rest = &s[colon + 1..];
    let (ss_str, ms_str) = if let Some(dot) = rest.find('.') {
        (&rest[..dot], &rest[dot + 1..])
    } else {
        (rest, "0")
    };

    let ss = ss_str.trim().parse::<u64>().ok()?;

    // ms_str 可能是 2 位（百分之一秒）或 3 位（毫秒）
    let ms = match ms_str.len() {
        0 => 0,
        1 => ms_str.parse::<u64>().ok()? * 100,
        2 => ms_str.parse::<u64>().ok()? * 10,
        _ => ms_str[..3].parse::<u64>().ok()?,
    };

    Some(mm * 60_000 + ss * 1_000 + ms)
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lrc_timestamp() {
        assert_eq!(parse_lrc_timestamp("00:30.50"), Some(30_500));
        assert_eq!(parse_lrc_timestamp("01:02.03"), Some(62_030));
        assert_eq!(parse_lrc_timestamp("00:00.000"), Some(0));
        assert_eq!(parse_lrc_timestamp("03:25.120"), Some(205_120));
    }

    #[test]
    fn test_parse_lrc_basic() {
        let raw = "[00:01.00]第一行\n[00:03.50]第二行\n[00:06.00]第三行";
        let lines = parse_lrc(raw);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start, 1_000);
        assert_eq!(lines[0].duration, 2_500);
        assert_eq!(lines[0].text, "第一行");
        assert!(matches!(lines[0].kind, LyricLineKind::Plain));
    }

    #[test]
    fn test_inject_translations() {
        let raw_main = "[00:01.00]Hello world\n[00:04.00]Goodbye";
        let raw_tl   = "[00:01.10]你好世界\n[00:04.20]再见";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        assert_eq!(lines[0].translation.as_deref(), Some("你好世界"));
        assert_eq!(lines[1].translation.as_deref(), Some("再见"));
    }
}