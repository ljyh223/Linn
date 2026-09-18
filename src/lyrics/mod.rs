//! Renderer-independent lyrics domain and timeline.
//!
//! This module deliberately has no GTK, Pango, GStreamer, or Relm4
//! dependencies. Parsers produce this model, the timeline turns it into a
//! frame plan, and rendering backends consume that plan.

pub mod clock;
pub mod config;
pub mod model;
pub mod parser;
pub mod presentation;
pub mod runtime;
pub mod timeline;
pub mod ttml;

pub use clock::PlaybackClock;
pub use config::LyricsV2Mode;
pub use model::{
    LyricModelError, LyricsDocument, LyricsLine, LyricsSource, TimedSpan, TimingQuality,
};
pub use parser::{RawLyrics, parse_document};
pub use presentation::LyricsPresentation;
pub use runtime::{LyricsRuntime, TimedFramePlan};
pub use timeline::{FramePlan, InterludePlan, KaraokePlan, frame_at};
