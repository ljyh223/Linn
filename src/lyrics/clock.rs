use std::time::Duration;

const SOFT_CORRECTION_LIMIT_MS: f64 = 120.0;
const SOFT_CORRECTION_WINDOW_MS: f64 = 300.0;

/// A monotonic, renderer-independent playback clock.
///
/// GStreamer position messages are observations rather than animation ticks.
/// While playback is running, this clock extrapolates between observations.
/// Small timestamp jitter is absorbed over a short window; seeks and large
/// discontinuities are applied immediately.
#[derive(Debug, Clone)]
pub struct PlaybackClock {
    anchor_position_ms: f64,
    anchor_time: Duration,
    correction_ms: f64,
    running: bool,
    rate: f64,
    duration_ms: Option<u64>,
}

impl PlaybackClock {
    pub fn new(position_ms: u64, now: Duration) -> Self {
        Self {
            anchor_position_ms: position_ms as f64,
            anchor_time: now,
            correction_ms: 0.0,
            running: false,
            rate: 1.0,
            duration_ms: None,
        }
    }

    pub fn position_ms(&self, now: Duration) -> u64 {
        self.position_f64(now).round().max(0.0) as u64
    }

    pub fn observe(&mut self, position_ms: u64, now: Duration) {
        let predicted = self.position_f64(now);
        let error = position_ms as f64 - predicted;
        self.anchor_position_ms = predicted;
        self.anchor_time = now;
        self.correction_ms = if self.running && error.abs() <= SOFT_CORRECTION_LIMIT_MS {
            error
        } else {
            self.anchor_position_ms = position_ms as f64;
            0.0
        };
    }

    /// Apply a user or MPRIS seek without smoothing across the discontinuity.
    pub fn seek(&mut self, position_ms: u64, now: Duration) {
        self.anchor_position_ms = position_ms as f64;
        self.anchor_time = now;
        self.correction_ms = 0.0;
    }

    pub fn set_running(&mut self, running: bool, now: Duration) {
        if self.running == running {
            return;
        }
        self.anchor_position_ms = self.position_f64(now);
        self.anchor_time = now;
        self.correction_ms = 0.0;
        self.running = running;
    }

    pub fn set_rate(&mut self, rate: f64, now: Duration) {
        let rate = if rate.is_finite() {
            rate.clamp(0.0, 4.0)
        } else {
            1.0
        };
        self.anchor_position_ms = self.position_f64(now);
        self.anchor_time = now;
        self.correction_ms = 0.0;
        self.rate = rate;
    }

    pub fn set_duration(&mut self, duration_ms: Option<u64>) {
        self.duration_ms = duration_ms;
    }

    fn position_f64(&self, now: Duration) -> f64 {
        let elapsed_ms = now.saturating_sub(self.anchor_time).as_secs_f64() * 1_000.0;
        let playback_delta = if self.running {
            elapsed_ms * self.rate
        } else {
            0.0
        };
        let correction_progress = (elapsed_ms / SOFT_CORRECTION_WINDOW_MS).clamp(0.0, 1.0);
        let correction_ease =
            correction_progress * correction_progress * (3.0 - 2.0 * correction_progress);
        let position =
            self.anchor_position_ms + playback_delta + self.correction_ms * correction_ease;
        self.duration_ms
            .map_or(position, |duration| position.min(duration as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    #[test]
    fn interpolates_between_coarse_player_observations() {
        let mut clock = PlaybackClock::new(1_000, at(0));
        clock.set_running(true, at(0));
        assert_eq!(clock.position_ms(at(250)), 1_250);
        assert_eq!(clock.position_ms(at(750)), 1_750);
    }

    #[test]
    fn pauses_without_advancing() {
        let mut clock = PlaybackClock::new(1_000, at(0));
        clock.set_running(true, at(0));
        clock.set_running(false, at(500));
        assert_eq!(clock.position_ms(at(2_000)), 1_500);
    }

    #[test]
    fn seek_is_an_immediate_discontinuity() {
        let mut clock = PlaybackClock::new(1_000, at(0));
        clock.set_running(true, at(0));
        clock.seek(30_000, at(500));
        assert_eq!(clock.position_ms(at(500)), 30_000);
        assert_eq!(clock.position_ms(at(750)), 30_250);
    }

    #[test]
    fn small_observation_error_is_corrected_without_a_frame_jump() {
        let mut clock = PlaybackClock::new(1_000, at(0));
        clock.set_running(true, at(0));
        clock.observe(1_550, at(500));
        assert_eq!(clock.position_ms(at(500)), 1_500);
        assert_eq!(clock.position_ms(at(800)), 1_850);
    }

    #[test]
    fn large_observation_error_is_treated_as_a_seek() {
        let mut clock = PlaybackClock::new(1_000, at(0));
        clock.set_running(true, at(0));
        clock.observe(5_000, at(500));
        assert_eq!(clock.position_ms(at(500)), 5_000);
    }

    #[test]
    fn duration_clamps_extrapolation() {
        let mut clock = PlaybackClock::new(9_500, at(0));
        clock.set_duration(Some(10_000));
        clock.set_running(true, at(0));
        assert_eq!(clock.position_ms(at(2_000)), 10_000);
    }
}
