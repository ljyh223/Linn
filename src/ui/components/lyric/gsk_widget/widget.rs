use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use relm4::gtk::glib::{self, Object, subclass::types::ObjectSubclassIsExt};
use relm4::gtk::{self, Accessible, Buildable, ConstraintTarget, Widget, prelude::*};
use pangocairo::pango;

use super::imp::LyricWidgetImp;
use crate::ui::components::lyric::lyric_widget::{
    LyricsWidgetState, LyricAlign, SCROLL_FRICTION,
};
use crate::ui::model::LyricLine;

glib::wrapper! {
    pub struct LyricWidget(ObjectSubclass<LyricWidgetImp>)
        @extends Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl LyricWidget {
    pub fn new(on_seek: impl Fn(u64) + 'static) -> Self {
        let obj: Self = Object::builder().build();
        let state = Rc::new(RefCell::new(LyricsWidgetState::new()));
        obj.imp().state.replace(state);
        obj.imp().on_seek_cb.replace(Some(Box::new(on_seek)));

        obj.setup_tick_callback();
        obj.setup_gestures();

        obj
    }

    pub fn state(&self) -> Rc<RefCell<LyricsWidgetState>> {
        self.imp().state()
    }

    fn setup_tick_callback(&self) {
        let state = self.imp().state();
        let obj = self.clone();

        let id = obj.add_tick_callback(move |widget, _frame_clock| {
            let mut st = state.borrow_mut();
            let now = Instant::now();
            let dt = st.last_frame_time
                .map(|t| now.duration_since(t).as_secs_f64())
                .unwrap_or(0.016)
                .min(0.1);
            st.last_frame_time = Some(now);

            if st.is_decelerating {
                let new_pos = st.scroll_spring.current_position + st.scroll_velocity * dt;
                st.scroll_spring.snap_to(new_pos);
                st.scroll_spring.set_target(new_pos);
                let friction = SCROLL_FRICTION.powf(dt / 0.016);
                st.scroll_velocity *= friction;
                if st.scroll_velocity.abs() < 5.0 {
                    st.is_decelerating = false;
                    st.scroll_velocity = 0.0;
                    st.user_scrolling = false;
                }
            }

            if !st.user_scrolling && !st.is_decelerating {
                st.update_line_states();
                let raw = st.active_line_index();
                if st.needs_initial_scroll || raw != st.last_raw_active_idx {
                    st.needs_initial_scroll = false;
                    st.last_raw_active_idx = raw;
                    let h = widget.height() as f64;
                    if let Some(idx) = raw {
                        st.update_scroll_target(h, idx);
                    } else if st.interlude_dots.visible {
                        st.update_scroll_for_interlude(h);
                    }
                }
            }

            st.tick_springs(dt);

            if st.active_line_index().is_some() || !st.cached_lines.is_empty() {
                widget.queue_draw();
            }

            glib::ControlFlow::Continue
        });
        self.imp().tick_id.replace(Some(id));
    }

    fn setup_gestures(&self) {
        let state = self.imp().state();
        let obj = self.clone();

        // Click to seek
        let gesture = gtk::GestureClick::new();
        let state_click = state.clone();
        let obj_click = obj.clone();
        gesture.connect_pressed(move |_, _, _x, click_y| {
            let st = state_click.borrow();
            if let Some(idx) = st.line_at_y(click_y) {
                let target_ms = st.cached_lines[idx].line.start;
                drop(st);
                if let Some(cb) = obj_click.imp().on_seek_cb.borrow().as_ref() {
                    cb(target_ms);
                }
                let mut st = state_click.borrow_mut();
                if let Some(old_idx) = st.last_active_idx {
                    if old_idx < st.line_states.len() {
                        st.line_states[old_idx].set_active(false);
                    }
                }
                if idx < st.line_states.len() {
                    st.line_states[idx].set_active(true);
                }
                st.current_ms = target_ms;
                st.last_active_idx = Some(idx);
                st.last_raw_active_idx = Some(idx);
                st.user_scrolling = false;
            }
        });
        self.add_controller(gesture);

        // Drag to scroll
        let drag_gesture = gtk::GestureDrag::new();
        let state_drag = state.clone();
        drag_gesture.connect_drag_begin(move |_, _, _| {
            let mut st = state_drag.borrow_mut();
            st.user_scrolling = true;
            st.is_decelerating = false;
            st.scroll_velocity = 0.0;
            st.last_drag_offset = 0.0;
            st.last_drag_time = None;
            st.drag_start_scroll = st.scroll_spring.current_position;
        });
        let state_drag_update = state.clone();
        drag_gesture.connect_drag_update(move |_, _offset_x, offset_y| {
            let mut st = state_drag_update.borrow_mut();
            let new_scroll = st.drag_start_scroll - offset_y;
            st.scroll_spring.snap_to(new_scroll);
            st.scroll_spring.set_target(new_scroll);
            let now = Instant::now();
            if let Some(last_time) = st.last_drag_time {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.001 {
                    let delta = -(offset_y - st.last_drag_offset);
                    st.scroll_velocity = (delta / dt).clamp(-8000.0, 8000.0);
                }
            }
            st.last_drag_offset = offset_y;
            st.last_drag_time = Some(now);
        });
        let state_drag_end = state.clone();
        drag_gesture.connect_drag_end(move |_, _, _| {
            let mut st = state_drag_end.borrow_mut();
            if st.scroll_velocity.abs() > 80.0 {
                st.is_decelerating = true;
            } else {
                st.user_scrolling = false;
                st.scroll_velocity = 0.0;
            }
        });
        self.add_controller(drag_gesture);

        // Scroll wheel
        let scroll_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let state_scroll = state.clone();
        scroll_controller.connect_scroll(move |_, _, dy| {
            let mut st = state_scroll.borrow_mut();
            let current = st.scroll_spring.current_position;
            let delta = dy * 40.0;
            st.scroll_spring.snap_to(current + delta);
            st.scroll_spring.set_target(current + delta);
            st.user_scrolling = true;
            st.is_decelerating = false;
            st.scroll_velocity = 0.0;
            let state_clone = state_scroll.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                let mut st = state_clone.borrow_mut();
                if !st.is_decelerating {
                    st.user_scrolling = false;
                }
            });
            gtk::glib::Propagation::Stop
        });
        self.add_controller(scroll_controller);
    }

    pub fn load_lines(&self, lines: Vec<LyricLine>, available_width: i32) {
        let pango_ctx = self.pango_context();
        self.state().borrow_mut().load_lines(lines, &pango_ctx, available_width);
        self.queue_draw();
    }

    pub fn update_time(&self, ms: u64) {
        self.state().borrow_mut().update_time(ms);
        self.queue_draw();
    }

    pub fn set_text_color(&self, r: f64, g: f64, b: f64, a: f64) {
        let state = self.state();
        let mut st = state.borrow_mut();
        st.set_text_color(r, g, b, a);
        st.enable_shadow = true;
        drop(st);
        self.queue_draw();
    }

    pub fn set_bg_color(&self, r: f64, g: f64, b: f64) {
        self.state().borrow_mut().set_bg_color(r, g, b);
        self.queue_draw();
    }

    pub fn set_align(&self, align: LyricAlign) {
        self.state().borrow_mut().set_align(align);
        self.queue_draw();
    }
}