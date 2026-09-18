use std::{fmt, ops::Range, sync::Arc};

/// Where the selected lyrics came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsSource {
    Lrc,
    Yrc,
    Qrc,
    Ttml,
    Unknown,
}

/// How trustworthy the sub-line timing is.
///
/// `Line` means there is no sub-line timing at all. An estimated line may be
/// animated only when the presentation explicitly opts in; it must never be
/// presented as source-authored karaoke timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingQuality {
    Line,
    Estimated,
    Source,
}

/// A source timing interval associated with a UTF-8 byte range in a line.
///
/// The range may cover a character, syllable, word, or phrase. Keeping the
/// original range avoids inventing per-character timestamps when a provider
/// only supplies word-level timing. Pango also addresses text using UTF-8 byte
/// indexes, so this representation does not require a per-frame conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedSpan {
    pub text_range: Range<usize>,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl TimedSpan {
    pub fn new(text_range: Range<usize>, start_ms: u64, end_ms: u64) -> Self {
        Self {
            text_range,
            start_ms,
            end_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsLine {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: Arc<str>,
    pub translation: Option<Arc<str>>,
    pub timing_quality: TimingQuality,
    pub spans: Arc<[TimedSpan]>,
}

impl LyricsLine {
    pub fn new(
        start_ms: u64,
        end_ms: u64,
        text: impl Into<Arc<str>>,
        translation: Option<Arc<str>>,
        timing_quality: TimingQuality,
        spans: Vec<TimedSpan>,
    ) -> Result<Self, LyricModelError> {
        let line = Self {
            start_ms,
            end_ms,
            text: text.into(),
            translation,
            timing_quality,
            spans: spans.into(),
        };
        line.validate()?;
        Ok(line)
    }

    pub fn validate(&self) -> Result<(), LyricModelError> {
        if self.end_ms < self.start_ms {
            return Err(LyricModelError::LineEndsBeforeStart);
        }
        if self.timing_quality == TimingQuality::Line && !self.spans.is_empty() {
            return Err(LyricModelError::LineTimingHasSpans);
        }
        if self.timing_quality != TimingQuality::Line && self.spans.is_empty() {
            return Err(LyricModelError::SubLineTimingHasNoSpans);
        }

        let mut previous_text_end = 0;
        let mut previous_start = 0;
        for (index, span) in self.spans.iter().enumerate() {
            if span.text_range.start >= span.text_range.end
                || span.text_range.end > self.text.len()
                || !self.text.is_char_boundary(span.text_range.start)
                || !self.text.is_char_boundary(span.text_range.end)
            {
                return Err(LyricModelError::InvalidTextRange { index });
            }
            if span.end_ms <= span.start_ms {
                return Err(LyricModelError::EmptyTimedSpan { index });
            }
            if span.start_ms < self.start_ms || span.end_ms > self.end_ms {
                return Err(LyricModelError::SpanOutsideLine { index });
            }
            if index > 0
                && (span.text_range.start < previous_text_end || span.start_ms < previous_start)
            {
                return Err(LyricModelError::UnorderedSpans { index });
            }
            previous_text_end = span.text_range.end;
            previous_start = span.start_ms;
        }
        Ok(())
    }

    pub fn has_source_timing(&self) -> bool {
        self.timing_quality == TimingQuality::Source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsDocument {
    pub source: LyricsSource,
    pub lines: Arc<[LyricsLine]>,
}

impl LyricsDocument {
    pub fn new(source: LyricsSource, lines: Vec<LyricsLine>) -> Result<Self, LyricModelError> {
        for (index, line) in lines.iter().enumerate() {
            line.validate()?;
            if index > 0 && line.start_ms < lines[index - 1].start_ms {
                return Err(LyricModelError::UnorderedLines { index });
            }
        }
        Ok(Self {
            source,
            lines: lines.into(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricModelError {
    LineEndsBeforeStart,
    LineTimingHasSpans,
    SubLineTimingHasNoSpans,
    InvalidTextRange { index: usize },
    EmptyTimedSpan { index: usize },
    SpanOutsideLine { index: usize },
    UnorderedSpans { index: usize },
    UnorderedLines { index: usize },
}

impl fmt::Display for LyricModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineEndsBeforeStart => formatter.write_str("lyric line ends before it starts"),
            Self::LineTimingHasSpans => {
                formatter.write_str("line-only timing must not contain timed spans")
            }
            Self::SubLineTimingHasNoSpans => {
                formatter.write_str("sub-line timing must contain at least one timed span")
            }
            Self::InvalidTextRange { index } => {
                write!(
                    formatter,
                    "timed span {index} has an invalid UTF-8 text range"
                )
            }
            Self::EmptyTimedSpan { index } => {
                write!(formatter, "timed span {index} has no positive duration")
            }
            Self::SpanOutsideLine { index } => {
                write!(formatter, "timed span {index} is outside its lyric line")
            }
            Self::UnorderedSpans { index } => {
                write!(
                    formatter,
                    "timed span {index} is out of order or overlaps text"
                )
            }
            Self::UnorderedLines { index } => {
                write!(
                    formatter,
                    "lyric line {index} is out of chronological order"
                )
            }
        }
    }
}

impl std::error::Error for LyricModelError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_utf8_source_spans_without_splitting_them() {
        let text: Arc<str> = Arc::from("Hello 世界");
        let line = LyricsLine::new(
            1_000,
            3_000,
            text.clone(),
            None,
            TimingQuality::Source,
            vec![
                TimedSpan::new(0..6, 1_000, 2_000),
                TimedSpan::new(6..text.len(), 2_000, 3_000),
            ],
        )
        .unwrap();

        assert_eq!(&line.text[line.spans[1].text_range.clone()], "世界");
        assert_eq!(line.spans.len(), 2);
        assert!(line.has_source_timing());
    }

    #[test]
    fn rejects_fake_zero_duration_karaoke() {
        let error = LyricsLine::new(
            1_000,
            2_000,
            "歌词",
            None,
            TimingQuality::Source,
            vec![TimedSpan::new(0..3, 1_000, 1_000)],
        )
        .unwrap_err();

        assert_eq!(error, LyricModelError::EmptyTimedSpan { index: 0 });
    }

    #[test]
    fn line_timing_cannot_masquerade_as_karaoke() {
        let error = LyricsLine::new(
            1_000,
            2_000,
            "line",
            None,
            TimingQuality::Line,
            vec![TimedSpan::new(0..4, 1_000, 2_000)],
        )
        .unwrap_err();

        assert_eq!(error, LyricModelError::LineTimingHasSpans);
    }
}
