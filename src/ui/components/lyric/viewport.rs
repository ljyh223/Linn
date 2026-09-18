//! Renderer-neutral viewport animation shared by lyric drawing backends.

use std::time::{Duration, Instant};

use crate::lyrics::TimedFramePlan;

const FOCUS_RATIO: f64 = 0.34;
const SCROLL_RESPONSE: f64 = 7.5;
const FOCUS_TRANSITION_DURATION: Duration = Duration::from_millis(520);
const MANUAL_RETURN_DELAY: Duration = Duration::from_millis(800);
const MANUAL_RETURN_RESPONSE: f64 = 8.0;
const INTERLUDE_PUSH_HEIGHT: f64 = 44.0;
const INTERLUDE_PUSH_ATTACK: f64 = 50.0;
const INTERLUDE_PUSH_RELEASE: f64 = 7.0;
const SEEK_DISCONTINUITY_MS: u64 = 1_500;
const CLICK_SEEK_TRAVEL_VIEWPORTS: f64 = 0.9;

#[derive(Clone, Copy)]
enum InterludeAnchor {
    BeforeFirst,
    After(usize),
}

pub(crate) struct LyricViewport {
    frame: Option<TimedFramePlan>,
    scroll_y: f64,
    target_scroll_y: f64,
    initialized: bool,
    last_tick: Option<Instant>,
    last_position_ms: Option<u64>,
    last_focus_line: Option<usize>,
    previous_focus_line: Option<usize>,
    focus_transition: f64,
    manual_offset: f64,
    manual_until: Option<Instant>,
    dragging: bool,
    last_drag_offset: f64,
    interlude_anchor: Option<InterludeAnchor>,
    interlude_push: f64,
    animate_next_seek: bool,
}

impl LyricViewport {
    pub(crate) fn new() -> Self {
        Self {
            frame: None,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
            initialized: false,
            last_tick: None,
            last_position_ms: None,
            last_focus_line: None,
            previous_focus_line: None,
            focus_transition: 1.0,
            manual_offset: 0.0,
            manual_until: None,
            dragging: false,
            last_drag_offset: 0.0,
            interlude_anchor: None,
            interlude_push: 0.0,
            animate_next_seek: false,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.frame = None;
        self.scroll_y = 0.0;
        self.target_scroll_y = 0.0;
        self.initialized = false;
        self.last_tick = None;
        self.last_position_ms = None;
        self.last_focus_line = None;
        self.previous_focus_line = None;
        self.focus_transition = 1.0;
        self.manual_offset = 0.0;
        self.manual_until = None;
        self.dragging = false;
        self.last_drag_offset = 0.0;
        self.interlude_anchor = None;
        self.interlude_push = 0.0;
        self.animate_next_seek = false;
    }

    pub(crate) fn invalidate_layout(&mut self) {
        self.initialized = false;
    }

    pub(crate) fn frame(&self) -> Option<&TimedFramePlan> {
        self.frame.as_ref()
    }

    pub(crate) fn scroll_y(&self) -> f64 {
        // A negative automatic position is intentional for the first lines:
        // it places their centre at the same visual focus as the rest of the
        // song. Manual interaction may not drag beyond the lower of the
        // current/target automatic positions (or zero later in the song).
        let minimum = self.scroll_y.min(self.target_scroll_y).min(0.0);
        (self.scroll_y + self.manual_offset).max(minimum)
    }

    pub(crate) fn line_offset(&self, index: usize) -> f64 {
        let follows_gap = match self.interlude_anchor {
            Some(InterludeAnchor::BeforeFirst) => true,
            Some(InterludeAnchor::After(after)) => index > after,
            None => false,
        };
        if follows_gap {
            self.interlude_push
        } else {
            0.0
        }
    }

    pub(crate) fn interlude_push(&self) -> f64 {
        self.interlude_push
    }

    /// Visual focus amount used by both renderers for line-level scale and
    /// opacity transitions. It is deliberately independent of lyric timing:
    /// changing focus must not rebuild layouts or cached textures.
    pub(crate) fn line_focus_mix(&self, index: usize) -> f64 {
        let eased = ease_out_cubic(self.focus_transition);
        if self.last_focus_line == Some(index) {
            eased
        } else if self.previous_focus_line == Some(index) {
            1.0 - eased
        } else {
            0.0
        }
    }

    pub(crate) fn begin_drag(&mut self) {
        self.dragging = true;
        self.last_drag_offset = 0.0;
        self.manual_until = None;
    }

    pub(crate) fn update_drag(&mut self, cumulative_y: f64) {
        if !self.dragging {
            return;
        }
        let delta = cumulative_y - self.last_drag_offset;
        self.manual_offset -= delta;
        self.last_drag_offset = cumulative_y;
    }

    pub(crate) fn end_drag(&mut self, now: Instant) {
        self.dragging = false;
        self.last_drag_offset = 0.0;
        self.manual_until = Some(now + MANUAL_RETURN_DELAY);
    }

    pub(crate) fn nudge_scroll(&mut self, delta_y: f64, now: Instant) {
        self.manual_offset += delta_y;
        self.manual_until = Some(now + MANUAL_RETURN_DELAY);
    }

    pub(crate) fn prepare_animated_seek(&mut self) {
        self.animate_next_seek = true;
        self.manual_offset = 0.0;
        self.manual_until = None;
        self.dragging = false;
        self.last_drag_offset = 0.0;
    }

    pub(crate) fn initialize(&mut self, focus_center_y: Option<f64>, viewport_height: i32) {
        if self.initialized {
            return;
        }
        if let Some(focus_center_y) = focus_center_y {
            self.target_scroll_y = target_scroll(focus_center_y, viewport_height);
            self.scroll_y = self.target_scroll_y;
        }
        self.initialized = true;
    }

    pub(crate) fn advance(
        &mut self,
        frame: Option<TimedFramePlan>,
        focus_center_y: Option<f64>,
        now: Instant,
        viewport_height: i32,
    ) -> bool {
        let dt = self
            .last_tick
            .map(|last| now.saturating_duration_since(last).as_secs_f64())
            .unwrap_or(1.0 / 60.0)
            .min(0.1);
        self.last_tick = Some(now);

        let has_interlude = frame
            .as_ref()
            .and_then(|frame| frame.frame.interlude.as_ref())
            .is_some();
        if let Some(interlude) = frame
            .as_ref()
            .and_then(|frame| frame.frame.interlude.as_ref())
        {
            self.interlude_anchor = Some(match interlude.after_line {
                Some(index) => InterludeAnchor::After(index),
                None => InterludeAnchor::BeforeFirst,
            });
        }
        let push_target = if has_interlude {
            INTERLUDE_PUSH_HEIGHT
        } else {
            0.0
        };
        let push_speed = if push_target > self.interlude_push {
            INTERLUDE_PUSH_ATTACK
        } else {
            INTERLUDE_PUSH_RELEASE
        };
        let push_blend = 1.0 - (-push_speed * dt).exp();
        let previous_push = self.interlude_push;
        self.interlude_push += (push_target - self.interlude_push) * push_blend;
        if (self.interlude_push - push_target).abs() <= 0.05 {
            self.interlude_push = push_target;
            if !has_interlude {
                self.interlude_anchor = None;
            }
        }
        let push_changed = (self.interlude_push - previous_push).abs() > 0.01;

        let position = frame.as_ref().map(|frame| frame.position_ms);
        let position_changed = position != self.last_position_ms;
        let animate_seek = self.animate_next_seek && position_changed;
        if animate_seek {
            self.animate_next_seek = false;
        }
        let position_discontinuity = self
            .last_position_ms
            .zip(position)
            .is_some_and(|(previous, current)| previous.abs_diff(current) > SEEK_DISCONTINUITY_MS);
        let focus = frame.as_ref().and_then(|frame| frame.frame.focus_line);
        let focus_changed = focus != self.last_focus_line;
        if focus_changed {
            self.previous_focus_line = self.last_focus_line;
            self.focus_transition = if self.last_focus_line.is_some()
                && focus.is_some()
                && (!position_discontinuity || animate_seek)
            {
                0.0
            } else {
                1.0
            };
        }
        self.last_position_ms = position;
        self.last_focus_line = focus;
        self.frame = frame;

        let previous_focus_transition = self.focus_transition;
        if self.focus_transition < 1.0 {
            self.focus_transition =
                (self.focus_transition + dt / FOCUS_TRANSITION_DURATION.as_secs_f64()).min(1.0);
            if self.focus_transition >= 1.0 {
                self.previous_focus_line = None;
            }
        }
        let focus_transition_changed =
            (self.focus_transition - previous_focus_transition).abs() > f64::EPSILON;

        if position_discontinuity {
            self.manual_offset = 0.0;
            self.manual_until = None;
            self.dragging = false;
            self.last_drag_offset = 0.0;
            self.interlude_push = push_target;
            if !has_interlude {
                self.interlude_anchor = None;
            }
        }

        if let Some(mut focus_center_y) = focus_center_y {
            if let Some(focus_index) = focus {
                focus_center_y += self.line_offset(focus_index);
            }
            self.target_scroll_y = target_scroll(focus_center_y, viewport_height);
            if !self.initialized || position_discontinuity && !animate_seek {
                self.scroll_y = self.target_scroll_y;
                self.initialized = true;
            } else if animate_seek {
                // A click should visibly travel in the requested direction,
                // but scrolling through dozens of intermediate lines is
                // disorienting. For distant targets, begin one viewport away
                // and animate the final approach.
                let maximum_travel = viewport_height.max(1) as f64 * CLICK_SEEK_TRAVEL_VIEWPORTS;
                let distance = self.target_scroll_y - self.scroll_y;
                if distance.abs() > maximum_travel {
                    self.scroll_y = self.target_scroll_y - distance.signum() * maximum_travel;
                }
            }
        }

        let distance = self.target_scroll_y - self.scroll_y;
        let scroll_changed = distance.abs() > 0.05;
        if scroll_changed {
            let blend = 1.0 - (-SCROLL_RESPONSE * dt).exp();
            self.scroll_y += distance * blend;
            if (self.target_scroll_y - self.scroll_y).abs() <= 0.05 {
                self.scroll_y = self.target_scroll_y;
            }
        }

        let manual_changed = if !self.dragging
            && self.manual_until.is_some_and(|deadline| now >= deadline)
            && self.manual_offset.abs() > 0.05
        {
            let blend = 1.0 - (-MANUAL_RETURN_RESPONSE * dt).exp();
            self.manual_offset *= 1.0 - blend;
            if self.manual_offset.abs() <= 0.05 {
                self.manual_offset = 0.0;
                self.manual_until = None;
            }
            true
        } else {
            false
        };

        position_changed
            || focus_changed
            || focus_transition_changed
            || scroll_changed
            || manual_changed
            || push_changed
    }
}

fn ease_out_cubic(value: f64) -> f64 {
    1.0 - (1.0 - value.clamp(0.0, 1.0)).powi(3)
}

fn target_scroll(focus_center_y: f64, viewport_height: i32) -> f64 {
    focus_center_y - viewport_height.max(1) as f64 * FOCUS_RATIO
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{FramePlan, InterludePlan};

    fn frame(position_ms: u64, focus_line: usize) -> TimedFramePlan {
        TimedFramePlan {
            position_ms,
            frame: FramePlan {
                focus_line: Some(focus_line),
                active_line: Some(focus_line),
                line_progress: 0.0,
                karaoke: None,
                interlude: None,
            },
        }
    }

    #[test]
    fn initial_focus_snaps_without_scrolling_from_the_previous_song() {
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 2)), Some(500.0), Instant::now(), 400);
        assert!((viewport.scroll_y() - 364.0).abs() < 0.001);
    }

    #[test]
    fn focus_change_uses_smooth_scroll_target() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 0)), Some(100.0), now, 400);
        assert_eq!(viewport.scroll_y(), -36.0);
        viewport.advance(
            Some(frame(2_000, 1)),
            Some(500.0),
            now + std::time::Duration::from_millis(16),
            400,
        );
        assert!(viewport.scroll_y() > -36.0);
        assert!(viewport.scroll_y() < 364.0);
        assert!(viewport.line_focus_mix(0) > 0.0);
        assert!(viewport.line_focus_mix(1) > 0.0);
        assert!(viewport.line_focus_mix(1) < 1.0);
    }

    #[test]
    fn focus_crossfade_finishes_without_changing_layout_state() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 0)), Some(100.0), now, 400);
        viewport.advance(
            Some(frame(1_100, 1)),
            Some(180.0),
            now + Duration::from_millis(16),
            400,
        );
        assert!(viewport.line_focus_mix(0) > 0.0);
        assert!(viewport.line_focus_mix(1) > 0.0);

        for step in 2..=15 {
            viewport.advance(
                Some(frame(1_100 + step * 40, 1)),
                Some(180.0),
                now + Duration::from_millis(16 + step * 40),
                400,
            );
        }
        assert_eq!(viewport.line_focus_mix(0), 0.0);
        assert_eq!(viewport.line_focus_mix(1), 1.0);
    }

    #[test]
    fn large_external_position_jump_clears_manual_offset_and_snaps() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 2)), Some(500.0), now, 400);
        viewport.nudge_scroll(80.0, now);
        assert!((viewport.scroll_y() - 444.0).abs() < 0.001);

        viewport.advance(
            Some(frame(10_000, 8)),
            Some(1_200.0),
            now + Duration::from_millis(16),
            400,
        );

        assert!(
            (viewport.scroll_y() - 1_064.0).abs() < 0.001,
            "MPRIS-style jumps must not retain a manual scroll or fly from the old line"
        );
    }

    #[test]
    fn lyric_click_animates_the_final_viewport_instead_of_snapping() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 0)), Some(100.0), now, 400);
        viewport.prepare_animated_seek();
        viewport.advance(
            Some(frame(30_000, 20)),
            Some(2_000.0),
            now + Duration::from_millis(16),
            400,
        );

        let target = target_scroll(2_000.0, 400);
        assert!(viewport.scroll_y() < target);
        assert!(target - viewport.scroll_y() < 400.0);
    }

    #[test]
    fn drag_follows_pointer_then_returns_to_timeline_focus() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 2)), Some(500.0), now, 400);
        let focused = viewport.scroll_y();

        viewport.begin_drag();
        viewport.update_drag(-80.0);
        assert_eq!(viewport.scroll_y(), focused + 80.0);
        viewport.end_drag(now);
        viewport.advance(
            Some(frame(1_000, 2)),
            Some(500.0),
            now + MANUAL_RETURN_DELAY + Duration::from_millis(16),
            400,
        );
        assert!(viewport.scroll_y() < focused + 80.0);
        assert!(viewport.scroll_y() > focused);
    }

    #[test]
    fn wheel_offset_waits_before_returning() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(1_000, 0)), Some(100.0), now, 400);
        viewport.nudge_scroll(40.0, now);
        viewport.advance(
            Some(frame(1_000, 0)),
            Some(100.0),
            now + Duration::from_millis(400),
            400,
        );
        assert_eq!(viewport.scroll_y(), 4.0);
    }

    #[test]
    fn first_line_reaches_the_visual_focus_without_unbounded_top_overscroll() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        viewport.advance(Some(frame(750, 0)), Some(60.0), now, 600);

        assert!((viewport.scroll_y() + 144.0).abs() < 0.001);
        assert!((60.0 - viewport.scroll_y() - 600.0 * FOCUS_RATIO).abs() < 0.001);

        viewport.nudge_scroll(-200.0, now);
        assert!(
            (viewport.scroll_y() + 144.0).abs() < 0.001,
            "manual scrolling must not pull the first line below its designed focus"
        );
    }

    #[test]
    fn interlude_pushes_only_following_lines_and_releases_smoothly() {
        let now = Instant::now();
        let mut viewport = LyricViewport::new();
        let mut during_gap = frame(2_500, 0);
        during_gap.frame.interlude = Some(InterludePlan {
            after_line: Some(0),
            alpha: 1.0,
            scale: 1.0,
            reveal: 1.0,
            dot_alphas: [1.0; 3],
        });
        viewport.advance(Some(during_gap), Some(100.0), now, 400);

        assert_eq!(viewport.line_offset(0), 0.0);
        assert!(viewport.line_offset(1) > 0.0);
        let pushed = viewport.line_offset(1);

        viewport.advance(
            // A normal next frame leaves the interlude naturally, so the
            // layout push releases smoothly. Large jumps are covered by the
            // seek-discontinuity test and intentionally snap instead.
            Some(frame(2_516, 1)),
            Some(200.0),
            now + Duration::from_millis(16),
            400,
        );
        assert!(viewport.line_offset(1) < pushed);
        assert!(viewport.line_offset(1) > 0.0);
    }
}
