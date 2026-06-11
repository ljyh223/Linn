use std::cell::RefCell;
use std::rc::Rc;

use relm4::gtk::glib::{
    self, ParamSpec, Properties, Value,
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
    },
};
use relm4::gtk::{
    self, Widget,
    gdk, gsk, graphene,
    prelude::{ObjectExt, SnapshotExt, WidgetExt},
    subclass::widget::WidgetImpl,
};

use crate::ui::model::LyricLineKind;
use crate::ui::components::lyric::lyric_widget::{
    LyricsWidgetState, LyricAlign, CachedLine,
    ALPHA_ACTIVE, ALPHA_DIM, GRADIENT_EDGE_PX, TL_GAP,
    TOP_PADDING, FADE_HEIGHT,
    x_for_layout, dim_color, fade_alpha_for_y, ease_in_out_cubic,
};
use crate::ui::components::lyric::interlude_dots::InterludeDots;
use crate::ui::components::lyric::spring::Spring;

use std::f64::consts::TAU;

const DOT_RADIUS: f64 = 4.0;
const DOT_SPACING: f64 = 12.0;
const DOT_LEFT_MARGIN: f64 = 28.0;

#[derive(Default, Properties)]
#[properties(wrapper_type = super::widget::LyricWidget)]
pub struct LyricWidgetImp {
    #[property(get, set)]
    pub current_ms: RefCell<u64>,
    pub state: RefCell<Rc<RefCell<LyricsWidgetState>>>,
    pub on_seek_cb: RefCell<Option<Box<dyn Fn(u64)>>>,
    pub tick_id: RefCell<Option<gtk::TickCallbackId>>,
}

impl LyricWidgetImp {
    pub fn state(&self) -> Rc<RefCell<LyricsWidgetState>> {
        self.state.borrow().clone()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for LyricWidgetImp {
    const NAME: &'static str = "LyricWidget";
    type Type = super::widget::LyricWidget;
    type ParentType = Widget;
}

impl ObjectImpl for LyricWidgetImp {
    fn properties() -> &'static [ParamSpec] {
        Self::derived_properties()
    }
    fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
        self.derived_set_property(id, value, pspec)
    }
    fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
        self.derived_property(id, pspec)
    }

    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        obj.set_hexpand(true);
        obj.set_vexpand(true);
        obj.set_can_target(true);
        obj.set_overflow(gtk::Overflow::Hidden);
    }

    fn dispose(&self) {
        if let Some(id) = self.tick_id.borrow_mut().take() {
            id.remove();
        }
    }
}

impl WidgetImpl for LyricWidgetImp {
    fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        match orientation {
            gtk::Orientation::Horizontal => (0, 400, -1, -1),
            gtk::Orientation::Vertical => (0, 600, -1, -1),
            _ => (0, 0, -1, -1),
        }
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let w = obj.width() as f64;
        let h = obj.height() as f64;
        if w <= 0.0 || h <= 0.0 { return; }

        let state_rc = self.state();
        let st = state_rc.borrow();

        if st.cached_lines.is_empty() { return; }

        let drag_offset = st.drag_offset;
        let active_idx = st.last_active_idx;
        let align = st.align;
        let enable_shadow = st.enable_shadow;
        let bg_color = st.bg_color;

        let (fr, fg, fb, fa) = st.text_color_override.unwrap_or_else(|| {
            let c = obj.color();
            (c.red() as f64, c.green() as f64, c.blue() as f64, c.alpha() as f64)
        });

        // Clip to widget bounds
        let clip_rect = graphene::Rect::new(0.0, 0.0, w as f32, h as f32);
        snapshot.push_clip(&clip_rect);

        for (i, cached) in st.cached_lines.iter().enumerate() {
            let line_state = &st.line_states[i];
            let line_y = line_state.y() + drag_offset;

            if line_y + cached.total_height < 0.0 || line_y > h { continue; }

            let fade_alpha = fade_alpha_for_y(line_y, h, FADE_HEIGHT);
            let alpha = line_state.current_alpha * fade_alpha;
            let scale = line_state.scale();
            let line_alpha = (fa * alpha) as f32;

            if active_idx == Some(i) {
                render_active_line(
                    snapshot, cached, st.current_ms, line_y, w, align,
                    fr, fg, fb, line_alpha,
                    scale, enable_shadow, bg_color,
                );
            } else {
                render_dim_line(
                    snapshot, cached, line_y, w, align,
                    fr, fg, fb, line_alpha,
                    scale, bg_color,
                );
            }
        }

        // Interlude dots
        if st.interlude_dots.visible {
            let (dot_y, dot_fade) = match st.interlude_dots.interlude_idx {
                Some(pi) if pi + 1 < st.line_states.len() => {
                    let bottom = st.line_states[pi].y() + drag_offset
                        + st.cached_lines[pi].total_height;
                    let top_next = st.line_states[pi + 1].y() + drag_offset;
                    let dy = (bottom + top_next) / 2.0;
                    (dy, fade_alpha_for_y(dy, h, FADE_HEIGHT))
                }
                _ if !st.line_states.is_empty() => {
                    let push = st.interlude_dots.push_amount;
                    let dy = TOP_PADDING + push / 2.0 + drag_offset;
                    (dy, fade_alpha_for_y(dy, h, FADE_HEIGHT))
                }
                _ => (0.0, 0.0),
            };
            if dot_fade > 0.01 {
                render_interlude_dots(
                    snapshot, &st.interlude_dots, dot_y, st.current_ms,
                    (fr * dot_fade, fg * dot_fade, fb * dot_fade),
                );
            }
        }

        snapshot.pop();
    }
}

// ─── GSK rendering ────────────────────────────────────────────────────────────

fn render_dim_line(
    snapshot: &gtk::Snapshot,
    cached: &CachedLine,
    y: f64,
    widget_w: f64,
    align: LyricAlign,
    fr: f64, fg: f64, fb: f64, fa: f32,
    scale: f64,
    bg_color: (f64, f64, f64),
) {
    let x = x_for_layout(widget_w, cached.text_width, align);
    let (r, g, b) = dim_color((fr, fg, fb), bg_color);

    snapshot.save();

    if (scale - 1.0).abs() > 0.001 {
        let pivot = graphene::Point::new(x as f32, y as f32);
        snapshot.translate(&pivot);
        snapshot.scale(scale as f32, scale as f32);
        let neg = graphene::Point::new(-(x as f32), -(y as f32));
        snapshot.translate(&neg);
    }

    snapshot.push_opacity(fa as f64);

    // Translate to text position, then append layout at (0,0)
    snapshot.translate(&graphene::Point::new(x as f32, y as f32));
    let color = gdk::RGBA::new(r as f32, g as f32, b as f32, 1.0f32);
    snapshot.append_layout(&cached.layout, &color);

    if let Some(tl) = &cached.tl_layout {
        let tl_x = x_for_layout(widget_w, cached.tl_text_width, align);
        let tl_y = (cached.layout_height + TL_GAP) as f32;
        // Translate relative to current position (which is already at x, y)
        let offset_x = (tl_x - x) as f32;
        snapshot.translate(&graphene::Point::new(offset_x, tl_y));
        let tl_color = gdk::RGBA::new(r as f32, g as f32, b as f32, ALPHA_DIM as f32);
        snapshot.append_layout(tl, &tl_color);
    }

    snapshot.pop(); // pop opacity
    snapshot.restore();
}

fn render_active_line(
    snapshot: &gtk::Snapshot,
    cached: &CachedLine,
    current_ms: u64,
    y: f64,
    widget_w: f64,
    align: LyricAlign,
    fr: f64, fg: f64, fb: f64, fa: f32,
    scale: f64,
    shadow: bool,
    bg_color: (f64, f64, f64),
) {
    let layout_x = x_for_layout(widget_w, cached.text_width, align);

    snapshot.save();

    if (scale - 1.0).abs() > 0.001 {
        let pivot = graphene::Point::new(layout_x as f32, y as f32);
        snapshot.translate(&pivot);
        snapshot.scale(scale as f32, scale as f32);
        let neg = graphene::Point::new(-(layout_x as f32), -(y as f32));
        snapshot.translate(&neg);
    }

    // Shadow via GSK blur
    if shadow {
        snapshot.push_blur(6.0);
        let sa = (fa as f64 * 0.35).min(0.35) as f32;
        let sc = gdk::RGBA::new(0.0, 0.0, 0.0, sa);
        snapshot.translate(&graphene::Point::new(1.0, 1.0));
        snapshot.translate(&graphene::Point::new(layout_x as f32, y as f32));
        snapshot.append_layout(&cached.layout, &sc);
        // Undo both translations for shadow offset
        snapshot.translate(&graphene::Point::new(-(layout_x as f32), -(y as f32)));
        snapshot.translate(&graphene::Point::new(-1.0, -1.0));
        snapshot.pop(); // pop blur
    }

    match &cached.line.kind {
        LyricLineKind::Verbatim(_) => {
            render_active_verbatim(
                snapshot, cached, current_ms, y, widget_w, align, bg_color,
                (fr, fg, fb), fa,
            );
        }
        LyricLineKind::Plain => {
            let pos = graphene::Point::new(layout_x as f32, y as f32);
            snapshot.translate(&pos);
            let color = gdk::RGBA::new(fr as f32, fg as f32, fb as f32, fa * ALPHA_ACTIVE as f32);
            snapshot.append_layout(&cached.layout, &color);
            snapshot.translate(&graphene::Point::new(-(layout_x as f32), -(y as f32)));
        }
    }

    // Translation
    if let Some(tl) = &cached.tl_layout {
        let (r, g, b) = dim_color((fr, fg, fb), bg_color);
        let tl_x = x_for_layout(widget_w, cached.tl_text_width, align);
        let tl_y = (y + cached.layout_height + TL_GAP) as f32;
        let tl_pos_x = tl_x as f32;
        snapshot.translate(&graphene::Point::new(tl_pos_x, tl_y));
        let tl_color = gdk::RGBA::new(r as f32, g as f32, b as f32, fa * ALPHA_DIM as f32);
        snapshot.append_layout(tl, &tl_color);
    }

    snapshot.restore();
}

fn render_active_verbatim(
    snapshot: &gtk::Snapshot,
    cached: &CachedLine,
    current_ms: u64,
    base_y: f64,
    widget_w: f64,
    align: LyricAlign,
    bg_color: (f64, f64, f64),
    (r, g, b): (f64, f64, f64),
    fa: f32,
) {
    let (fully_lit, char_progress) = cached.highlight_progress(current_ms);
    let n_chars = cached.char_x_offsets.len();
    let layout_x = x_for_layout(widget_w, cached.text_width, align);
    let (dim_r, dim_g, dim_b) = dim_color((r, g, b), bg_color);

    // Position for all layout draws in this function
    let pos_x = layout_x as f32;
    let pos_y = base_y as f32;

    // Layer 1: dim full text
    snapshot.translate(&graphene::Point::new(pos_x, pos_y));
    let dim_color_val = gdk::RGBA::new(dim_r as f32, dim_g as f32, dim_b as f32, fa);
    snapshot.append_layout(&cached.layout, &dim_color_val);
    snapshot.translate(&graphene::Point::new(-pos_x, -pos_y));

    // Layer 2: per-visual-line bright clip with mask gradient
    for (vl_idx, vl) in cached.visual_lines.iter().enumerate() {
        let chars_in_line: Vec<usize> = (0..n_chars)
            .filter(|&ci| cached.char_visual_line[ci] == vl_idx)
            .collect();
        if chars_in_line.is_empty() { continue; }

        let first_char = *chars_in_line.first().unwrap();
        let last_char = *chars_in_line.last().unwrap();

        let clip_right: Option<f64> = if fully_lit > last_char {
            Some(cached.char_x_offsets[last_char] + cached.char_widths[last_char])
        } else if fully_lit >= first_char && fully_lit <= last_char {
            if fully_lit == first_char && char_progress == 0.0 {
                None
            } else {
                let clip = if fully_lit < n_chars && cached.char_visual_line[fully_lit] == vl_idx {
                    cached.char_x_offsets[fully_lit] + cached.char_widths[fully_lit] * char_progress
                } else {
                    cached.char_x_offsets[last_char] + cached.char_widths[last_char]
                };
                Some(clip)
            }
        } else {
            None
        };

        let Some(clip_right) = clip_right else { continue; };
        if clip_right <= 0.0 { continue; }

        let vl_y = base_y + vl.y_offset;

        // Bright text clipped to revealed portion
        let clip_w = (clip_right + GRADIENT_EDGE_PX) as f32;
        let clip_rect = graphene::Rect::new(layout_x as f32, vl_y as f32, clip_w, vl.height as f32);
        snapshot.push_clip(&clip_rect);

        let bright = gdk::RGBA::new(r as f32, g as f32, b as f32, fa * ALPHA_ACTIVE as f32);
        snapshot.translate(&graphene::Point::new(pos_x, pos_y));
        snapshot.append_layout(&cached.layout, &bright);
        snapshot.translate(&graphene::Point::new(-pos_x, -pos_y));
        snapshot.pop(); // pop clip

        // Gradient edge using push_mask + linear gradient
        let grad_start = (clip_right - GRADIENT_EDGE_PX).max(0.0);
        let grad_rect = graphene::Rect::new(
            (layout_x + grad_start) as f32,
            vl_y as f32,
            (2.0 * GRADIENT_EDGE_PX) as f32,
            vl.height as f32,
        );

        snapshot.push_mask(gsk::MaskMode::Alpha);
        snapshot.append_linear_gradient(
            &grad_rect,
            &graphene::Point::new((layout_x + grad_start) as f32, vl_y as f32),
            &graphene::Point::new((layout_x + clip_right + GRADIENT_EDGE_PX) as f32, vl_y as f32),
            &[
                gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
            ],
        );
        snapshot.pop(); // end mask content (gradient)

        // Source: bright text over gradient zone
        let full_rect = graphene::Rect::new(
            layout_x as f32,
            vl_y as f32,
            (clip_right + GRADIENT_EDGE_PX) as f32,
            vl.height as f32,
        );
        snapshot.push_clip(&full_rect);
        snapshot.translate(&graphene::Point::new(pos_x, pos_y));
        snapshot.append_layout(&cached.layout, &bright);
        snapshot.translate(&graphene::Point::new(-pos_x, -pos_y));
        snapshot.pop(); // pop clip (source)

        snapshot.pop(); // pop mask
    }

    // Layer 3: long word glow
    if let LyricLineKind::Verbatim(chars) = &cached.line.kind {
        if fully_lit < n_chars {
            let ch = &chars[fully_lit];
            let dur = ch.duration;
            if dur >= 1000 {
                let progress = ((current_ms - ch.start) as f64 / dur as f64).clamp(0.0, 1.0);
                let pulse = ease_in_out_cubic(progress) as f32;
                let glow_alpha = ((dur as f64 - 1000.0) / 2000.0).min(1.0) as f32 * 0.35 * pulse;

                let char_x = (layout_x + cached.char_x_offsets[fully_lit]) as f32;
                let char_w = cached.char_widths[fully_lit] as f32;
                let vl_idx = cached.char_visual_line[fully_lit];
                let vl_y = (base_y + cached.visual_lines[vl_idx].y_offset) as f32;
                let vl_h = cached.visual_lines[vl_idx].height as f32;

                let glow_rect = graphene::Rect::new(
                    char_x - GRADIENT_EDGE_PX as f32,
                    vl_y,
                    char_w + 2.0 * GRADIENT_EDGE_PX as f32,
                    vl_h,
                );
                snapshot.push_clip(&glow_rect);
                snapshot.translate(&graphene::Point::new(pos_x, pos_y));
                let glow_color = gdk::RGBA::new(r as f32, g as f32, b as f32, fa * glow_alpha);
                snapshot.append_layout(&cached.layout, &glow_color);
                snapshot.translate(&graphene::Point::new(-pos_x, -pos_y));
                snapshot.pop(); // pop clip
            }
        }
    }
}

fn render_interlude_dots(
    snapshot: &gtk::Snapshot,
    dots: &InterludeDots,
    center_y: f64,
    current_ms: u64,
    (r, g, b): (f64, f64, f64),
) {
    if !dots.visible { return; }

    let stage = dots.current_stage(current_ms);
    let (alpha, scale, _reveal) = dots.stage_params(current_ms, stage);
    if alpha < 0.001 { return; }

    // Use append_cairo for the complex dot rendering
    let surf_w = 100.0f64;
    let surf_h = 24.0f64;
    let bounds = graphene::Rect::new(
        0.0,
        (center_y - 12.0) as f32,
        surf_w as f32,
        surf_h as f32,
    );
    let cr = snapshot.append_cairo(&bounds);
    cr.save().unwrap();

    if (scale - 1.0).abs() > 0.001 {
        let total_w = 2.0 * DOT_RADIUS + 2.0 * DOT_SPACING;
        let base_x = DOT_LEFT_MARGIN;
        let cx = base_x + total_w / 2.0;
        cr.translate(cx, center_y);
        cr.scale(scale, scale);
        cr.translate(-cx, -center_y);
    }

    for i in 0..3 {
        let dot_alpha = dots.dot_alpha(i, current_ms, stage);
        let final_alpha = dot_alpha * alpha;
        if final_alpha < 0.005 { continue; }
        let cx = DOT_LEFT_MARGIN + i as f64 * DOT_SPACING;
        cr.set_source_rgba(r, g, b, final_alpha * 0.6);
        cr.arc(cx, center_y, DOT_RADIUS, 0.0, TAU);
        cr.fill().unwrap();
    }

    cr.restore().unwrap();
}