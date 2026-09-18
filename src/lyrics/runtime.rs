use std::{sync::Arc, time::Duration};

use super::{FramePlan, LyricsDocument, PlaybackClock, frame_at};

/// A frame plan paired with the exact interpolated position used to build it.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedFramePlan {
    pub position_ms: u64,
    pub frame: FramePlan,
}

/// Owns the renderer-independent live state for one lyric surface.
///
/// Both reference and accelerated backends consume this runtime rather than
/// maintaining their own active-line or playback-time state.
#[derive(Debug, Clone)]
pub struct LyricsRuntime {
    document: Option<Arc<LyricsDocument>>,
    clock: PlaybackClock,
}

impl LyricsRuntime {
    pub fn new(now: Duration) -> Self {
        Self {
            document: None,
            clock: PlaybackClock::new(0, now),
        }
    }

    pub fn load(&mut self, document: Arc<LyricsDocument>) {
        self.document = Some(document);
    }

    pub fn clear(&mut self) {
        self.document = None;
    }

    pub fn document(&self) -> Option<&LyricsDocument> {
        self.document.as_deref()
    }

    pub fn observe_position(&mut self, position_ms: u64, duration_ms: Option<u64>, now: Duration) {
        self.clock.set_duration(duration_ms);
        self.clock.observe(position_ms, now);
    }

    pub fn set_playing(&mut self, playing: bool, now: Duration) {
        self.clock.set_running(playing, now);
    }

    pub fn seek(&mut self, position_ms: u64, now: Duration) {
        self.clock.seek(position_ms, now);
    }

    pub fn frame(&self, now: Duration) -> Option<TimedFramePlan> {
        let document = self.document.as_deref()?;
        let position_ms = self.clock.position_ms(now);
        Some(TimedFramePlan {
            position_ms,
            frame: frame_at(document, position_ms),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{LyricsLine, LyricsSource, TimingQuality};

    fn at(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    fn document() -> Arc<LyricsDocument> {
        Arc::new(
            LyricsDocument::new(
                LyricsSource::Lrc,
                vec![
                    LyricsLine::new(1_000, 2_000, "one", None, TimingQuality::Line, vec![])
                        .unwrap(),
                    LyricsLine::new(2_000, 3_000, "two", None, TimingQuality::Line, vec![])
                        .unwrap(),
                ],
            )
            .unwrap(),
        )
    }

    #[test]
    fn no_document_produces_no_frame() {
        assert_eq!(LyricsRuntime::new(at(0)).frame(at(0)), None);
    }

    #[test]
    fn one_clock_drives_document_focus_and_activity() {
        let mut runtime = LyricsRuntime::new(at(0));
        runtime.load(document());
        runtime.observe_position(1_000, Some(3_000), at(0));
        runtime.set_playing(true, at(0));

        let frame = runtime.frame(at(1_250)).unwrap();
        assert_eq!(frame.position_ms, 2_250);
        assert_eq!(frame.frame.focus_line, Some(1));
        assert_eq!(frame.frame.active_line, Some(1));
    }

    #[test]
    fn seek_updates_the_next_frame_immediately() {
        let mut runtime = LyricsRuntime::new(at(0));
        runtime.load(document());
        runtime.set_playing(true, at(0));
        runtime.seek(2_500, at(100));

        let frame = runtime.frame(at(100)).unwrap();
        assert_eq!(frame.position_ms, 2_500);
        assert_eq!(frame.frame.active_line, Some(1));
    }

    #[test]
    fn clearing_releases_document_from_frame_path() {
        let mut runtime = LyricsRuntime::new(at(0));
        runtime.load(document());
        runtime.clear();
        assert_eq!(runtime.frame(at(0)), None);
    }
}
