use std::collections::HashMap;

use anyhow::{Context, anyhow};
use quick_xml::{Reader, events::Event};

use super::{LyricsDocument, LyricsLine, LyricsSource, TimedSpan, TimingQuality};

#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: HashMap<String, String>,
    children: Vec<Node>,
    text: String,
}

#[derive(Debug)]
struct PendingLine {
    start_ms: u64,
    end_ms: Option<u64>,
    text: String,
    translation: Option<String>,
    spans: Vec<TimedSpan>,
}

pub fn is_ttml(raw: &str) -> bool {
    raw.contains("http://www.w3.org/ns/ttml") || raw.contains("<tt")
}

pub fn parse_ttml(raw: &str) -> anyhow::Result<LyricsDocument> {
    let root = parse_xml(raw)?;
    let translations = translation_map(&root);
    let mut paragraphs = Vec::new();
    find_nodes(&root, "p", &mut paragraphs);
    let mut pending = Vec::new();

    for paragraph in paragraphs {
        let Some(start_ms) = attr(paragraph, "begin").and_then(parse_time) else {
            continue;
        };
        let declared_end = attr(paragraph, "end").and_then(parse_time);
        let inline_translation = paragraph
            .children
            .iter()
            .find(|node| {
                node.name == "span" && has_role(node, "x-translation") && !has_role(node, "x-bg")
            })
            .map(text_content)
            .map(|text| decode_entities(&text).trim().to_owned())
            .filter(|text| !text.is_empty());
        let linked_translation = attr(paragraph, "key")
            .and_then(|key| translations.get(key))
            .map(|text| split_translation(text));

        let mut text = String::new();
        let mut spans = Vec::new();
        for (index, span) in paragraph.children.iter().enumerate().filter(|(_, node)| {
            node.name == "span"
                && !has_role(node, "x-translation")
                && !has_role(node, "x-bg")
                && !has_role(node, "x-roman")
        }) {
            let (Some(span_start), Some(span_end)) = (
                attr(span, "begin").and_then(parse_time),
                attr(span, "end").and_then(parse_time),
            ) else {
                continue;
            };
            if span_end <= span_start {
                continue;
            }
            let mut segment = decode_entities(&text_content(span));
            if let Some(separator) = paragraph
                .children
                .get(index + 1)
                .filter(|node| node.name == "#text")
            {
                segment.push_str(&decode_entities(&separator.text));
            }
            if segment.is_empty() {
                continue;
            }
            let range_start = text.len();
            text.push_str(&segment);
            spans.push(TimedSpan::new(
                range_start..text.len(),
                span_start,
                span_end,
            ));
        }

        if spans.is_empty() {
            text = paragraph
                .children
                .iter()
                .filter(|node| {
                    !(node.name == "span"
                        && (has_role(node, "x-translation")
                            || has_role(node, "x-bg")
                            || has_role(node, "x-roman")))
                })
                .map(text_content)
                .collect::<String>();
            text = decode_entities(&text).trim().to_owned();
        }
        if text.is_empty() {
            continue;
        }
        pending.push(PendingLine {
            start_ms,
            end_ms: declared_end,
            text,
            translation: inline_translation.or(linked_translation),
            spans,
        });
    }
    pending.sort_by_key(|line| line.start_ms);

    let mut lines = Vec::with_capacity(pending.len());
    for (index, line) in pending.iter().enumerate() {
        let timed_end = line.spans.iter().map(|span| span.end_ms).max();
        let fallback_end = pending
            .get(index + 1)
            .map_or(line.start_ms.saturating_add(5_000), |next| next.start_ms);
        let end_ms = line
            .end_ms
            .into_iter()
            .chain(timed_end)
            .max()
            .unwrap_or(fallback_end)
            .max(line.start_ms);
        lines.push(LyricsLine::new(
            line.start_ms,
            end_ms,
            line.text.as_str(),
            line.translation.as_deref().map(Into::into),
            if line.spans.is_empty() {
                TimingQuality::Line
            } else {
                TimingQuality::Source
            },
            line.spans.clone(),
        )?);
    }
    Ok(LyricsDocument::new(LyricsSource::Ttml, lines)?)
}

fn parse_xml(raw: &str) -> anyhow::Result<Node> {
    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(false);
    let mut stack = Vec::<Node>::new();
    let mut root = None;
    loop {
        match reader.read_event().context("invalid TTML document")? {
            Event::Start(event) => stack.push(node_from_event(&reader, &event)?),
            Event::Empty(event) => {
                let node = node_from_event(&reader, &event)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::Text(event) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node {
                        name: "#text".into(),
                        text: event.decode()?.into_owned(),
                        ..Node::default()
                    });
                }
            }
            Event::CData(event) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node {
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

fn node_from_event(
    reader: &Reader<&[u8]>,
    event: &quick_xml::events::BytesStart<'_>,
) -> anyhow::Result<Node> {
    let mut node = Node {
        name: local_name(event.name().as_ref()),
        ..Node::default()
    };
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute.context("invalid TTML attribute")?;
        node.attrs.insert(
            local_name(attribute.key.as_ref()),
            attribute
                .decode_and_unescape_value(reader.decoder())?
                .into_owned(),
        );
    }
    Ok(node)
}

fn local_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name)
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attrs.get(name).map(String::as_str)
}

fn has_role(node: &Node, role: &str) -> bool {
    node.attrs
        .iter()
        .any(|(name, value)| name == "role" && value == role)
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
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return milliseconds
            .trim()
            .parse::<f64>()
            .ok()
            .map(|value| value.max(0.0) as u64);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return seconds
            .trim()
            .parse::<f64>()
            .ok()
            .map(|value| (value.max(0.0) * 1_000.0) as u64);
    }
    let parts: Vec<_> = value.split(':').collect();
    let seconds = match parts.as_slice() {
        [minutes, seconds] => minutes.parse::<f64>().ok()? * 60.0 + seconds.parse::<f64>().ok()?,
        [hours, minutes, seconds] => {
            hours.parse::<f64>().ok()? * 3_600.0
                + minutes.parse::<f64>().ok()? * 60.0
                + seconds.parse::<f64>().ok()?
        }
        _ => return None,
    };
    Some((seconds.max(0.0) * 1_000.0) as u64)
}

fn find_nodes<'a>(node: &'a Node, name: &str, output: &mut Vec<&'a Node>) {
    if node.name == name {
        output.push(node);
    }
    for child in &node.children {
        find_nodes(child, name, output);
    }
}

fn translation_map(root: &Node) -> HashMap<String, String> {
    let mut translations = HashMap::new();
    let mut containers = Vec::new();
    find_nodes(root, "translation", &mut containers);
    for text in containers
        .into_iter()
        .flat_map(|container| container.children.iter())
        .filter(|node| node.name == "text")
    {
        if let Some(key) = attr(text, "for") {
            let value = decode_entities(&text_content(text)).trim().to_owned();
            if !value.is_empty() {
                translations.insert(key.to_owned(), value);
            }
        }
    }
    translations
}

fn split_translation(value: &str) -> String {
    let value = value.trim();
    value
        .strip_suffix('）')
        .and_then(|without_suffix| {
            without_suffix
                .rsplit_once('（')
                .map(|(translation, _)| translation.trim().to_owned())
        })
        .unwrap_or_else(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_ttml_spans_instead_of_copying_timing_to_each_character() {
        let document =
            parse_ttml(include_str!("../../tests/fixtures/lyrics/karaoke.ttml")).unwrap();
        let line = &document.lines[0];
        assert_eq!(&*line.text, "Hello 世界");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(&line.text[line.spans[0].text_range.clone()], "Hello ");
        assert_eq!(&line.text[line.spans[1].text_range.clone()], "世界");
        assert_eq!(line.translation.as_deref(), Some("你好，世界"));
    }

    #[test]
    fn rejects_invalid_xml() {
        assert!(parse_ttml("<tt><body>").is_err());
    }
}
