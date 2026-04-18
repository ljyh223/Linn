use gst::ClockTime;
use gst_play::{Play, PlayMessage, PlayState};

use super::messages::{PlaybackState, PlayerEvent};

/// GstEngine 只管 GStreamer，事件通过回调向上汇报。
pub(crate) struct GstEngine {
    play: Play,
    pub is_playing: bool,
}

impl GstEngine {
    pub fn new() -> Self {
        Self { play: Play::default(), is_playing: false }
    }

    pub fn play_url(&mut self, url: &str) {
        self.play.set_uri(Some(url));
        self.play.play();
        self.is_playing = true;
    }

    pub fn toggle(&mut self) {
        if self.is_playing {
            self.play.pause();
            self.is_playing = false;
        } else {
            self.play.play();
            self.is_playing = true;
        }
    }

    pub fn resume(&mut self) {
        self.play.play();
        self.is_playing = true;
    }

    pub fn pause(&mut self) {
        self.play.pause();
        self.is_playing = false;
    }

    pub fn seek(&mut self, offset_ms: u64) {
        self.play.seek(ClockTime::from_mseconds(offset_ms));
    }

    pub fn duration_ms(&self) -> u64 {
        self.play.duration().map_or(0, |d| d.mseconds())
    }

    /// 非阻塞轮询消息总线，最多等 10ms。
    /// 返回解析好的 PlayerEvent，调用者决定怎么处理。
    pub fn poll(&self) -> Option<GstEvent> {
        let msg = self.play.message_bus().timed_pop(ClockTime::from_mseconds(10))?;
        let play_msg = PlayMessage::parse(&msg).ok()?;
        match play_msg {
            PlayMessage::StateChanged(s) => {
                let state = match s.state() {
                    PlayState::Playing  => PlaybackState::Playing,
                    PlayState::Paused   => PlaybackState::Paused,
                    PlayState::Stopped  => PlaybackState::Stopped,
                    _                   => return None,
                };
                Some(GstEvent::State(state))
            }
            PlayMessage::EndOfStream(_) => Some(GstEvent::EndOfStream),
            PlayMessage::PositionUpdated(p) => {
                let pos = p.position().unwrap_or_default().mseconds();
                let dur = self.duration_ms();
                Some(GstEvent::Position { position: pos, duration: dur })
            }
            PlayMessage::Error(e) => Some(GstEvent::Error(e.details().unwrap().to_string())),
            _ => None,
        }
    }
}

/// GstEngine 向上汇报的事件（中间层，不直接等于 PlayerEvent）
pub(crate) enum GstEvent {
    State(PlaybackState),
    EndOfStream,
    Position { position: u64, duration: u64 },
    Error(String),
}