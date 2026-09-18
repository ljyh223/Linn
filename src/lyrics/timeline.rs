use std::ops::Range;

use super::model::{LyricsDocument, TimingQuality};

const INTERLUDE_THRESHOLD_MS: u64 = 2_000;
const INTERLUDE_ENTER_MS: u64 = 3_000;
const INTERLUDE_PRE_EXIT_MS: u64 = 3_000;
const INTERLUDE_STILL_MS: u64 = 200;
const INTERLUDE_EXIT_MS: u64 = 200;
const INTERLUDE_BREATH_MS: f64 = 3_000.0;

/// Everything a renderer needs for a single presentation frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FramePlan {
    /// The line the viewport should follow. Before the first line this is 0;
    /// in a gap it remains on the most recently started line.
    pub focus_line: Option<usize>,
    /// The line whose source interval currently contains the playback time.
    pub active_line: Option<usize>,
    pub line_progress: f32,
    pub karaoke: Option<KaraokePlan>,
    /// Present only while playback is inside a source-authored long gap.
    /// Renderers consume these deterministic values instead of maintaining
    /// their own gap-detection and animation clocks.
    pub interlude: Option<InterludePlan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KaraokePlan {
    /// UTF-8 byte boundary through which all source spans have completed.
    pub completed_text_end: usize,
    /// The source-authored range currently being sung.
    pub active_text_range: Option<Range<usize>>,
    pub active_span_progress: f32,
    /// The next source-authored range while playback is inside a timing gap.
    /// Renderers keep the soft edge parked here instead of blinking it off.
    pub upcoming_text_range: Option<Range<usize>>,
    /// Estimated timing is visible to the backend, which may choose a more
    /// conservative effect than it uses for source-authored timing.
    pub timing_quality: TimingQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterludePlan {
    /// `None` means the gap precedes the first line; otherwise this is the
    /// line immediately before the gap.
    pub after_line: Option<usize>,
    pub alpha: f32,
    pub scale: f32,
    pub reveal: f32,
    pub dot_alphas: [f32; 3],
}

pub fn frame_at(document: &LyricsDocument, position_ms: u64) -> FramePlan {
    let lines = &document.lines;
    if lines.is_empty() {
        return FramePlan {
            focus_line: None,
            active_line: None,
            line_progress: 0.0,
            karaoke: None,
            interlude: None,
        };
    }

    let started = lines.partition_point(|line| line.start_ms <= position_ms);
    let focus_line = started.checked_sub(1).or(Some(0));
    let active_line = started.checked_sub(1).filter(|&index| {
        let line = &lines[index];
        position_ms >= line.start_ms && position_ms < line.end_ms
    });
    let interlude = active_line
        .is_none()
        .then(|| interlude_at(document, position_ms, started))
        .flatten();

    let Some(index) = active_line else {
        return FramePlan {
            focus_line,
            active_line: None,
            line_progress: 0.0,
            karaoke: None,
            interlude,
        };
    };

    let line = &lines[index];
    let line_progress = interval_progress(position_ms, line.start_ms, line.end_ms);
    let karaoke = (line.timing_quality != TimingQuality::Line).then(|| {
        let completed_count = line
            .spans
            .partition_point(|span| span.end_ms <= position_ms);
        let completed_text_end = line.spans[..completed_count]
            .last()
            .map_or(0, |span| span.text_range.end);

        let active_index = line
            .spans
            .partition_point(|span| span.start_ms <= position_ms)
            .checked_sub(1)
            .filter(|&span_index| position_ms < line.spans[span_index].end_ms);

        let (active_text_range, active_span_progress) =
            active_index.map_or((None, 0.0), |span_index| {
                let span = &line.spans[span_index];
                (
                    Some(span.text_range.clone()),
                    interval_progress(position_ms, span.start_ms, span.end_ms),
                )
            });
        let upcoming_text_range = active_index
            .is_none()
            .then(|| {
                line.spans
                    .get(completed_count)
                    .map(|span| span.text_range.clone())
            })
            .flatten();

        KaraokePlan {
            completed_text_end,
            active_text_range,
            active_span_progress,
            upcoming_text_range,
            timing_quality: line.timing_quality,
        }
    });

    FramePlan {
        focus_line,
        active_line: Some(index),
        line_progress,
        karaoke,
        interlude,
    }
}

fn interlude_at(
    document: &LyricsDocument,
    position_ms: u64,
    started_line_count: usize,
) -> Option<InterludePlan> {
    let (after_line, start_ms, end_ms) = if started_line_count == 0 {
        let end_ms = document.lines.first()?.start_ms;
        (None, 0, end_ms)
    } else {
        let index = started_line_count - 1;
        let current = document.lines.get(index)?;
        let next = document.lines.get(index + 1)?;
        (Some(index), current.end_ms, next.start_ms)
    };

    let duration = end_ms.saturating_sub(start_ms);
    if duration < INTERLUDE_THRESHOLD_MS
        || (after_line.is_some() && duration == INTERLUDE_THRESHOLD_MS)
        || position_ms < start_ms
        || position_ms >= end_ms
    {
        return None;
    }

    let fixed_total =
        INTERLUDE_ENTER_MS + INTERLUDE_PRE_EXIT_MS + INTERLUDE_STILL_MS + INTERLUDE_EXIT_MS;
    let factor = (duration as f64 / fixed_total as f64).min(1.0);
    let enter_duration = (INTERLUDE_ENTER_MS as f64 * factor).round() as u64;
    let pre_exit_duration = (INTERLUDE_PRE_EXIT_MS as f64 * factor).round() as u64;
    let still_duration = (INTERLUDE_STILL_MS as f64 * factor).round() as u64;
    let exit_duration = duration
        .saturating_sub(enter_duration)
        .saturating_sub(pre_exit_duration)
        .saturating_sub(still_duration);
    let enter_end = start_ms + enter_duration;
    let exit_start = end_ms.saturating_sub(exit_duration);
    let still_start = exit_start.saturating_sub(still_duration);
    let pre_exit_start = still_start.saturating_sub(pre_exit_duration);

    let (alpha, scale, reveal) = if position_ms < enter_end {
        let eased = ease_in_out_cubic(interval_progress(position_ms, start_ms, enter_end));
        (eased, eased * 0.8, eased)
    } else if position_ms < pre_exit_start {
        let phase = (position_ms - enter_end) as f64 / INTERLUDE_BREATH_MS;
        let scale = 0.9 - 0.1 * (phase * std::f64::consts::TAU).cos();
        (1.0, scale as f32, 1.0)
    } else if position_ms < still_start {
        let progress = interval_progress(position_ms, pre_exit_start, still_start) as f64;
        let scale = 0.8 + 0.2 * (progress * std::f64::consts::TAU).cos();
        (1.0, scale as f32, 1.0)
    } else if position_ms < exit_start {
        (1.0, 1.0, 1.0)
    } else {
        let eased = ease_in_out_cubic(1.0 - interval_progress(position_ms, exit_start, end_ms));
        (eased, eased, 1.0)
    };

    let wave_start = enter_end;
    let wave_end = pre_exit_start;
    let wave_duration = wave_end.saturating_sub(wave_start) as f32;
    let dot_alphas = std::array::from_fn(|index| {
        if position_ms < enter_end {
            return 0.4;
        }
        if wave_duration <= 0.0 || position_ms >= exit_start {
            return 1.0;
        }
        let slot = wave_duration / 3.0;
        let dot_start = wave_start as f32 + index as f32 * slot;
        (0.4 + 0.6 * ((position_ms as f32 - dot_start) / slot).clamp(0.0, 1.0)).min(1.0)
    });

    Some(InterludePlan {
        after_line,
        alpha,
        scale,
        reveal,
        dot_alphas,
    })
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn interval_progress(position_ms: u64, start_ms: u64, end_ms: u64) -> f32 {
    let duration = end_ms.saturating_sub(start_ms);
    if duration == 0 {
        return 1.0;
    }
    (position_ms.saturating_sub(start_ms) as f64 / duration as f64).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::lyrics::{LyricsLine, LyricsSource, TimedSpan};

    fn document() -> LyricsDocument {
        LyricsDocument::new(
            LyricsSource::Yrc,
            vec![
                LyricsLine::new(
                    1_000,
                    3_000,
                    "歌词",
                    None,
                    TimingQuality::Source,
                    vec![
                        TimedSpan::new(0..3, 1_000, 2_000),
                        TimedSpan::new(3..6, 2_000, 3_000),
                    ],
                )
                .unwrap(),
                LyricsLine::new(
                    4_000,
                    5_000,
                    Arc::<str>::from("next"),
                    None,
                    TimingQuality::Line,
                    vec![],
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    #[test]
    fn follows_first_line_before_playback_reaches_it() {
        let plan = frame_at(&document(), 500);
        assert_eq!(plan.focus_line, Some(0));
        assert_eq!(plan.active_line, None);
    }

    #[test]
    fn exposes_source_span_progress_at_utf8_boundaries() {
        let plan = frame_at(&document(), 1_500);
        let karaoke = plan.karaoke.unwrap();
        assert_eq!(plan.active_line, Some(0));
        assert_eq!(karaoke.completed_text_end, 0);
        assert_eq!(karaoke.active_text_range, Some(0..3));
        assert!((karaoke.active_span_progress - 0.5).abs() < f32::EPSILON);
        assert_eq!(karaoke.upcoming_text_range, None);
    }

    #[test]
    fn timing_gap_keeps_the_next_range_for_a_stationary_soft_edge() {
        let document = LyricsDocument::new(
            LyricsSource::Yrc,
            vec![
                LyricsLine::new(
                    0,
                    2_000,
                    "歌词",
                    None,
                    TimingQuality::Source,
                    vec![
                        TimedSpan::new(0..3, 0, 500),
                        TimedSpan::new(3..6, 700, 1_200),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let karaoke = frame_at(&document, 600).karaoke.unwrap();
        assert_eq!(karaoke.completed_text_end, 3);
        assert_eq!(karaoke.active_text_range, None);
        assert_eq!(karaoke.upcoming_text_range, Some(3..6));
    }

    #[test]
    fn keeps_focus_but_has_no_active_line_in_a_gap() {
        let plan = frame_at(&document(), 3_500);
        assert_eq!(plan.focus_line, Some(0));
        assert_eq!(plan.active_line, None);
        assert_eq!(plan.karaoke, None);
        assert_eq!(plan.interlude, None);
    }

    #[test]
    fn line_timing_never_produces_a_karaoke_plan() {
        let plan = frame_at(&document(), 4_500);
        assert_eq!(plan.active_line, Some(1));
        assert_eq!(plan.karaoke, None);
    }

    #[test]
    fn long_gap_has_one_renderer_independent_interlude_plan() {
        let document = LyricsDocument::new(
            LyricsSource::Lrc,
            vec![
                LyricsLine::new(1_000, 2_000, "first", None, TimingQuality::Line, vec![]).unwrap(),
                LyricsLine::new(7_000, 8_000, "second", None, TimingQuality::Line, vec![]).unwrap(),
            ],
        )
        .unwrap();

        let plan = frame_at(&document, 4_000).interlude.unwrap();
        assert_eq!(plan.after_line, Some(0));
        assert!(plan.alpha > 0.0);
        assert!(plan.scale > 0.0);
        assert!(plan.dot_alphas.iter().all(|alpha| *alpha >= 0.4));
    }

    #[test]
    fn opening_gap_and_short_gap_have_distinct_semantics() {
        let opening = LyricsDocument::new(
            LyricsSource::Lrc,
            vec![
                LyricsLine::new(3_000, 4_000, "first", None, TimingQuality::Line, vec![]).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(
            frame_at(&opening, 1_500).interlude.unwrap().after_line,
            None
        );

        let short = document();
        assert!(frame_at(&short, 3_500).interlude.is_none());
    }

    #[test]
    #[ignore = "manual performance baseline; run with --release and --nocapture"]
    fn perf_baseline_one_million_frames() {
        let mut lines = Vec::with_capacity(300);
        for index in 0..300_u64 {
            let start = index * 4_000;
            lines.push(
                LyricsLine::new(
                    start,
                    start + 3_000,
                    "一二三四五六",
                    Some(Arc::from("translation")),
                    TimingQuality::Source,
                    vec![
                        TimedSpan::new(0..3, start, start + 500),
                        TimedSpan::new(3..6, start + 500, start + 1_000),
                        TimedSpan::new(6..9, start + 1_000, start + 1_500),
                        TimedSpan::new(9..12, start + 1_500, start + 2_000),
                        TimedSpan::new(12..15, start + 2_000, start + 2_500),
                        TimedSpan::new(15..18, start + 2_500, start + 3_000),
                    ],
                )
                .unwrap(),
            );
        }
        let document = LyricsDocument::new(LyricsSource::Yrc, lines).unwrap();
        let started = std::time::Instant::now();
        for frame in 0..1_000_000_u64 {
            std::hint::black_box(frame_at(
                &document,
                (frame * 17) % document.lines.last().unwrap().end_ms,
            ));
        }
        let elapsed = started.elapsed();
        println!(
            "lyrics timeline: 1,000,000 frames / 300 lines / 1,800 spans in {:.3}s ({:.1} ns/frame)",
            elapsed.as_secs_f64(),
            elapsed.as_nanos() as f64 / 1_000_000.0
        );
    }
}
