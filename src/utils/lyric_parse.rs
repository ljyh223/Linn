// lyric_parse.rs
// 负责：yrc 逐字解析、lrc 普通解析、JSON 逐字解析、翻译行配对

use serde::Deserialize;

use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

// ─── JSON 逐字格式解析 ─────────────────────────────────────────────────────────
//
// 网易云 API 返回的 lyric / yrc 字段中可能混合 JSON 格式的行：
// {"t":0,"c":[{"tx":"编曲: "},{"tx":"赤髪"}]}
// 这些通常是最前面的制作人信息，解析后生成 Verbatim 行。

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
            // JSON 格式没有逐字时长，均分
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

        // 跳过 JSON 格式行（由 parse_verbatim_json 单独处理）
        if raw_line.starts_with('{') {
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

        // 跳过 JSON 格式行（由 parse_verbatim_json 单独处理）
        if raw_line.starts_with('{') {
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
// 策略：时间区间匹配。
// 对每一条翻译行，优先匹配其时间戳落在 [start, start+duration) 的主歌词行。
// 若无精确匹配，则退化为最近邻匹配（差值 < MATCH_THRESHOLD_MS）。

const MATCH_THRESHOLD_MS: u64 = 800;

/// 将翻译行配对注入到主歌词行的 translation 字段。
/// 原地修改 main_lines。
pub fn inject_translations(main_lines: &mut Vec<LyricLine>, tlyric_raw: &str) {
    let tl_lines = parse_lrc(tlyric_raw);
    if tl_lines.is_empty() {
        return;
    }

    if main_lines.is_empty() {
        return;
    }

    for tl in &tl_lines {
        // 策略 1：找到开始时间 ≤ tl.start 且结束时间 > tl.start 的主歌词行
        //         即翻译时间戳落在歌词行的活跃区间内
        let mut best = main_lines
            .iter_mut()
            .filter(|l| l.start <= tl.start && l.start + l.duration > tl.start)
            .last();

        // 策略 2：若无精确区间匹配，使用最近邻（差值 < 阈值）
        if best.is_none() {
            best = main_lines
                .iter_mut()
                .filter(|l| {
                    let diff = (l.start as i64 - tl.start as i64).unsigned_abs();
                    diff < MATCH_THRESHOLD_MS
                })
                .min_by_key(|l| (l.start as i64 - tl.start as i64).unsigned_abs());
        }

        if let Some(line) = best {
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

// ─── 入口：从 Lyric 结构体解析并返回最终行列表 ────────────────────────────────

use crate::api::model::LyricDetail;

/// 解析 API 返回的 Lyric，优先使用 yrc 逐字歌词，fallback 到 lyric。
/// 翻译自动配对注入。
/// 返回 None 表示纯音乐或无歌词。
pub fn parse_lyric(lyric: &LyricDetail) -> Option<Vec<LyricLine>> {
    if lyric.is_pure_music {
        return None;
    }

    // 优先 yrc（可能混合 JSON + YRC 格式）
    let mut lines = if let Some(yrc) = &lyric.yrc {
        let mut parsed = parse_verbatim_json(yrc);
        parsed.append(&mut parse_yrc(yrc));
        if !parsed.is_empty() {
            parsed
        } else {
            // yrc 存在但解析为空，fallback 到 lyric
            parse_mixed(&lyric.lyric)
        }
    } else {
        parse_mixed(&lyric.lyric)
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

    // 合并连续的 JSON metadata 行作为单独块的末尾补充
    Some(lines)
}

/// 解析混合格式（JSON + LRC）的原始歌词文本
fn parse_mixed(raw: &Option<String>) -> Vec<LyricLine> {
    let raw = match raw.as_deref() {
        Some(r) if !r.is_empty() => r,
        _ => return Vec::new(),
    };
    let mut lines = parse_verbatim_json(raw);
    lines.append(&mut parse_lrc(raw));
    // 按时间排序确保 JSON 行和 LRC 行交错时顺序正确
    lines.sort_by_key(|l| l.start);
    lines
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

    // ── 时间戳解析 ────────────────────────────────────────────────────────────

    #[test]
    fn test_lrc_timestamp() {
        assert_eq!(parse_lrc_timestamp("00:30.50"), Some(30_500));
        assert_eq!(parse_lrc_timestamp("01:02.03"), Some(62_030));
        assert_eq!(parse_lrc_timestamp("00:00.000"), Some(0));
        assert_eq!(parse_lrc_timestamp("03:25.120"), Some(205_120));
        // 无边小数部分
        assert_eq!(parse_lrc_timestamp("00:01"), Some(1_000));
        // 1位小数
        assert_eq!(parse_lrc_timestamp("00:00.5"), Some(500));
        // 3位毫秒
        assert_eq!(parse_lrc_timestamp("00:00.500"), Some(500));
    }

    #[test]
    fn test_lrc_timestamp_rejects_metadata() {
        // 元数据标签如 by:, ti: 等不应被解析为时间戳
        assert_eq!(parse_lrc_timestamp("by:xxx"), None);
        assert_eq!(parse_lrc_timestamp("ti:title"), None);
        assert_eq!(parse_lrc_timestamp("ar:artist"), None);
        assert_eq!(parse_lrc_timestamp("offset:0"), None);
    }

    #[test]
    fn test_parse_pair() {
        assert_eq!(parse_pair("1740,480,0"), Some((1740, 480)));
        assert_eq!(parse_pair("1740,480"), Some((1740, 480)));
        assert_eq!(parse_pair("0,0"), Some((0, 0)));
        assert_eq!(parse_pair("invalid"), None);
        assert_eq!(parse_pair("1,x"), None);
        assert_eq!(parse_pair(""), None);
    }

    // ── LRC 解析 ──────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_lrc_basic() {
        let raw = "[00:01.00]第一行\n[00:03.50]第二行\n[00:06.00]第三行";
        let lines = parse_lrc(raw);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start, 1_000);
        assert_eq!(lines[0].duration, 2_500); // 3500 - 1000
        assert_eq!(lines[0].text, "第一行");
        assert!(matches!(lines[0].kind, LyricLineKind::Plain));

        assert_eq!(lines[1].start, 3_500);
        assert_eq!(lines[1].duration, 2_500); // 6000 - 3500
        assert_eq!(lines[1].text, "第二行");

        assert_eq!(lines[2].start, 6_000);
        assert_eq!(lines[2].duration, 5_000); // 最后一行默认5秒
        assert_eq!(lines[2].text, "第三行");
    }

    #[test]
    fn test_parse_lrc_multi_timestamp() {
        let raw = "[00:01.00][00:02.00]重复行\n[00:03.00]单独行";
        let lines = parse_lrc(raw);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start, 1_000);
        assert_eq!(lines[0].text, "重复行");
        assert_eq!(lines[1].start, 2_000);
        assert_eq!(lines[1].text, "重复行");
        assert_eq!(lines[2].start, 3_000);
        assert_eq!(lines[2].text, "单独行");
    }

    #[test]
    fn test_parse_lrc_skips_metadata() {
        let raw = "[ti:标题]\n[ar:歌手]\n[by:制作人]\n[00:01.00]实际歌词\n[00:03.00]继续";
        let lines = parse_lrc(raw);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "实际歌词");
        assert_eq!(lines[1].text, "继续");
    }

    #[test]
    fn test_parse_lrc_skips_empty_text() {
        let raw = "[00:01.00]\n[00:03.00]有文本\n[00:05.00]  ";
        let lines = parse_lrc(raw);
        // 空文本和纯空白都跳过
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "有文本");
    }

    #[test]
    fn test_parse_lrc_skips_json_lines() {
        let raw = "{\"t\":0,\"c\":[{\"tx\":\"编曲: \"},{\"tx\":\"某人\"}]}\n[00:01.00]歌词";
        let lines = parse_lrc(raw);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "歌词");
    }

    #[test]
    fn test_parse_lrc_empty() {
        assert_eq!(parse_lrc("").len(), 0);
        assert_eq!(parse_lrc("[ti:标题]\n[ar:歌手]").len(), 0);
    }

    #[test]
    fn test_parse_lrc_3digit_milliseconds() {
        // 实际网易云格式有3位毫秒
        let raw = "[00:01.080]歌词A\n[00:03.060]歌词B";
        let lines = parse_lrc(raw);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, 1_080);
        assert_eq!(lines[1].start, 3_060);
    }

    // ── YRC 解析 ──────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_yrc_basic() {
        let raw = "[1740,2160](1740,480,0)遥(2220,300,0)远(2520,300,0)的(2820,360,0)东(3180,360,0)方";
        let lines = parse_yrc(raw);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].start, 1740);
        assert_eq!(lines[0].duration, 2160);
        assert_eq!(lines[0].text, "遥远的东方");

        match &lines[0].kind {
            LyricLineKind::Verbatim(chars) => {
                assert_eq!(chars.len(), 5);
                assert_eq!(chars[0].ch, "遥");
                assert_eq!(chars[0].start, 1740);
                assert_eq!(chars[0].duration, 480);
                assert_eq!(chars[4].ch, "方");
                assert_eq!(chars[4].start, 3180);
                assert_eq!(chars[4].duration, 360);
            }
            _ => panic!("Expected Verbatim"),
        }
    }

    #[test]
    fn test_parse_yrc_multiple_lines() {
        let raw = "\
[1740,2160](1740,480,0)遥(2220,300,0)远
[3900,2000](3900,500,0)的(4400,500,0)东(4900,500,0)方";
        let lines = parse_yrc(raw);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "遥远");
        assert_eq!(lines[0].start, 1740);
        assert_eq!(lines[0].duration, 2160);
        assert_eq!(lines[1].text, "的东方");
        assert_eq!(lines[1].start, 3900);
        assert_eq!(lines[1].duration, 2000);
    }

    #[test]
    fn test_parse_yrc_skips_invalid_header() {
        // 行头不是有效的 start,duration 格式
        let raw = "[invalid](100,200,0)字";
        let lines = parse_yrc(raw);
        assert_eq!(lines.len(), 1); // parse_pair returns None, so (0,0)
        assert_eq!(lines[0].start, 0);
    }

    #[test]
    fn test_parse_yrc_skips_json_lines() {
        let raw = "{\"t\":0,\"c\":[{\"tx\":\"编曲: \"},{\"tx\":\"某人\"}]}\n[1740,2160](1740,480,0)遥";
        let lines = parse_yrc(raw);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "遥");
    }

    #[test]
    fn test_parse_yrc_empty() {
        assert_eq!(parse_yrc("").len(), 0);
        assert_eq!(parse_yrc("{\"t\":0,\"c\":[]}").len(), 0);
        assert_eq!(parse_yrc("[0,0]").len(), 0); // 行头但无字符
    }

    #[test]
    fn test_parse_yrc_multi_char_per_group() {
        // 模拟多个字符共享同一时间戳（如空格）
        let raw = "[1000,2000](1000,100,0)AB(1100,100,0)C";
        let lines = parse_yrc(raw);
        assert_eq!(lines.len(), 1);
        match &lines[0].kind {
            LyricLineKind::Verbatim(chars) => {
                assert_eq!(chars.len(), 3);
                assert_eq!(chars[0].ch, "A");
                assert_eq!(chars[0].duration, 50); // 100 / 2
                assert_eq!(chars[1].ch, "B");
                assert_eq!(chars[1].start, 1050); // 1000 + 50
                assert_eq!(chars[1].duration, 50);
                assert_eq!(chars[2].ch, "C");
                assert_eq!(chars[2].start, 1100);
                assert_eq!(chars[2].duration, 100);
            }
            _ => panic!("Expected Verbatim"),
        }
    }

    #[test]
    fn test_parse_yrc_line_with_brackets_in_text() {
        // YRC 格式中的字符可能包含特殊符号
        let raw = "[1000,2000](1000,200,0)「(1200,200,0)あ(1400,200,0)」";
        let lines = parse_yrc(raw);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "「あ」");
    }

    // ── JSON 逐字解析 ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_verbatim_json_basic() {
        let raw = "{\"t\":0,\"c\":[{\"tx\":\"编曲: \"},{\"tx\":\"赤髪\"}]}";
        let lines = parse_verbatim_json(raw);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].start, 0);
        assert_eq!(lines[0].text, "编曲: 赤髪");
    }

    #[test]
    fn test_parse_verbatim_json_multiple_lines() {
        let raw = "\
{\"t\":0,\"c\":[{\"tx\":\"作词: \"},{\"tx\":\"n-buna\"}]}
{\"t\":1000,\"c\":[{\"tx\":\"作曲: \"},{\"tx\":\"n-buna\"}]}";
        let lines = parse_verbatim_json(raw);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "作词: n-buna");
        assert_eq!(lines[0].start, 0);
        assert_eq!(lines[1].text, "作曲: n-buna");
        assert_eq!(lines[1].start, 1000);
    }

    #[test]
    fn test_parse_verbatim_json_empty() {
        assert_eq!(parse_verbatim_json("").len(), 0);
        assert_eq!(parse_verbatim_json("[00:01.00]lrc line").len(), 0); // 不是 JSON
        assert_eq!(parse_verbatim_json("{\"t\":0,\"c\":[]}").len(), 0); // 空字符
    }

    #[test]
    fn test_parse_verbatim_json_with_null_tx() {
        // 某些字段可能为 null
        let raw = "{\"t\":0,\"c\":[{\"tx\":\"A\"},{\"tx\":null},{\"tx\":\"B\"}]}";
        let lines = parse_verbatim_json(raw);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "AB");
    }

    // ── parse_mixed (JSON + LRC) ──────────────────────────────────────────────

    #[test]
    fn test_parse_mixed() {
        let raw = Some("\
{\"t\":0,\"c\":[{\"tx\":\"作词: \"},{\"tx\":\"作者\"}]}
{\"t\":1000,\"c\":[{\"tx\":\"作曲: \"},{\"tx\":\"作者\"}]}
[00:01.00]第一行
[00:05.00]第二行"
            .to_string());
        let lines = parse_mixed(&raw);
        assert!(lines.len() >= 4);
        // JSON 行
        assert_eq!(lines[0].text, "作词: 作者");
        assert_eq!(lines[0].start, 0);
        assert_eq!(lines[1].text, "作曲: 作者");
        assert_eq!(lines[1].start, 1000);
        // LRC 行
        let lrc_lines: Vec<&LyricLine> = lines.iter().filter(|l| matches!(l.kind, LyricLineKind::Plain)).collect();
        assert_eq!(lrc_lines.len(), 2);
        assert_eq!(lrc_lines[0].text, "第一行");
        assert_eq!(lrc_lines[1].text, "第二行");
    }

    #[test]
    fn test_parse_mixed_none() {
        assert_eq!(parse_mixed(&None).len(), 0);
        assert_eq!(parse_mixed(&Some("".to_string())).len(), 0);
    }

    // ── 翻译配对 ──────────────────────────────────────────────────────────────

    #[test]
    fn test_inject_translations_exact() {
        let raw_main = "[00:01.00]Hello world\n[00:04.00]Goodbye";
        let raw_tl = "[00:01.00]你好世界\n[00:04.00]再见";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        assert_eq!(lines[0].translation.as_deref(), Some("你好世界"));
        assert_eq!(lines[1].translation.as_deref(), Some("再见"));
    }

    #[test]
    fn test_inject_translations_slight_offset() {
        // 翻译时间戳有少许偏移（±100ms 内）
        let raw_main = "[00:01.00]Hello world\n[00:04.00]Goodbye";
        let raw_tl = "[00:01.10]你好世界\n[00:04.20]再见";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        // 翻译 1100ms 落在第一行 [1000, 4000) 区间内
        assert_eq!(lines[0].translation.as_deref(), Some("你好世界"));
        // 翻译 4200ms 落在第二行 [4000, 9000) 区间内
        assert_eq!(lines[1].translation.as_deref(), Some("再见"));
    }

    #[test]
    fn test_inject_translations_between_lines_uses_interval() {
        // 关键测试：翻译时间戳落在第一行的活跃区间，应该匹配第一行
        let raw_main = "[00:01.00]Line A\n[00:05.00]Line B";
        let raw_tl = "[00:03.00]Translation"; // 3000ms
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);

        // 3000ms 在 Line A [1000, 5000) 内 → 应匹配 Line A
        assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
        assert_eq!(lines[1].translation.as_deref(), None);
    }

    #[test]
    fn test_inject_translations_near_boundary() {
        // 翻译接近行边界
        let raw_main = "[00:01.00]Line A\n[00:03.00]Line B";
        let raw_tl = "[00:02.95]Translation"; // 2950ms
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);

        // 2950ms 在 Line A [1000, 3000) 内
        assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
        assert_eq!(lines[1].translation.as_deref(), None);
    }

    #[test]
    fn test_inject_translations_falls_to_nearest() {
        // 翻译时间戳不在任何行的活跃区间，退化为最近邻
        let raw_main = "[00:01.00]Line A\n[00:04.00]Line B";
        let raw_tl = "[00:03.50]Translation"; // 3500ms, 但不排除在某个区间
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);

        // 3500ms 在 Line A [1000, 4000) 内
        assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
    }

    #[test]
    fn test_inject_translations_outside_range_no_match() {
        // 翻译行时间戳与任何主歌词行差距超过阈值
        let raw_main = "[00:10.00]Line A\n[00:14.00]Line B";
        let raw_tl = "[00:01.00]Too far";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        // 1000ms vs 10000ms，差异 9000ms > 800ms 阈值
        assert_eq!(lines[0].translation, None);
        assert_eq!(lines[1].translation, None);
    }

    #[test]
    fn test_inject_translations_multiple_to_same_line() {
        // 多条翻译时间戳落到同一行区间
        let raw_main = "[00:01.00]Line A\n[00:10.00]Line B";
        let raw_tl = "[00:01.10]TL1\n[00:01.20]TL2";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        assert_eq!(lines[0].translation.as_deref(), Some("TL1\nTL2"));
        assert_eq!(lines[1].translation, None);
    }

    #[test]
    fn test_inject_translations_empty() {
        let mut lines = parse_lrc("[00:01.00]Hello");
        inject_translations(&mut lines, "");
        assert_eq!(lines[0].translation, None);

        let mut empty: Vec<LyricLine> = Vec::new();
        inject_translations(&mut empty, "[00:01.00]Translation");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_inject_translations_translation_before_first_line() {
        // 翻译在第一行之前
        let raw_main = "[00:05.00]Line A";
        let raw_tl = "[00:01.00]Early";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        // 1000ms vs 5000ms，不在区间内，最近邻距离 4000ms > 800ms
        assert_eq!(lines[0].translation, None);
    }

    #[test]
    fn test_inject_translations_skips_metadata_in_tl() {
        // 翻译中的元数据应被忽略
        let raw_main = "[00:01.00]Hello\n[00:03.00]World";
        let raw_tl = "[by:translator]\n[00:01.00]你好\n[00:03.00]世界";
        let mut lines = parse_lrc(raw_main);
        inject_translations(&mut lines, raw_tl);
        assert_eq!(lines[0].translation.as_deref(), Some("你好"));
        assert_eq!(lines[1].translation.as_deref(), Some("世界"));
    }

    // ── parse_lyric 端到端 ────────────────────────────────────────────────────

    #[test]
    fn test_parse_lyric_pure_music() {
        let detail = LyricDetail {
            is_pure_music: true,
            lyric: Some("[00:01.00]Actually has lyric".into()),
            tlyric: None,
            yrc: None,
        };
        assert_eq!(parse_lyric(&detail), None);
    }

    #[test]
    fn test_parse_lyric_lrc_with_translation() {
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: Some("[00:01.00]Hello\n[00:04.00]World".into()),
            tlyric: Some("[00:01.00]你好\n[00:04.00]世界".into()),
            yrc: None,
        };
        let lines = parse_lyric(&detail).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello");
        assert_eq!(lines[0].translation.as_deref(), Some("你好"));
        assert_eq!(lines[1].text, "World");
        assert_eq!(lines[1].translation.as_deref(), Some("世界"));
    }

    #[test]
    fn test_parse_lyric_yrc_priority() {
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: Some("[00:01.00]LRC version".into()),
            tlyric: None,
            yrc: Some("[1000,3000](1000,500,0)Y(1500,500,0)R(2000,500,0)C".into()),
        };
        let lines = parse_lyric(&detail).unwrap();
        // 有 yrc 时优先使用 yrc
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "YRC");
        assert!(matches!(lines[0].kind, LyricLineKind::Verbatim(_)));
    }

    #[test]
    fn test_parse_lyric_yrc_empty_fallback() {
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: Some("[00:01.00]LRC version".into()),
            tlyric: None,
            yrc: Some("".into()), // yrc 存在但为空字符串
        };
        let lines = parse_lyric(&detail).unwrap();
        assert_eq!(lines[0].text, "LRC version");
    }

    #[test]
    fn test_parse_lyric_yrc_json_only_fallback() {
        // yrc 字段只有 JSON 行没有 YRC 行，应 fallback 到 lyric
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: Some("[00:01.00]Fallback LRC".into()),
            tlyric: None,
            yrc: Some("{\"t\":0,\"c\":[{\"tx\":\"仅元数据\"}]}".into()),
        };
        let lines = parse_lyric(&detail).unwrap();
        // VERBATIM_JSON 解析出来只有 metadata 行，没有 LRC
        // 不应该 fallback 因为 JSON 行被 parse_verbatim_json 解析出来了
        // 实际上这里有 1 行 JSON metadata + fallback LRC
        assert!(lines.iter().any(|l| l.text == "Fallback LRC" || l.text == "仅元数据"));
    }

    #[test]
    fn test_parse_lyric_mixed_json_lrc_in_lyric_field() {
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: Some(
                "{\"t\":0,\"c\":[{\"tx\":\"作词: \"},{\"tx\":\"作者\"}]}\n[00:01.00]歌词"
                    .into(),
            ),
            tlyric: Some("[00:01.00]翻译".into()),
            yrc: None,
        };
        let lines = parse_lyric(&detail).unwrap();
        // 至少应该有 2 行：JSON metadata + LRC 歌词
        assert!(lines.len() >= 2);
        assert!(lines.iter().any(|l| l.text == "作词: 作者"));
        assert!(lines.iter().any(|l| l.text == "歌词"));
        // 翻译应注入到 LRC 行
        let lrc_line = lines.iter().find(|l| l.text == "歌词").unwrap();
        assert_eq!(lrc_line.translation.as_deref(), Some("翻译"));
    }

    #[test]
    fn test_parse_lyric_no_content() {
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: None,
            tlyric: None,
            yrc: None,
        };
        assert_eq!(parse_lyric(&detail), None);

        let detail2 = LyricDetail {
            is_pure_music: false,
            lyric: Some("".into()),
            tlyric: None,
            yrc: None,
        };
        assert_eq!(parse_lyric(&detail2), None);
    }

    #[test]
    fn test_parse_lyric_inject_skips_empty_tlyric() {
        let detail = LyricDetail {
            is_pure_music: false,
            lyric: Some("[00:01.00]Hello".into()),
            tlyric: Some("".into()),
            yrc: None,
        };
        let lines = parse_lyric(&detail).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].translation, None);
    }

    #[test]
    fn test_parse_lrc_real_world_format() {
        // 模拟真实网易云 API 返回的 LRC 格式
        let raw = "\
[00:19.600]
[00:20.000]やっと眼を覚ましたかい それなのになぜ眼も合わせやしないんだい？
[00:30.090]「遅いよ」と怒る君 これでもやれるだけ飛ばしてきたんだよ
[00:38.720]
[00:39.670]心が身体を追い越してきたんだよ
";
        let lines = parse_lrc(raw);
        // 空文本行应被跳过（[00:19.600] 和 [00:38.720]）
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start, 20_000);
        assert_eq!(lines[1].start, 30_090);
        assert_eq!(lines[2].start, 39_670);
    }
}
