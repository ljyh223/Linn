use super::*;

fn mk_detail(
    lyric: Option<&str>,
    tlyric: Option<&str>,
    yrc: Option<&str>,
    ytlrc: Option<&str>,
    pure: bool,
) -> LyricDetail {
    LyricDetail {
        lyric: lyric.map(|s| s.to_string()),
        tlyric: tlyric.map(|s| s.to_string()),
        yrc: yrc.map(|s| s.to_string()),
        ytlrc: ytlrc.map(|s| s.to_string()),
        is_pure_music: pure,
    }
}

// ── 时间戳解析 ────────────────────────────────────────────────────────────

#[test]
fn test_lrc_timestamp() {
    assert_eq!(parse_lrc_timestamp("00:30.50"), Some(30_500));
    assert_eq!(parse_lrc_timestamp("01:02.03"), Some(62_030));
    assert_eq!(parse_lrc_timestamp("00:00.000"), Some(0));
    assert_eq!(parse_lrc_timestamp("03:25.120"), Some(205_120));
    assert_eq!(parse_lrc_timestamp("00:01"), Some(1_000));
    assert_eq!(parse_lrc_timestamp("00:00.5"), Some(500));
    assert_eq!(parse_lrc_timestamp("00:00.500"), Some(500));
    assert_eq!(parse_lrc_timestamp("100:30.00"), Some(6_030_000));
    assert_eq!(parse_lrc_timestamp("00:30:50"), Some(30_500));
}

#[test]
fn test_lrc_timestamp_rejects_metadata() {
    assert_eq!(parse_lrc_timestamp("by:xxx"), None);
    assert_eq!(parse_lrc_timestamp("ti:title"), None);
    assert_eq!(parse_lrc_timestamp("ar:artist"), None);
    assert_eq!(parse_lrc_timestamp("offset:0"), None);
}

// ── LRC 解析 ──────────────────────────────────────────────────────────────

#[test]
fn test_parse_lrc_basic() {
    let raw = "[00:01.00]第一行\n[00:03.50]第二行\n[00:06.00]第三行";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].start, 1_000);
    assert_eq!(lines[0].duration, 2_500);
    assert_eq!(lines[0].text, "第一行");
    assert!(matches!(lines[0].kind, LyricLineKind::Plain));

    assert_eq!(lines[1].start, 3_500);
    assert_eq!(lines[1].duration, 2_500);
    assert_eq!(lines[1].text, "第二行");

    assert_eq!(lines[2].start, 6_000);
    assert_eq!(lines[2].duration, 5_000);
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
    let raw = "[00:01.080]歌词A\n[00:03.060]歌词B";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].start, 1_080);
    assert_eq!(lines[1].start, 3_060);
}

#[test]
fn test_parse_lrc_real_world_format() {
    let raw = "\
[00:19.600]
[00:20.000]やっと眼を覚ましたかい それなのになぜ眼も合わせやしないんだい？
[00:30.090]「遅いよ」と怒る君 これでもやれるだけ飛ばしてきたんだよ
[00:38.720]
[00:39.670]心が身体を追い越してきたんだよ
";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].start, 20_000);
    assert_eq!(lines[1].start, 30_090);
    assert_eq!(lines[2].start, 39_670);
}

#[test]
fn test_parse_lrc_skips_copyright() {
    let raw = "[00:01.00]著作权归原作者所有\n[00:03.00]正常歌词\n[00:05.00]QQ音乐提供";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "正常歌词");
}

#[test]
fn test_parse_lrc_skips_double_slash() {
    let raw = "[00:01.00]// 这是注释\n[00:03.00]正常歌词";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "正常歌词");
}

#[test]
fn test_parse_lrc_3digit_minutes() {
    let raw = "[100:30.00]非常长的音乐\n[101:00.00]结束";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].start, 100 * 60_000 + 30_000);
    assert_eq!(lines[1].start, 101 * 60_000 + 0);
}

#[test]
fn test_parse_lrc_colon_separator() {
    let raw = "[00:01:50]用冒号分隔\n[00:03:25]第二行";
    let lines = parse_lrc(raw);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].start, 1_500);
    assert_eq!(lines[1].start, 3_250);
}

// ── YRC 解析 ──────────────────────────────────────────────────────────────

#[test]
fn test_parse_yrc_basic() {
    let raw = "[1740,2160](1740,480,1)遥(2220,300,2)远(2520,300,3)的(2820,360,4)东(3180,360,5)方";
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
    assert_eq!(lines[1].text, "的东方");
    assert_eq!(lines[1].start, 3900);
}

#[test]
fn test_parse_yrc_skips_invalid_header() {
    let raw = "[invalid](100,200,0)字";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 0);
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
    assert_eq!(parse_yrc("[0,0]").len(), 0);
}

#[test]
fn test_parse_yrc_multi_char_per_group() {
    let raw = "[1000,2000](1000,100,0)AB(1100,100,0)C";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    match &lines[0].kind {
        LyricLineKind::Verbatim(chars) => {
            assert_eq!(chars.len(), 3);
            assert_eq!(chars[0].ch, "A");
            assert_eq!(chars[0].duration, 50);
            assert_eq!(chars[1].ch, "B");
            assert_eq!(chars[1].start, 1050);
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
    let raw = "[1000,2000](1000,200,0)「(1200,200,0)あ(1400,200,0)」";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "「あ」");
}

#[test]
fn test_parse_qrc_two_field_character_timing() {
    let raw = "[1000,1000](1000,300)你(1300,300)好";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "你好");
    match &lines[0].kind {
        LyricLineKind::Verbatim(chars) => {
            assert_eq!(chars.len(), 2);
            assert_eq!(chars[0].start, 1000);
            assert_eq!(chars[0].duration, 300);
            assert_eq!(chars[1].start, 1300);
            assert_eq!(chars[1].duration, 300);
        }
        _ => panic!("Expected Verbatim"),
    }
}

#[test]
fn test_parse_qrc_character_timing_after_text() {
    let raw = "[1000,1000]你(1000,300)好(1300,300)";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "你好");
    match &lines[0].kind {
        LyricLineKind::Verbatim(chars) => {
            assert_eq!(chars.len(), 2);
            assert_eq!(chars[0].ch, "你");
            assert_eq!(chars[0].start, 1000);
            assert_eq!(chars[1].ch, "好");
            assert_eq!(chars[1].start, 1300);
        }
        _ => panic!("Expected Verbatim"),
    }
}

#[test]
fn test_parse_qrc_relative_character_timing() {
    let raw = "[1000,1000](0,300)你(300,300)好";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    match &lines[0].kind {
        LyricLineKind::Verbatim(chars) => {
            assert_eq!(chars[0].start, 1000);
            assert_eq!(chars[1].start, 1300);
        }
        _ => panic!("Expected Verbatim"),
    }
}

#[test]
fn test_parse_qrc_preserves_english_spaces() {
    let raw = "[1000,1000](1000,100)H(1100,100)e(1200,100)l(1300,100)l(1400,100)o(1500,100) (1600,100)w(1700,100)o(1800,100)r(1900,100)l(2000,100)d";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "Hello world");
    match &lines[0].kind {
        LyricLineKind::Verbatim(chars) => {
            assert_eq!(
                chars.iter().map(|c| c.ch.as_str()).collect::<String>(),
                "Hello world"
            );
            assert_eq!(chars[5].ch, " ");
        }
        _ => panic!("Expected Verbatim"),
    }
}

#[test]
fn test_parse_qrc_after_text_preserves_english_spaces() {
    // QQ QRC commonly puts timing after a word. In that form the separator
    // lives between two timing markers and must not be trimmed away.
    let raw = "[1000,1800]Hello(1000,600) world(1600,600) again(2200,600)";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "Hello world again");
    match &lines[0].kind {
        LyricLineKind::Verbatim(chars) => {
            assert_eq!(
                chars.iter().map(|c| c.ch.as_str()).collect::<String>(),
                "Hello world again"
            );
            assert_eq!(chars[5].ch, " ");
            assert_eq!(chars[11].ch, " ");
        }
        _ => panic!("Expected Verbatim"),
    }
}

#[test]
fn test_parse_yrc_numeric_xml_line_breaks() {
    let raw = "[1000,2000](1000,500,0)第&#10;[3500,1000](3500,500,0)二";
    let lines = parse_yrc(raw);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, "第");
    assert_eq!(lines[1].text, "二");
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
    assert_eq!(lines[1].text, "作曲: n-buna");
}

#[test]
fn test_parse_verbatim_json_empty() {
    assert_eq!(parse_verbatim_json("").len(), 0);
    assert_eq!(parse_verbatim_json("[00:01.00]lrc line").len(), 0);
    assert_eq!(parse_verbatim_json("{\"t\":0,\"c\":[]}").len(), 0);
}

#[test]
fn test_parse_verbatim_json_with_null_tx() {
    let raw = "{\"t\":0,\"c\":[{\"tx\":\"A\"},{\"tx\":null},{\"tx\":\"B\"}]}";
    let lines = parse_verbatim_json(raw);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "AB");
}

// ── parse_mixed (JSON + LRC) ──────────────────────────────────────────────

#[test]
fn test_parse_mixed() {
    let raw = Some(
        "\
{\"t\":0,\"c\":[{\"tx\":\"作词: \"},{\"tx\":\"作者\"}]}
{\"t\":1000,\"c\":[{\"tx\":\"作曲: \"},{\"tx\":\"作者\"}]}
[00:01.00]第一行
[00:05.00]第二行"
            .to_string(),
    );
    let lines = parse_mixed(&raw);
    assert!(lines.len() >= 4);
    assert_eq!(lines[0].text, "作词: 作者");
    assert_eq!(lines[0].start, 0);
    assert_eq!(lines[1].text, "作曲: 作者");
    assert_eq!(lines[1].start, 1000);
    let lrc_lines: Vec<&LyricLine> = lines
        .iter()
        .filter(|l| matches!(l.kind, LyricLineKind::Plain))
        .collect();
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
    let mut lines = parse_lrc("[00:01.00]Hello world\n[00:04.00]Goodbye");
    inject_translations(&mut lines, "[00:01.00]你好世界\n[00:04.00]再见");
    assert_eq!(lines[0].translation.as_deref(), Some("你好世界"));
    assert_eq!(lines[1].translation.as_deref(), Some("再见"));
}

#[test]
fn test_inject_translations_slight_offset() {
    let mut lines = parse_lrc("[00:01.00]Hello world\n[00:04.00]Goodbye");
    inject_translations(&mut lines, "[00:01.10]你好世界\n[00:04.20]再见");
    assert_eq!(lines[0].translation.as_deref(), Some("你好世界"));
    assert_eq!(lines[1].translation.as_deref(), Some("再见"));
}

#[test]
fn test_inject_translations_between_lines_uses_interval() {
    let mut lines = parse_lrc("[00:01.00]Line A\n[00:05.00]Line B");
    inject_translations(&mut lines, "[00:03.00]Translation");
    assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
    assert_eq!(lines[1].translation.as_deref(), None);
}

#[test]
fn test_inject_translations_near_boundary() {
    let mut lines = parse_lrc("[00:01.00]Line A\n[00:03.00]Line B");
    inject_translations(&mut lines, "[00:02.95]Translation");
    assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
    assert_eq!(lines[1].translation.as_deref(), None);
}

#[test]
fn test_inject_translations_falls_to_nearest() {
    let mut lines = parse_lrc("[00:01.00]Line A\n[00:04.00]Line B");
    inject_translations(&mut lines, "[00:03.50]Translation");
    assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
}

#[test]
fn test_inject_translations_outside_range_no_match() {
    let mut lines = parse_lrc("[00:10.00]Line A\n[00:14.00]Line B");
    inject_translations(&mut lines, "[00:01.00]Too far");
    assert_eq!(lines[0].translation, None);
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
    let mut lines = parse_lrc("[00:05.00]Line A");
    inject_translations(&mut lines, "[00:01.00]Early");
    assert_eq!(lines[0].translation, None);
}

#[test]
fn test_inject_translations_skips_metadata_in_tl() {
    let mut lines = parse_lrc("[00:01.00]Hello\n[00:03.00]World");
    inject_translations(
        &mut lines,
        "[by:translator]\n[00:01.00]你好\n[00:03.00]世界",
    );
    assert_eq!(lines[0].translation.as_deref(), Some("你好"));
    assert_eq!(lines[1].translation.as_deref(), Some("世界"));
}

#[test]
fn test_inject_translations_longer_text_tie_break() {
    let mut lines = parse_lrc("[00:00.50]短\n[00:00.50]较长的歌词行文字");
    inject_translations(&mut lines, "[00:00.10]翻译");
    assert_eq!(lines[1].translation.as_deref(), Some("翻译"));
    assert_eq!(lines[0].translation, None);
}

#[test]
fn test_inject_translations_one_to_one_constraint() {
    let mut lines = parse_lrc("[00:01.00]Line A\n[00:04.00]Line B");
    inject_translations(&mut lines, "[00:01.10]Trans A\n[00:01.20]Trans A2");
    // Trans A 先匹配 Line A，Trans A2 不能再匹配 Line A（1:1约束）
    // Trans A2 会尝试匹配 Line B: |1200-4000|=2800 < 3000，匹配 Line B
    assert_eq!(lines[0].translation.as_deref(), Some("Trans A"));
    // Trans A2 在 Line A 的区间 [1000,4000) 内，但 Line A 已被占用
    // 退化到最近邻: 距 Line A=80 但 used, 距 Line B=2800 < 3000
    assert_eq!(lines[1].translation.as_deref(), Some("Trans A2"));
}

#[test]
fn test_inject_translations_large_threshold_enabled() {
    // 3000ms 阈值允许更大的偏移
    let mut lines = parse_lrc("[00:01.00]Line A\n[00:05.00]Line B");
    inject_translations(&mut lines, "[00:03.50]Translation");
    // 3500ms: 距 Line A 2500 < 3000, 不在 Line B [5000,10000) 区间
    // 所以匹配 Line A
    assert_eq!(lines[0].translation.as_deref(), Some("Translation"));
}

#[test]
fn test_inject_translations_skips_empty_tl_text() {
    let mut lines = parse_lrc("[00:01.00]Hello\n[00:04.00]World");
    inject_translations(&mut lines, "[00:01.00]\n[00:04.00]世界");
    assert_eq!(lines[0].translation, None);
    assert_eq!(lines[1].translation.as_deref(), Some("世界"));
}

#[test]
fn test_inject_translations_copyright_in_tl_ignored() {
    let mut lines = parse_lrc("[00:01.00]Hello\n[00:04.00]World");
    inject_translations(
        &mut lines,
        "[00:01.00]你好\n[00:02.00]著作权归QQ音乐所有\n[00:04.00]世界",
    );
    assert_eq!(lines[0].translation.as_deref(), Some("你好"));
    assert_eq!(lines[1].translation.as_deref(), Some("世界"));
}

// ── 真实歌曲场景 ──────────────────────────────────────────────────────────
//
// 以下测试全部使用网易云 API 返回的真实歌词数据。
// 数据来源：通过 ncm_api_rs 直接调用 get_lryic 获取 51 首歌曲的完整歌词。
//
// 覆盖模式：
//   LRC + tlyric: JP/CN 歌词 + 中文翻译（绝大多数含 [by:xxx] 翻译者标注）
//   YRC + ytlrc: YRC 逐字歌词 + LRC 翻译
//   空翻译: tlyric/ytlrc 仅含时间戳无文本
//   边界: 版权信息、元数据过滤、特殊字符

mod real_songs {
    use super::*;

    fn parse_ok(detail: &LyricDetail) -> Vec<LyricLine> {
        parse_lyric(detail).expect("parse_lyric returned None")
    }

    // ─── パレード (1357628744) - JP LRC + CN tlyric ([by:十二面体]) ─────

    #[test]
    fn parade_jp_lrc_cn_tlyric() {
        let jp = [
            "[00:17.18]身体の奥　喉の真下",
            "[00:25.94]心があるとするなら君はそこなんだろうから",
            "[00:52.17]ずっと前からわかっていたけど",
            "[01:00.88]歳取れば君の顔も忘れてしまうからさ",
            "[01:09.48]身体の奥　喉の中で　言葉が出来る瞬間を僕は知りたいから",
            "[01:27.11]このまま夜が明けたら",
            "[01:34.51]乾かないように想い出を",
            "[01:38.97]失くさないようにこの歌を",
            "[01:43.33]忘れないで　もうちょっとだけでいい",
            "[01:52.18]一人ぼっちのパレードを",
            "[02:12.90]ずっと前から思ってたけど",
            "[02:21.47]君の指先の中にはたぶん神様が住んでいる",
            "[02:30.67]今日、昨日よりずっと前から、ずっとその昔の昔から。",
            "[02:46.86]わかるんだ",
        ]
        .join("\n");
        let cn = [
            "[by:十二面体]",
            "[00:17.18]身体深处 喉咙正下",
            "[00:25.94]如果那里存在心脏的话 你一定在其中吧",
            "[00:52.17]虽然从很久以前就明白了",
            "[01:00.88]随着岁月流逝 连你的容颜也会被我忘记",
            "[01:09.48]但我还是想弄明白 在身体深处、喉咙之中 孕育出言语的那个瞬间啊",
            "[01:27.11]若就这样迎来黎明",
            "[01:34.51]为了不让这份回忆淡去",
            "[01:38.97]为了不让这首歌荡然无遗",
            "[01:43.33]请你不要忘记 再铭记一小会儿就可以",
            "[01:52.18]请不要忘记我独身一人的游行",
            "[02:12.90]虽然从很久很久以前就开始这么想了",
            "[02:21.47]你的指尖上 大概栖居着神明大人吧",
            "[02:30.67]但其实早在比今天，比昨天，还要久远的过去的过去",
            "[02:46.86]就已心知肚明了",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 14, "expected 14 lines");

        // 验证翻译匹配 (逐行检查)
        let expected: &[(&str, &str)] = &[
            ("身体の奥　喉の真下", "身体深处 喉咙正下"),
            (
                "心があるとするなら君はそこなんだろうから",
                "如果那里存在心脏的话 你一定在其中吧",
            ),
            ("ずっと前からわかっていたけど", "虽然从很久以前就明白了"),
            (
                "歳取れば君の顔も忘れてしまうからさ",
                "随着岁月流逝 连你的容颜也会被我忘记",
            ),
            (
                "身体の奥　喉の中で　言葉が出来る瞬間を僕は知りたいから",
                "但我还是想弄明白 在身体深处、喉咙之中 孕育出言语的那个瞬间啊",
            ),
            ("このまま夜が明けたら", "若就这样迎来黎明"),
            ("乾かないように想い出を", "为了不让这份回忆淡去"),
            ("失くさないようにこの歌を", "为了不让这首歌荡然无遗"),
            (
                "忘れないで　もうちょっとだけでいい",
                "请你不要忘记 再铭记一小会儿就可以",
            ),
            ("一人ぼっちのパレードを", "请不要忘记我独身一人的游行"),
            (
                "ずっと前から思ってたけど",
                "虽然从很久很久以前就开始这么想了",
            ),
            (
                "君の指先の中にはたぶん神様が住んでいる",
                "你的指尖上 大概栖居着神明大人吧",
            ),
            (
                "今日、昨日よりずっと前から、ずっとその昔の昔から。",
                "但其实早在比今天，比昨天，还要久远的过去的过去",
            ),
            ("わかるんだ", "就已心知肚明了"),
        ];
        for (i, &(jp_text, cn_text)) in expected.iter().enumerate() {
            assert_eq!(lines[i].text, jp_text, "line {} text mismatch", i);
            assert_eq!(
                lines[i].translation.as_deref(),
                Some(cn_text),
                "line {} translation mismatch",
                i
            );
        }
        // [by:十二面体] 在 tlyric 中应已被 parse_lrc 跳过
    }

    // ─── 百回目のキス (28018262) - JP LRC + CN tlyric ([by:游陈十代]) ──

    #[test]
    fn hyakkaime_jp_lrc_cn_tlyric_with_by() {
        let jp = [
            "[00:13.920]朝が来れば二人はもう",
            "[00:21.130]離れ離れ だから強く",
            "[00:28.490]握ったその手で抱いて",
            "[00:35.740]君の鼓動感じてたい",
            "[00:42.860]夜が終わるまえに",
            "[01:03.450]逃げ出して もっと もっと強く",
            "[01:12.080]してよ 全部壊してしまうくらい",
            "[01:19.420]さよならは言わないで",
            "[01:23.460]百回目のキスをして",
        ]
        .join("\n");
        let cn = [
            "[by:游陈十代]",
            "[00:13.920]待到清晨来临 二人就会",
            "[00:21.130]分别 分别了 因此再用力",
            "[00:28.490]用交握的手紧紧相拥吧",
            "[00:35.740]想要感觉你心脏的跳动",
            "[00:42.860]直到黑夜终结之前",
            "[01:03.450]逃走吧 更加 更加强烈些",
            "[01:12.080]强烈到将这一切全都破坏掉",
            "[01:19.420]别说那句再见",
            "[01:23.460]千百次地亲吻吧",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 9);
        let pairs: &[(&str, &str)] = &[
            ("朝が来れば二人はもう", "待到清晨来临 二人就会"),
            ("離れ離れ だから強く", "分别 分别了 因此再用力"),
            ("握ったその手で抱いて", "用交握的手紧紧相拥吧"),
            ("君の鼓動感じてたい", "想要感觉你心脏的跳动"),
            ("夜が終わるまえに", "直到黑夜终结之前"),
            ("逃げ出して もっと もっと強く", "逃走吧 更加 更加强烈些"),
            ("してよ 全部壊してしまうくらい", "强烈到将这一切全都破坏掉"),
            ("さよならは言わないで", "别说那句再见"),
            ("百回目のキスをして", "千百次地亲吻吧"),
        ];
        for (i, &(jp_text, cn_text)) in pairs.iter().enumerate() {
            assert_eq!(lines[i].text, jp_text);
            assert_eq!(lines[i].translation.as_deref(), Some(cn_text));
        }
    }

    // ─── 夏空 (31830616) - JP LRC + CN tlyric ([by:挖巨大]) ────────────

    #[test]
    fn natsuzora_jp_lrc_cn_tlyric() {
        let jp = [
            "[00:32.24]夜焦りの暗がり 去りし夏の宵(よい)",
            "[00:40.23]二人眺めてるの",
            "[00:43.09]闇空に放たれるほど儚く",
            "[00:48.93]降らせ今この静(せい)に 灯りはまだ遠く",
            "[00:56.52]揺らぎ揺らがれるほど",
            "[00:59.38]ただ浮かべば消える空を見上げた",
            "[01:20.16]あなたに問いかける",
            "[01:24.70]寂しさと別れは切り離されぬのか",
            "[01:28.87]響(な)る空花火の散るを見遣って",
            "[01:37.08]夏が僕らを染め上げる光になって",
        ]
        .join("\n");
        let cn = [
            "[by:挖巨大]",
            "[00:32.24]焦躁夜裹的暗处中夏日浅夜经已过去",
            "[00:40.23]俩人一同眺望着",
            "[00:43.09]仿似要绽放于夜空中般的虚幻",
            "[00:48.93]如今却散落的这份静寂灯火仍在远方",
            "[00:56.52]摇摇曳曳",
            "[00:59.38]我们仰望着它浮现空中而消失",
            "[01:20.16]我对你问道",
            "[01:24.70]寂寞与离别该不会是无法分离的吧",
            "[01:28.87]眺望着空中鸣响绽放的烟火",
            "[01:37.08]夏日化作将我们染上色彩的光芒",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 10);
        let checks: &[(usize, &str, &str)] = &[
            (
                0,
                "夜焦りの暗がり 去りし夏の宵(よい)",
                "焦躁夜裹的暗处中夏日浅夜经已过去",
            ),
            (1, "二人眺めてるの", "俩人一同眺望着"),
            (2, "闇空に放たれるほど儚く", "仿似要绽放于夜空中般的虚幻"),
            (
                3,
                "降らせ今この静(せい)に 灯りはまだ遠く",
                "如今却散落的这份静寂灯火仍在远方",
            ),
            (
                9,
                "夏が僕らを染め上げる光になって",
                "夏日化作将我们染上色彩的光芒",
            ),
        ];
        for &(i, jp_text, cn_text) in checks {
            assert_eq!(lines[i].text, jp_text);
            assert_eq!(lines[i].translation.as_deref(), Some(cn_text));
        }
        // 验证 [by:挖巨大] 被过滤 (不应该出现在任何一行)
        assert!(lines.iter().all(|l| !l.text.contains("by:")));
    }

    // ─── 空想 (2015622697) - JP LRC + CN tlyric ──────────────────────

    #[test]
    fn kuusou_jp_lrc_cn_tlyric() {
        let jp = [
            "[00:24.77]つじつま合わせるように生きていくなんてさ",
            "[00:32.38]まるで僕らしくないよね",
            "[00:40.05]いつか死んでしまうなら",
            "[00:44.04]まだ見えない遠くへ",
            "[00:48.18]「今だ」ってこの一歩を",
            "[00:51.40]踏み出すしかないな",
        ]
        .join("\n");
        let cn = [
            "[by:Han_Henceforth]",
            "[00:24.77]为了合乎情理活下去之类的",
            "[00:32.38]可是一点都不像我",
            "[00:40.05]若终有一天会死的话",
            "[00:44.04]现在向着前方的未知之处",
            "[00:48.18]“就是现在”踏出那一步",
            "[00:51.40]是唯一的选择了",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 6);
        for i in 0..6 {
            assert!(
                lines[i].translation.is_some(),
                "line {} should have translation",
                i
            );
        }
        assert_eq!(
            lines[0].translation.as_deref(),
            Some("为了合乎情理活下去之类的")
        );
        assert_eq!(lines[5].translation.as_deref(), Some("是唯一的选择了"));
    }

    // ─── U (31830614) - LRC + CN tlyric ([by:不動明皇风冥雷光]) ─────

    #[test]
    fn u_song_lrc_cn_tlyric() {
        let jp = [
            "[00:18.61]夏の匂いに　気付くまでの",
            "[00:24.27]日々の熱が　溶け焦げるまで",
            "[00:29.96]まだちょっとこのままで",
            "[00:36.59]覚めない夢を見させて",
            "[00:43.30]言えないことは　言えないけれど",
            "[00:48.85]それだけでいいと思ってる",
            "[00:54.65]なんて何回も",
            "[00:59.64]思い直した今日だった",
        ]
        .join("\n");
        let cn = [
            "[by:不动明皇风冥雷光]",
            "[00:18.61]在注意到那夏日气息之前",
            "[00:24.27]在蓬勃暑气快要焦熔化去之前",
            "[00:29.96]再稍稍这样一会就好",
            "[00:36.59]让我做一场不醒的美梦吧",
            "[00:43.30]虽然说不出口的事情",
            "[00:48.85]依然还是说不出口呢",
            "[00:54.65]就那样我也觉得",
            "[00:59.64]算是差强人意的今天",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 8);
        // 逐行验证翻译全匹配
        assert_eq!(
            lines[0].translation.as_deref(),
            Some("在注意到那夏日气息之前")
        );
        assert_eq!(
            lines[1].translation.as_deref(),
            Some("在蓬勃暑气快要焦熔化去之前")
        );
        assert_eq!(lines[2].translation.as_deref(), Some("再稍稍这样一会就好"));
        assert_eq!(
            lines[3].translation.as_deref(),
            Some("让我做一场不醒的美梦吧")
        );
        assert_eq!(lines[7].translation.as_deref(), Some("算是差强人意的今天"));
    }

    // ─── 快晴/起风了 (557583473) - JP LRC + CN tlyric ──────────────

    #[test]
    fn kaisei_jp_lrc_cn_tlyric() {
        let jp = [
            "[00:55.700]それは時の果てる 劇場世界のプロローグ",
            "[01:01.250]アライ A lie? 君は誰？ どことなく物憂げに",
            "[01:07.100]裸足のままで 張りつく夜に遊ぶように",
            "[01:11.960]彷徨う僕は何故か 君を探しているのだ",
            "[01:17.590]だぁ だぁ",
        ]
        .join("\n");
        let cn = [
            "[00:55.700]那是时光尽头的 剧场世界的序章",
            "[01:01.250]Ally A lie? 你是谁？总觉得有些没精打采",
            "[01:07.100]赤着脚 就像是在这拼凑出的夜里游玩似的",
            "[01:11.960]正在迷惘中的我是为何 寻找着你的身影呢",
            "[01:17.590]哒 哒",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 5);
        assert_eq!(
            lines[0].translation.as_deref(),
            Some("那是时光尽头的 剧场世界的序章")
        );
        assert_eq!(lines[4].translation.as_deref(), Some("哒 哒"));
    }

    // ─── だから僕は音楽を辞めた (1357953768) - 大规模 66 行 ─────────

    #[test]
    fn dakara_boku_full_translation_many_lines() {
        // 前 15 行 (3 JSON meta + 12 LRC)
        let jp = [
            "[00:01.080]考えたってわからないし",
            "[00:03.060]青空の下、君を待った",
            "[00:05.180]風が吹いた正午、昼下がりを抜け出す想像",
            "[00:08.310]ねぇ、これからどうなるんだろうね",
            "[00:10.900]進め方教わらないんだよ",
            "[00:12.720]君の目を見た\u{3000}何も言えず僕は歩いた",
            "[00:31.790]考えたってわからないし",
            "[00:33.840]青春なんてつまらないし",
            "[00:35.620]辞めた筈のピアノ、机を弾く癖が抜けない",
            "[00:39.110]ねぇ、将来何してるだろうね",
            "[00:41.540]音楽はしてないといいね",
            "[00:45.110]困らないでよ",
        ]
        .join("\n");
        let cn = [
            "[00:01.080]想过之后依然搞不懂",
            "[00:03.060]在蔚蓝的天空下等待着你",
            "[00:05.180]吹着风的正午 午后的思绪逐渐飘离",
            "[00:08.310]呐，今后该如何是好呢",
            "[00:10.900]向前迈进的方法没有学过啊",
            "[00:12.720]看着你的双眼 什么也没说的就走了",
            "[00:31.790]想过之后依然搞不懂",
            "[00:33.840]青春什麽的无聊透顶",
            "[00:35.620]理当放弃了的钢琴 却改不掉弹奏桌面的习惯",
            "[00:39.110]呐，将来要做什麽好呢",
            "[00:41.540]要是不做音乐就好了",
            "[00:45.110]不要让我困扰啊",
        ]
        .join("\n");

        let detail = LyricDetail {
            lyric: Some(jp),
            tlyric: Some(cn),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 12);

        // 验证 U+3000 表意空格保留
        assert!(lines[5].text.contains('\u{3000}'));
        // 逐行验证翻译
        let pairs: &[(&str, &str)] = &[
            ("考えたってわからないし", "想过之后依然搞不懂"),
            ("青空の下、君を待った", "在蔚蓝的天空下等待着你"),
            (
                "風が吹いた正午、昼下がりを抜け出す想像",
                "吹着风的正午 午后的思绪逐渐飘离",
            ),
            ("ねぇ、これからどうなるんだろうね", "呐，今后该如何是好呢"),
            ("進め方教わらないんだよ", "向前迈进的方法没有学过啊"),
            (
                "君の目を見た\u{3000}何も言えず僕は歩いた",
                "看着你的双眼 什么也没说的就走了",
            ),
            ("考えたってわからないし", "想过之后依然搞不懂"),
            ("青春なんてつまらないし", "青春什麽的无聊透顶"),
            (
                "辞めた筈のピアノ、机を弾く癖が抜けない",
                "理当放弃了的钢琴 却改不掉弹奏桌面的习惯",
            ),
            ("ねぇ、将来何してるだろうね", "呐，将来要做什麽好呢"),
            ("音楽はしてないといいね", "要是不做音乐就好了"),
            ("困らないでよ", "不要让我困扰啊"),
        ];
        for (i, &(jp_text, cn_text)) in pairs.iter().enumerate() {
            assert_eq!(lines[i].text, jp_text, "line {}: expected '{}'", i, jp_text);
            assert_eq!(
                lines[i].translation.as_deref(),
                Some(cn_text),
                "line {}: tl mismatch",
                i
            );
        }
    }

    // ─── 翻译仅有时间戳无文本（类似晴天 186016 的空 tlyric 场景） ──

    #[test]
    fn empty_timestamps_in_tlyric_no_match() {
        let jp = "[00:28.950]故事的小黄花\n[00:32.380]从出生那年就飘着";
        // 真实场景：tlyric 只有时间戳，没有翻译文本
        let cn = "[00:28.950]\n[00:32.380]\n[00:35.870]";
        let detail = LyricDetail {
            lyric: Some(jp.into()),
            tlyric: Some(cn.into()),
            yrc: None,
            ytlrc: None,
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 2);
        // 所有翻译应为 None (tlyric 行无文本)
        for l in &lines {
            assert_eq!(l.translation, None);
        }
    }

    // ─── YRC ytlrc 优先于 tlyric ────────────────────────────────────

    #[test]
    fn ytlrc_overrides_tlyric() {
        let detail = LyricDetail {
            lyric: Some("[00:01.00]Hello\n[00:04.00]World".into()),
            tlyric: Some("[00:01.00]t你好\n[00:04.00]t世界".into()),
            yrc: None,
            ytlrc: Some("[00:01.00]yt你好\n[00:04.00]yt世界".into()),
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines[0].translation.as_deref(), Some("yt你好"));
        assert_eq!(lines[1].translation.as_deref(), Some("yt世界"));
    }

    // ─── YRC verbatim + ytlrc 翻译 ──────────────────────────────────

    #[test]
    fn yrc_verbatim_with_ytlrc_translation() {
        let detail = LyricDetail {
            lyric: None,
            tlyric: None,
            yrc: Some("[1000,2000](1000,500,0)歌(1500,500,0)词".into()),
            ytlrc: Some("[00:01.00]歌词翻译".into()),
            is_pure_music: false,
        };
        let lines = parse_ok(&detail);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "歌词");
        assert_eq!(lines[0].translation.as_deref(), Some("歌词翻译"));
        assert!(matches!(lines[0].kind, LyricLineKind::Verbatim(_)));
    }

    // ─── 纯音乐 ─────────────────────────────────────────────────────

    #[test]
    fn pure_music_returns_none() {
        let detail = LyricDetail {
            lyric: None,
            tlyric: None,
            yrc: None,
            ytlrc: None,
            is_pure_music: true,
        };
        assert_eq!(parse_lyric(&detail), None);
    }

    // ─── "暂无歌词" ─────────────────────────────────────────────────

    #[test]
    fn zanwu_geci_placeholder() {
        let lines = parse_lrc("[00:00.00]暂无歌词");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "暂无歌词");
    }

    // ─── tlyric 含 [by:xxx] + [99Lrc.net] 广告行应被过滤 ─────────

    #[test]
    fn tlyric_with_ad_metadata_filtered() {
        let tlyric =
            "[by:五月蠅_]\n[00:17.18]翻译A\n[00:25.94]翻译B\n[tool:歌词滚动 https://]\n[99:00.00]";
        let lines = parse_lrc(tlyric);
        // [99:00.00] 是3位分钟，但后面无文本 -> 空文本被跳过
        // [tool:...] 无合法时间戳 -> 被跳过
        // [by:...] 无合法时间戳 -> 被跳过
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "翻译A");
        assert_eq!(lines[1].text, "翻译B");
    }
}

// ── 压力/批量测试 ─────────────────────────────────────────────────────────

/// 大量行的解析和配对
#[test]
fn many_lines_stress() {
    let mut lyric_lines = String::new();
    let mut tl_lines = String::new();
    for i in 0..200 {
        let ms = i * 3000;
        let sec = ms / 1000;
        let cs = (ms % 1000) / 10;
        lyric_lines.push_str(&format!(
            "[{:02}:{:02}.{:02}]Line {}\n",
            sec / 60,
            sec % 60,
            cs,
            i
        ));
        tl_lines.push_str(&format!(
            "[{:02}:{:02}.{:02}]翻译{}\n",
            sec / 60,
            sec % 60,
            cs,
            i
        ));
    }
    let detail = mk_detail(Some(&lyric_lines), Some(&tl_lines), None, None, false);
    let lines = parse_lyric(&detail).unwrap();
    assert_eq!(lines.len(), 200);
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(
            line.translation.as_deref(),
            Some(&format!("翻译{}", i) as &str)
        );
    }
}
