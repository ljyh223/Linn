//! Shared Pango layout contract for the Cairo and OpenGL lyric backends.

use std::ops::Range;

use pangocairo::pango;

use crate::lyrics::{LyricsDocument, LyricsPresentation};

pub(crate) const TRANSLATION_GAP: f64 = 4.0;
pub(crate) const TOP_PADDING: f64 = 48.0;

const MAIN_FONT_SIDEBAR: i32 = 20;
const MAIN_FONT_FULLSCREEN: i32 = 24;
const TRANSLATION_FONT_SIDEBAR: i32 = 13;
const TRANSLATION_FONT_FULLSCREEN: i32 = 15;
const LINE_GAP_SIDEBAR: f64 = 22.0;
const LINE_GAP_FULLSCREEN: f64 = 28.0;

pub(crate) struct PangoLineLayout {
    pub(crate) layout: pango::Layout,
    pub(crate) translation: Option<pango::Layout>,
    pub(crate) y: f64,
    pub(crate) main_height: f64,
    pub(crate) total_height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextRectangle {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) rtl: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SoftRevealGeometry {
    pub(crate) clip: TextRectangle,
    pub(crate) edge_x: f64,
}

/// Let the soft karaoke edge reach beyond a short source span. CJK providers
/// commonly time one character per span; clipping to that glyph makes the
/// highlight look like a typewriter even when the shader/gradient is soft.
pub(crate) fn soft_reveal_geometry(
    rectangle: TextRectangle,
    progress: f64,
    softness: f64,
    layout_width: f64,
) -> SoftRevealGeometry {
    let reveal_width = rectangle.width * progress.clamp(0.0, 1.0);
    let edge_x = if rectangle.rtl {
        rectangle.x + rectangle.width - reveal_width
    } else {
        rectangle.x + reveal_width
    };
    let (start, end) = if rectangle.rtl {
        (
            (edge_x - softness).max(0.0),
            (rectangle.x + rectangle.width).min(layout_width),
        )
    } else {
        (rectangle.x.max(0.0), (edge_x + softness).min(layout_width))
    };
    SoftRevealGeometry {
        clip: TextRectangle {
            x: start,
            width: (end - start).max(0.0),
            ..rectangle
        },
        edge_x,
    }
}

pub(crate) fn layout_document(
    context: &pango::Context,
    document: &LyricsDocument,
    presentation: LyricsPresentation,
    available_width: i32,
) -> Vec<PangoLineLayout> {
    let (main_size, translation_size, line_gap) = match presentation {
        LyricsPresentation::Sidebar => (
            MAIN_FONT_SIDEBAR,
            TRANSLATION_FONT_SIDEBAR,
            LINE_GAP_SIDEBAR,
        ),
        LyricsPresentation::Fullscreen => (
            MAIN_FONT_FULLSCREEN,
            TRANSLATION_FONT_FULLSCREEN,
            LINE_GAP_FULLSCREEN,
        ),
    };

    let mut y = TOP_PADDING;
    document
        .lines
        .iter()
        .map(|line| {
            let layout = make_layout(context, main_size, available_width, true);
            layout.set_text(&line.text);
            let main_height = layout.pixel_size().1.max(1) as f64;

            let translation = line.translation.as_ref().map(|text| {
                let layout = make_layout(context, translation_size, available_width, false);
                layout.set_text(text);
                layout
            });
            let translation_height = translation
                .as_ref()
                .map_or(0.0, |layout| layout.pixel_size().1.max(1) as f64);
            let total_height = main_height
                + if translation_height > 0.0 {
                    TRANSLATION_GAP + translation_height
                } else {
                    0.0
                };
            let cached = PangoLineLayout {
                layout,
                translation,
                y,
                main_height,
                total_height,
            };
            y += total_height + line_gap;
            cached
        })
        .collect()
}

pub(crate) fn make_layout(
    context: &pango::Context,
    font_size: i32,
    available_width: i32,
    bold: bool,
) -> pango::Layout {
    let layout = pango::Layout::new(context);
    let mut description = pango::FontDescription::new();
    description.set_family("Sans");
    description.set_size(font_size * pango::SCALE);
    description.set_weight(if bold {
        pango::Weight::Bold
    } else {
        pango::Weight::Normal
    });
    layout.set_font_description(Some(&description));
    layout.set_width(available_width.max(1) * pango::SCALE);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout
}

/// Find the conservative visible line window without walking the whole song.
/// Layout Y positions are monotonic, and an interlude offset is a bounded
/// positive step, so two partition-point searches are sufficient.
pub(crate) fn visible_line_range<F>(
    line_count: usize,
    scroll_y: f64,
    maximum_line_offset: f64,
    viewport_height: i32,
    geometry: F,
) -> Range<usize>
where
    F: Fn(usize) -> (f64, f64) + Copy,
{
    if line_count == 0 {
        return 0..0;
    }
    let top = scroll_y - 8.0;
    let bottom = scroll_y + viewport_height.max(1) as f64 + 8.0;
    let maximum_line_offset = maximum_line_offset.max(0.0);
    let start = partition_indices(line_count, |index| {
        let (y, height) = geometry(index);
        y + height + maximum_line_offset < top
    });
    let end = partition_indices(line_count, |index| {
        let (y, _) = geometry(index);
        y <= bottom
    });
    start.min(end)..end
}

pub(crate) fn line_index_at_viewport_y<F>(
    line_count: usize,
    viewport_y: f64,
    scroll_y: f64,
    geometry: F,
) -> Option<usize>
where
    F: Fn(usize) -> (f64, f64),
{
    let content_y = viewport_y + scroll_y;
    (0..line_count).find(|&index| {
        let (y, height) = geometry(index);
        content_y >= y - 8.0 && content_y <= y + height + 8.0
    })
}

fn partition_indices(mut length: usize, mut predicate: impl FnMut(usize) -> bool) -> usize {
    let mut left = 0;
    while length > 0 {
        let half = length / 2;
        let middle = left + half;
        if predicate(middle) {
            left = middle + 1;
            length -= half + 1;
        } else {
            length = half;
        }
    }
    left
}

pub(crate) fn range_rectangles(layout: &pango::Layout, range: Range<usize>) -> Vec<TextRectangle> {
    let mut rectangles = Vec::new();
    for line in layout.lines_readonly() {
        let line_start = line.start_index().max(0) as usize;
        let line_end = line_start + line.length().max(0) as usize;
        let start = range.start.max(line_start);
        let end = range.end.min(line_end);
        if start >= end {
            continue;
        }

        // LayoutLine extents are relative to that line's baseline, so their Y is
        // commonly a negative ascent. `index_to_pos` is relative to the layout
        // origin, which is the coordinate space used by both the cached texture
        // and our Cairo/GL clipping rectangles.
        let position = layout.index_to_pos(line_start as i32);
        let y = position.y() as f64 / pango::SCALE as f64;
        let height = position.height().max(pango::SCALE) as f64 / pango::SCALE as f64;
        let rtl = matches!(
            line.resolved_direction(),
            pango::Direction::Rtl | pango::Direction::WeakRtl | pango::Direction::TtbRtl
        );
        let mut line_rectangles = Vec::new();
        for pair in line.x_ranges(start as i32, end as i32).chunks_exact(2) {
            let left = pair[0].min(pair[1]) as f64 / pango::SCALE as f64;
            let right = pair[0].max(pair[1]) as f64 / pango::SCALE as f64;
            if right > left {
                line_rectangles.push(TextRectangle {
                    x: left,
                    y,
                    width: right - left,
                    height,
                    rtl,
                });
            }
        }
        line_rectangles.sort_by(|a, b| {
            let ordering = a.x.total_cmp(&b.x);
            if rtl { ordering.reverse() } else { ordering }
        });
        rectangles.extend(line_rectangles);
    }
    rectangles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{LyricsSource, RawLyrics, parse_document};
    use pangocairo::pango::prelude::FontMapExt;

    #[test]
    fn utf8_range_uses_pango_byte_boundaries() {
        let context = pangocairo::FontMap::default().create_context();
        let layout = make_layout(&context, 20, 400, true);
        layout.set_text("A歌词B");
        let start = "A".len();
        let end = start + "歌词".len();
        let rectangles = range_rectangles(&layout, start..end);

        assert!(!rectangles.is_empty());
        assert!(rectangles.iter().all(|rectangle| rectangle.width > 0.0));
        assert!(rectangles.iter().all(|rectangle| rectangle.y >= 0.0));
        assert!(rectangles.iter().all(|rectangle| {
            rectangle.y + rectangle.height <= layout.pixel_size().1 as f64 + 1.0
        }));
    }

    #[test]
    fn click_mapping_follows_a_negative_initial_scroll() {
        let geometry = [(48.0, 40.0), (110.0, 40.0)];
        let scroll_y = -144.0;
        let first_line_viewport_center = geometry[0].0 + geometry[0].1 * 0.5 - scroll_y;

        assert_eq!(
            line_index_at_viewport_y(
                geometry.len(),
                first_line_viewport_center,
                scroll_y,
                |index| geometry[index],
            ),
            Some(0)
        );
    }

    #[test]
    fn rtl_range_preserves_visual_direction() {
        let context = pangocairo::FontMap::default().create_context();
        let layout = make_layout(&context, 20, 400, true);
        let text = "مرحبا";
        layout.set_text(text);
        let rectangles = range_rectangles(&layout, 0..text.len());

        assert!(!rectangles.is_empty());
        assert!(rectangles.iter().all(|rectangle| rectangle.rtl));
    }

    #[test]
    fn wrapped_range_rectangles_follow_each_layout_row() {
        let context = pangocairo::FontMap::default().create_context();
        let layout = make_layout(&context, 20, 90, true);
        let text = "一段很长的逐字歌词需要换行并保持高亮位置正确";
        layout.set_text(text);
        let rectangles = range_rectangles(&layout, 0..text.len());
        let mut rows = rectangles
            .iter()
            .map(|rectangle| rectangle.y)
            .collect::<Vec<_>>();
        rows.sort_by(f64::total_cmp);
        rows.dedup_by(|a, b| (*a - *b).abs() < 0.5);

        assert!(rows.len() > 1, "fixture must wrap onto multiple Pango rows");
        assert!(rows.windows(2).all(|pair| pair[1] > pair[0]));
        assert!(rectangles.iter().all(|rectangle| {
            rectangle.y >= 0.0
                && rectangle.y + rectangle.height <= layout.pixel_size().1 as f64 + 1.0
        }));
    }

    #[test]
    fn cjk_character_reveal_spills_into_the_following_glyph() {
        let character = TextRectangle {
            x: 24.0,
            y: 0.0,
            width: 24.0,
            height: 28.0,
            rtl: false,
        };
        let reveal = soft_reveal_geometry(character, 0.5, 42.0, 240.0);

        assert!(reveal.clip.width > character.width);
        assert_eq!(reveal.edge_x, 36.0);
        assert!(reveal.clip.x + reveal.clip.width <= 240.0);
    }

    #[test]
    fn lyric_fixture_matrix_layouts_every_source_in_both_presentations() {
        let fixtures = [
            (
                "lrc",
                RawLyrics {
                    lyric: Some(include_str!("../../../../tests/fixtures/lyrics/basic.lrc")),
                    translation: None,
                    word_synced: None,
                    word_translation: None,
                    word_source: LyricsSource::Unknown,
                    is_pure_music: false,
                },
            ),
            (
                "yrc",
                RawLyrics {
                    lyric: Some(include_str!("../../../../tests/fixtures/lyrics/basic.lrc")),
                    translation: None,
                    word_synced: Some(include_str!(
                        "../../../../tests/fixtures/lyrics/word-timed.yrc"
                    )),
                    word_translation: None,
                    word_source: LyricsSource::Yrc,
                    is_pure_music: false,
                },
            ),
            (
                "qrc",
                RawLyrics {
                    lyric: Some(include_str!("../../../../tests/fixtures/lyrics/basic.lrc")),
                    translation: None,
                    word_synced: Some(include_str!(
                        "../../../../tests/fixtures/lyrics/word-timed.qrc"
                    )),
                    word_translation: None,
                    word_source: LyricsSource::Qrc,
                    is_pure_music: false,
                },
            ),
            (
                "ttml",
                RawLyrics {
                    lyric: Some(include_str!(
                        "../../../../tests/fixtures/lyrics/karaoke.ttml"
                    )),
                    translation: None,
                    word_synced: None,
                    word_translation: None,
                    word_source: LyricsSource::Ttml,
                    is_pure_music: false,
                },
            ),
        ];
        let context = pangocairo::FontMap::default().create_context();

        for (fixture, raw) in fixtures {
            let document = parse_document(raw).unwrap().unwrap();
            for (presentation, width) in [
                (LyricsPresentation::Sidebar, 320),
                (LyricsPresentation::Fullscreen, 720),
            ] {
                let layouts = layout_document(&context, &document, presentation, width);
                assert_eq!(layouts.len(), document.lines.len(), "{fixture}");
                assert!(!layouts.is_empty(), "{fixture}");

                for (index, (line, layout)) in document.lines.iter().zip(layouts.iter()).enumerate()
                {
                    assert!(layout.main_height > 0.0, "{fixture} line {index}");
                    assert!(
                        layout.total_height >= layout.main_height,
                        "{fixture} line {index}"
                    );
                    assert!(
                        layout.layout.pixel_size().0 <= width,
                        "{fixture} line {index}"
                    );
                    if index > 0 {
                        assert!(layout.y > layouts[index - 1].y, "{fixture} line {index}");
                    }
                    for span in line.spans.iter() {
                        let rectangles = range_rectangles(&layout.layout, span.text_range.clone());
                        assert!(
                            !rectangles.is_empty(),
                            "{fixture} {presentation:?} line {index} span {:?} has no glyph geometry",
                            span.text_range
                        );
                        assert!(
                            rectangles.iter().all(|rectangle| {
                                rectangle.y >= 0.0
                                    && rectangle.y + rectangle.height <= layout.main_height + 1.0
                            }),
                            "{fixture} {presentation:?} line {index} span geometry must use layout-local Y coordinates"
                        );
                    }
                }
            }
        }
    }
}
