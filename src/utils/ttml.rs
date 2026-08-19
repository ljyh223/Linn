use anyhow::{Context, anyhow};
use quick_xml::{Reader, events::Event};
use std::collections::HashMap;

use crate::ui::model::{LyricChar, LyricLine, LyricLineKind};

#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: HashMap<String, String>,
    children: Vec<Node>,
    text: String,
}

fn local_name(name: &[u8]) -> String {
    let name = String::from_utf8_lossy(name);
    name.rsplit(':').next().unwrap_or_default().to_string()
}

fn parse_xml(raw: &str) -> anyhow::Result<Node> {
    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;

    loop {
        match reader.read_event().context("invalid TTML document")? {
            Event::Start(event) => {
                let mut node = Node {
                    name: local_name(event.name().as_ref()),
                    ..Node::default()
                };
                for attr in event.attributes().with_checks(false) {
                    let attr = attr.context("invalid TTML attribute")?;
                    let key = local_name(attr.key.as_ref());
                    let value = attr
                        .decode_and_unescape_value(reader.decoder())?
                        .into_owned();
                    node.attrs.insert(key, value);
                }
                stack.push(node);
            }
            Event::Empty(event) => {
                let mut node = Node {
                    name: local_name(event.name().as_ref()),
                    ..Node::default()
                };
                for attr in event.attributes().with_checks(false) {
                    let attr = attr.context("invalid TTML attribute")?;
                    node.attrs.insert(
                        local_name(attr.key.as_ref()),
                        attr.decode_and_unescape_value(reader.decoder())?
                            .into_owned(),
                    );
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::Text(event) => {
                if let Some(node) = stack.last_mut() {
                    node.children.push(Node {
                        name: "#text".into(),
                        text: event.decode()?.into_owned(),
                        ..Node::default()
                    });
                }
            }
            Event::CData(event) => {
                if let Some(node) = stack.last_mut() {
                    node.children.push(Node {
                        name: "#text".into(),
                        text: String::from_utf8_lossy(&event).into_owned(),
                        ..Node::default()
                    });
                }
            }
            Event::End(_) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| anyhow!("unbalanced TTML document"))?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.ok_or_else(|| anyhow!("empty TTML document"))
}

fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attrs.get(name).map(String::as_str)
}

fn role(node: &Node, expected: &str) -> bool {
    node.attrs
        .iter()
        .any(|(name, value)| name == "role" && value == expected)
}

fn text_content(node: &Node) -> String {
    let mut text = node.text.clone();
    for child in &node.children {
        text.push_str(&text_content(child));
    }
    text
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
}

fn parse_time(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(ms) = value.strip_suffix("ms") {
        return ms.trim().parse::<f64>().ok().map(|v| v.max(0.0) as u64);
    }
    if let Some(sec) = value.strip_suffix('s') {
        return sec
            .trim()
            .parse::<f64>()
            .ok()
            .map(|v| (v.max(0.0) * 1000.0) as u64);
    }
    let parts: Vec<_> = value.split(':').collect();
    let seconds = match parts.as_slice() {
        [mm, ss] => mm.parse::<f64>().ok()? * 60.0 + ss.parse::<f64>().ok()?,
        [hh, mm, ss] => {
            hh.parse::<f64>().ok()? * 3600.0
                + mm.parse::<f64>().ok()? * 60.0
                + ss.parse::<f64>().ok()?
        }
        _ => return None,
    };
    Some((seconds.max(0.0) * 1000.0) as u64)
}

fn find_nodes<'a>(node: &'a Node, name: &str, out: &mut Vec<&'a Node>) {
    if node.name == name {
        out.push(node);
    }
    for child in &node.children {
        find_nodes(child, name, out);
    }
}

fn translations(root: &Node) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut nodes = Vec::new();
    find_nodes(root, "translation", &mut nodes);
    for translation in nodes {
        for text in translation
            .children
            .iter()
            .filter(|child| child.name == "text")
        {
            if let Some(key) = attr(text, "for") {
                let value = decode_entities(&text_content(text)).trim().to_string();
                if !value.is_empty() {
                    result.insert(key.to_string(), value);
                }
            }
        }
    }
    result
}

fn split_translation(value: &str) -> String {
    let value = value.trim();
    if value.ends_with('）') {
        if let Some(index) = value.rfind('（') {
            return value[..index].trim().to_string();
        }
    }
    value.to_string()
}

pub fn is_ttml(raw: &str) -> bool {
    raw.contains("http://www.w3.org/ns/ttml") || raw.contains("<tt")
}

pub fn parse_ttml(raw: &str) -> anyhow::Result<Vec<LyricLine>> {
    let root = parse_xml(raw)?;
    let translation_map = translations(&root);
    let mut paragraphs = Vec::new();
    find_nodes(&root, "p", &mut paragraphs);
    let mut lines = Vec::new();

    for p in paragraphs {
        let Some(start) = attr(p, "begin").and_then(parse_time) else {
            continue;
        };
        let end = attr(p, "end").and_then(parse_time);
        let key = attr(p, "key");
        let inline_translation = p
            .children
            .iter()
            .find(|child| {
                child.name == "span" && role(child, "x-translation") && !role(child, "x-bg")
            })
            .map(text_content)
            .map(|text| decode_entities(&text).trim().to_string())
            .filter(|text| !text.is_empty());

        let mut chars = Vec::new();
        for (index, span) in p.children.iter().enumerate().filter(|(_, child)| {
            child.name == "span"
                && !role(child, "x-translation")
                && !role(child, "x-bg")
                && !role(child, "x-roman")
        }) {
            let (Some(char_start), Some(char_end)) = (
                attr(span, "begin").and_then(parse_time),
                attr(span, "end").and_then(parse_time),
            ) else {
                continue;
            };
            let mut text = decode_entities(&text_content(span));
            if let Some(next) = p
                .children
                .get(index + 1)
                .filter(|child| child.name == "#text")
            {
                text.push_str(&decode_entities(&next.text));
            }
            for ch in text.chars() {
                chars.push(LyricChar {
                    ch: ch.to_string(),
                    start: char_start,
                    duration: char_end.saturating_sub(char_start),
                });
            }
        }
        let plain_text = p
            .children
            .iter()
            .filter(|child| {
                !(child.name == "span"
                    && (role(child, "x-translation")
                        || role(child, "x-bg")
                        || role(child, "x-roman")))
            })
            .map(text_content)
            .collect::<String>();
        let text = if chars.is_empty() {
            decode_entities(&plain_text).trim().to_string()
        } else {
            chars.iter().map(|ch| ch.ch.as_str()).collect()
        };
        if text.is_empty() {
            continue;
        }
        let duration = end.map(|end| end.saturating_sub(start)).unwrap_or(0);
        let translation = inline_translation.or_else(|| {
            key.and_then(|key| {
                translation_map
                    .get(key)
                    .map(|value| split_translation(value))
            })
        });
        lines.push(LyricLine {
            start,
            duration,
            text,
            kind: if chars.is_empty() {
                LyricLineKind::Plain
            } else {
                LyricLineKind::Verbatim(chars)
            },
            translation,
        });
    }

    lines.sort_by_key(|line| line.start);
    for index in 0..lines.len() {
        if lines[index].duration == 0 {
            lines[index].duration = lines
                .get(index + 1)
                .map(|next| next.start.saturating_sub(lines[index].start))
                .unwrap_or(5000);
        }
    }
    log::info!(
        "[lyrics][ttml] parsed lines={} chars={}",
        lines.len(),
        lines
            .iter()
            .map(|line| match &line.kind {
                LyricLineKind::Verbatim(chars) => chars.len(),
                LyricLineKind::Plain => 0,
            })
            .sum::<usize>()
    );
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::parse_ttml;

    #[test]
    fn parses_amll_karaoke_and_translation() {
        let raw = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><p begin="1s" end="3s" itunes:key="L1"><span begin="1s" end="2s">你</span><span begin="2s" end="3s">好</span><span ttm:role="x-translation">hello</span></p></body></tt>"#;
        let lines = parse_ttml(raw).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].start, 1000);
        assert_eq!(lines[0].text, "你好");
        assert_eq!(lines[0].translation.as_deref(), Some("hello"));
    }
}
