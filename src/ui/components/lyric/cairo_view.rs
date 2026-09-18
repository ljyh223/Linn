//! Cairo/Pango correctness backend for the renderer-independent lyric model.
//!
//! This backend intentionally keeps animation modest. Its job is to define
//! wrapping, bidi, translation, seek and karaoke semantics that the GL backend
//! can later reproduce without owning a second timeline implementation.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use relm4::gtk;
use relm4::gtk::cairo;
use relm4::gtk::prelude::*;

use crate::lyrics::{
    InterludePlan, KaraokePlan, LyricsDocument, LyricsPresentation, LyricsRuntime, TimedFramePlan,
};
use crate::ui::components::lyric::pango_layout::{
    PangoLineLayout, TRANSLATION_GAP, layout_document, line_index_at_viewport_y, range_rectangles,
    soft_reveal_geometry, visible_line_range,
};
use crate::ui::components::lyric::viewport::LyricViewport;

const REVEAL_EDGE_PX: f64 = 42.0;
const FADE_HEIGHT: f64 = 96.0;
const DOT_RADIUS: f64 = 4.0;
const DOT_SPACING: f64 = 16.0;

#[derive(Clone)]
pub struct CairoLyricsView {
    widget: gtk::DrawingArea,
    state: Rc<RefCell<CairoState>>,
}

struct CairoState {
    presentation: LyricsPresentation,
    document: Option<Arc<LyricsDocument>>,
    cached_width: i32,
    lines: Vec<PangoLineLayout>,
    viewport: LyricViewport,
    text_color: Option<(f64, f64, f64, f64)>,
    album_color: (f64, f64, f64),
}

impl CairoLyricsView {
    pub fn new(
        runtime: Rc<RefCell<LyricsRuntime>>,
        presentation: LyricsPresentation,
        epoch: Instant,
        on_seek: impl Fn(u64) + 'static,
    ) -> Self {
        let widget = gtk::DrawingArea::new();
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        widget.set_can_target(true);
        widget.set_overflow(gtk::Overflow::Hidden);

        let state = Rc::new(RefCell::new(CairoState::new(presentation)));
        widget.set_draw_func({
            let state = state.clone();
            move |widget, cr, width, height| {
                let mut state = state.borrow_mut();
                state.ensure_layouts(&widget.pango_context(), width);
                let resolved = widget.color();
                state.draw(
                    cr,
                    width,
                    height,
                    (
                        resolved.red() as f64,
                        resolved.green() as f64,
                        resolved.blue() as f64,
                        resolved.alpha() as f64,
                    ),
                );
            }
        });

        widget.add_tick_callback({
            let state = state.clone();
            let runtime = runtime.clone();
            move |widget, _| {
                let now = Instant::now();
                let frame = runtime.borrow().frame(epoch.elapsed());
                let mut state = state.borrow_mut();
                let dirty = state.advance(frame, now, widget.height());
                if dirty {
                    widget.queue_draw();
                }
                gtk::glib::ControlFlow::Continue
            }
        });

        let click = gtk::GestureClick::new();
        click.connect_released({
            let state = state.clone();
            move |_, _, _, y| {
                let target = {
                    let state = state.borrow();
                    state.line_at_y(y).and_then(|index| {
                        state
                            .document
                            .as_ref()
                            .and_then(|document| document.lines.get(index))
                            .map(|line| line.start_ms)
                    })
                };
                if let Some(position_ms) = target {
                    on_seek(position_ms);
                }
            }
        });
        widget.add_controller(click);

        let drag = gtk::GestureDrag::new();
        drag.connect_drag_begin({
            let state = state.clone();
            move |_, _, _| state.borrow_mut().viewport.begin_drag()
        });
        drag.connect_drag_update({
            let state = state.clone();
            let widget = widget.clone();
            move |_, _, offset_y| {
                state.borrow_mut().viewport.update_drag(offset_y);
                widget.queue_draw();
            }
        });
        drag.connect_drag_end({
            let state = state.clone();
            move |_, _, _| state.borrow_mut().viewport.end_drag(Instant::now())
        });
        widget.add_controller(drag);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll({
            let state = state.clone();
            let widget = widget.clone();
            move |_, _, delta_y| {
                state
                    .borrow_mut()
                    .viewport
                    .nudge_scroll(delta_y * 40.0, Instant::now());
                widget.queue_draw();
                gtk::glib::Propagation::Stop
            }
        });
        widget.add_controller(scroll);

        Self { widget, state }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.widget
    }

    pub fn load_document(&self, document: Arc<LyricsDocument>) {
        self.state.borrow_mut().load(document);
        self.widget.queue_draw();
    }

    pub fn clear(&self) {
        self.state.borrow_mut().clear();
        self.widget.queue_draw();
    }

    pub fn refresh(&self) {
        self.widget.queue_draw();
    }

    pub fn sync_after_seek(&self) {
        self.state.borrow_mut().viewport.reset();
        self.widget.queue_draw();
    }

    pub fn animate_after_seek(&self) {
        self.state.borrow_mut().viewport.prepare_animated_seek();
        self.widget.queue_draw();
    }

    pub fn set_text_color(&self, r: f64, g: f64, b: f64, a: f64) {
        self.state.borrow_mut().text_color = Some((r, g, b, a));
        self.widget.queue_draw();
    }

    pub fn set_album_color(&self, r: f64, g: f64, b: f64) {
        let mut state = self.state.borrow_mut();
        if state.presentation.uses_album_palette() {
            state.album_color = (r, g, b);
            drop(state);
            self.widget.queue_draw();
        }
    }
}

impl CairoState {
    fn new(presentation: LyricsPresentation) -> Self {
        Self {
            presentation,
            document: None,
            cached_width: 0,
            lines: Vec::new(),
            viewport: LyricViewport::new(),
            text_color: None,
            album_color: (0.0, 0.0, 0.0),
        }
    }

    fn load(&mut self, document: Arc<LyricsDocument>) {
        self.document = Some(document);
        self.invalidate_layouts();
        self.viewport.reset();
    }

    fn clear(&mut self) {
        self.document = None;
        self.lines.clear();
        self.cached_width = 0;
        self.viewport.reset();
    }

    fn invalidate_layouts(&mut self) {
        self.cached_width = 0;
        self.lines.clear();
        self.viewport.invalidate_layout();
    }

    fn ensure_layouts(&mut self, context: &pangocairo::pango::Context, widget_width: i32) {
        let padding = self.presentation.horizontal_padding() as i32;
        let available_width = (widget_width - padding * 2).max(100);
        if self.cached_width == available_width {
            return;
        }
        let Some(document) = self.document.as_ref() else {
            return;
        };

        self.lines = layout_document(context, document, self.presentation, available_width);
        self.cached_width = available_width;
        self.viewport.invalidate_layout();
    }

    fn advance(
        &mut self,
        frame: Option<TimedFramePlan>,
        now: Instant,
        viewport_height: i32,
    ) -> bool {
        let focus = frame.as_ref().and_then(|frame| frame.frame.focus_line);
        let focus_center = focus.and_then(|index| {
            self.lines
                .get(index)
                .map(|line| line.y + line.main_height * 0.5)
        });
        self.viewport
            .advance(frame, focus_center, now, viewport_height)
    }

    fn line_at_y(&self, viewport_y: f64) -> Option<usize> {
        line_index_at_viewport_y(
            self.lines.len(),
            viewport_y,
            self.viewport.scroll_y(),
            |index| {
                let line = &self.lines[index];
                (line.y + self.viewport.line_offset(index), line.total_height)
            },
        )
    }

    fn draw(
        &mut self,
        cr: &cairo::Context,
        width: i32,
        height: i32,
        resolved_foreground: (f64, f64, f64, f64),
    ) {
        if self.lines.is_empty() || width <= 0 || height <= 0 {
            return;
        }

        let focus = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.focus_line);
        let focus_center = focus.and_then(|index| {
            self.lines
                .get(index)
                .map(|line| line.y + line.main_height * 0.5 + self.viewport.line_offset(index))
        });
        self.viewport.initialize(focus_center, height);

        let foreground = self.text_color.unwrap_or(resolved_foreground);
        let active = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.active_line);
        let interlude = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.interlude.clone());
        let padding = self.presentation.horizontal_padding() as f64;
        let inactive_rgb = inactive_color(self.presentation, foreground, self.album_color);

        cr.save().expect("save lyric viewport");
        cr.rectangle(0.0, 0.0, width as f64, height as f64);
        cr.clip();

        let visible_lines = visible_line_range(
            self.lines.len(),
            self.viewport.scroll_y(),
            self.viewport.interlude_push(),
            height,
            |index| {
                let line = &self.lines[index];
                (line.y, line.total_height)
            },
        );
        for index in visible_lines {
            let line = &self.lines[index];
            let y = line.y + self.viewport.line_offset(index) - self.viewport.scroll_y();
            if y + line.total_height < -8.0 || y > height as f64 + 8.0 {
                continue;
            }

            let fade = vertical_fade(y, line.total_height, height as f64);
            if fade <= 0.01 {
                continue;
            }
            let focus_mix = self.viewport.line_focus_mix(index);
            let scale = self.presentation.line_scale(focus_mix);
            let karaoke = (active == Some(index))
                .then(|| {
                    self.viewport
                        .frame()
                        .and_then(|frame| frame.frame.karaoke.as_ref())
                })
                .flatten();
            let base_alpha =
                self.presentation
                    .line_base_opacity(index, focus, karaoke.is_some(), focus_mix)
                    as f64
                    * fade
                    * foreground.3;

            let anchor_y = y + line.main_height * 0.78;
            cr.save().expect("save lyric line transition");
            cr.translate(padding, anchor_y);
            cr.scale(scale, scale);
            cr.translate(-padding, -anchor_y);
            draw_layout(
                cr,
                &line.layout,
                padding,
                y,
                (inactive_rgb.0, inactive_rgb.1, inactive_rgb.2, base_alpha),
            );

            if active == Some(index) {
                let overlay_alpha = self.presentation.active_overlay_opacity(focus_mix) as f64;
                match karaoke {
                    Some(karaoke) => draw_karaoke(
                        cr,
                        &line.layout,
                        karaoke,
                        padding,
                        y,
                        (
                            foreground.0,
                            foreground.1,
                            foreground.2,
                            foreground.3 * fade * overlay_alpha,
                        ),
                    ),
                    None => draw_layout(
                        cr,
                        &line.layout,
                        padding,
                        y,
                        (
                            foreground.0,
                            foreground.1,
                            foreground.2,
                            foreground.3 * fade * overlay_alpha,
                        ),
                    ),
                }
            }

            if let Some(translation) = &line.translation {
                let translation_y = y + line.main_height + TRANSLATION_GAP;
                let translation_alpha =
                    self.presentation
                        .translation_opacity(index, focus, focus_mix) as f64;
                draw_layout(
                    cr,
                    translation,
                    padding,
                    translation_y,
                    (
                        foreground.0,
                        foreground.1,
                        foreground.2,
                        foreground.3 * translation_alpha * fade,
                    ),
                );
            }
            cr.restore().expect("restore lyric line transition");
        }

        if let Some(interlude) = interlude.as_ref()
            && let Some(center_y) = interlude_center_y(
                &self.lines,
                interlude,
                self.viewport.interlude_push(),
                self.viewport.scroll_y(),
            )
        {
            draw_interlude_dots(cr, padding, center_y, interlude, foreground, height as f64);
        }

        cr.restore().expect("restore lyric viewport");
    }
}

fn interlude_center_y(
    lines: &[PangoLineLayout],
    interlude: &InterludePlan,
    push: f64,
    scroll_y: f64,
) -> Option<f64> {
    let base_center = match interlude.after_line {
        None => lines.first().map(|line| line.y * 0.5)?,
        Some(index) => {
            let current = lines.get(index)?;
            let next = lines.get(index + 1)?;
            (current.y + current.total_height + next.y) * 0.5
        }
    };
    Some(base_center + push * 0.5 - scroll_y)
}

fn draw_interlude_dots(
    cr: &cairo::Context,
    x: f64,
    center_y: f64,
    interlude: &InterludePlan,
    color: (f64, f64, f64, f64),
    viewport_height: f64,
) {
    let fade = vertical_fade(center_y - DOT_RADIUS, DOT_RADIUS * 2.0, viewport_height);
    let alpha = interlude.alpha as f64 * color.3 * fade * 0.6;
    if alpha <= 0.001 || interlude.scale <= 0.001 {
        return;
    }
    let group_center = x + DOT_RADIUS + DOT_SPACING;
    let reveal_right = x + (DOT_RADIUS * 2.0 + DOT_SPACING * 2.0) * interlude.reveal as f64;
    for (index, dot_alpha) in interlude.dot_alphas.iter().enumerate() {
        let original_x = x + DOT_RADIUS + index as f64 * DOT_SPACING;
        let center_x = group_center + (original_x - group_center) * interlude.scale as f64;
        let radius = DOT_RADIUS * interlude.scale as f64;
        let reveal_alpha =
            ((reveal_right - (center_x - radius)) / (radius * 2.0).max(0.01)).clamp(0.0, 1.0);
        let final_alpha = alpha * *dot_alpha as f64 * reveal_alpha;
        if final_alpha <= 0.001 {
            continue;
        }
        cr.set_source_rgba(color.0, color.1, color.2, final_alpha);
        cr.arc(center_x, center_y, radius, 0.0, std::f64::consts::TAU);
        cr.fill().expect("draw lyric interlude dot");
    }
}

fn draw_layout(
    cr: &cairo::Context,
    layout: &pangocairo::pango::Layout,
    x: f64,
    y: f64,
    color: (f64, f64, f64, f64),
) {
    cr.save().expect("save lyric layout");
    cr.move_to(x, y);
    cr.set_source_rgba(color.0, color.1, color.2, color.3);
    pangocairo::functions::show_layout(cr, layout);
    cr.restore().expect("restore lyric layout");
}

fn draw_karaoke(
    cr: &cairo::Context,
    layout: &pangocairo::pango::Layout,
    karaoke: &KaraokePlan,
    x: f64,
    y: f64,
    color: (f64, f64, f64, f64),
) {
    if karaoke.completed_text_end > 0 {
        draw_range_solid(cr, layout, 0..karaoke.completed_text_end, x, y, color);
    }
    if let Some(range) = karaoke.active_text_range.as_ref() {
        draw_range_reveal(
            cr,
            layout,
            range.clone(),
            karaoke.active_span_progress as f64,
            x,
            y,
            color,
        );
    } else if let Some(range) = karaoke.upcoming_text_range.as_ref() {
        draw_range_reveal(cr, layout, range.clone(), 0.0, x, y, color);
    }
}

fn draw_range_solid(
    cr: &cairo::Context,
    layout: &pangocairo::pango::Layout,
    range: Range<usize>,
    x: f64,
    y: f64,
    color: (f64, f64, f64, f64),
) {
    let rectangles = range_rectangles(layout, range);
    if rectangles.is_empty() {
        return;
    }
    cr.save().expect("save completed lyric range");
    for rectangle in rectangles {
        cr.rectangle(
            x + rectangle.x,
            y + rectangle.y,
            rectangle.width,
            rectangle.height,
        );
    }
    cr.clip();
    draw_layout(cr, layout, x, y, color);
    cr.restore().expect("restore completed lyric range");
}

fn draw_range_reveal(
    cr: &cairo::Context,
    layout: &pangocairo::pango::Layout,
    range: Range<usize>,
    progress: f64,
    x: f64,
    y: f64,
    color: (f64, f64, f64, f64),
) {
    let progress = progress.clamp(0.0, 1.0);
    let rectangles = range_rectangles(layout, range);
    let layout_width = layout.pixel_size().0.max(1) as f64;
    let total_width: f64 = rectangles.iter().map(|rectangle| rectangle.width).sum();
    let mut revealed_width = total_width * progress;
    for (index, rectangle) in rectangles.into_iter().enumerate() {
        if index > 0 && revealed_width <= 0.0 {
            break;
        }
        let local_progress = (revealed_width / rectangle.width).clamp(0.0, 1.0);
        revealed_width = (revealed_width - rectangle.width).max(0.0);
        let reveal = soft_reveal_geometry(rectangle, local_progress, REVEAL_EDGE_PX, layout_width);
        let (gradient_start, gradient_end, leading_alpha, trailing_alpha) = if rectangle.rtl {
            (
                reveal.edge_x - REVEAL_EDGE_PX,
                reveal.edge_x + REVEAL_EDGE_PX,
                0.0,
                color.3,
            )
        } else {
            (
                reveal.edge_x - REVEAL_EDGE_PX,
                reveal.edge_x + REVEAL_EDGE_PX,
                color.3,
                0.0,
            )
        };
        let gradient = cairo::LinearGradient::new(
            x + gradient_start,
            0.0,
            x + gradient_end.max(gradient_start + 0.01),
            0.0,
        );
        gradient.add_color_stop_rgba(0.0, color.0, color.1, color.2, leading_alpha);
        gradient.add_color_stop_rgba(1.0, color.0, color.1, color.2, trailing_alpha);

        cr.save().expect("save active lyric range");
        cr.rectangle(
            x + reveal.clip.x,
            y + reveal.clip.y,
            reveal.clip.width,
            reveal.clip.height,
        );
        cr.clip();
        cr.move_to(x, y);
        cr.set_source(&gradient).expect("set active lyric gradient");
        pangocairo::functions::show_layout(cr, layout);
        cr.restore().expect("restore active lyric range");
        if local_progress < 1.0 {
            break;
        }
    }
}

fn vertical_fade(y: f64, line_height: f64, viewport_height: f64) -> f64 {
    let center = y + line_height * 0.5;
    let top = (center / FADE_HEIGHT).clamp(0.0, 1.0);
    let bottom = ((viewport_height - center) / FADE_HEIGHT).clamp(0.0, 1.0);
    top.min(bottom)
}

fn inactive_color(
    presentation: LyricsPresentation,
    foreground: (f64, f64, f64, f64),
    album_color: (f64, f64, f64),
) -> (f64, f64, f64) {
    match presentation {
        LyricsPresentation::Sidebar => (foreground.0, foreground.1, foreground.2),
        LyricsPresentation::Fullscreen => (
            foreground.0 * 0.88 + album_color.0 * 0.12,
            foreground.1 * 0.88 + album_color.1 * 0.12,
            foreground.2 * 0.88 + album_color.2 * 0.12,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{LyricsLine, LyricsSource, TimedSpan, TimingQuality, frame_at};
    use crate::ui::components::lyric::pango_layout::make_layout;
    use pangocairo::pango::prelude::FontMapExt;

    #[test]
    fn sidebar_color_is_independent_from_album_palette() {
        let foreground = (0.8, 0.7, 0.6, 1.0);
        assert_eq!(
            inactive_color(LyricsPresentation::Sidebar, foreground, (1.0, 0.0, 0.0)),
            inactive_color(LyricsPresentation::Sidebar, foreground, (0.0, 0.0, 1.0))
        );
    }

    #[test]
    fn fullscreen_color_can_follow_album_palette() {
        let foreground = (1.0, 1.0, 1.0, 1.0);
        assert_ne!(
            inactive_color(LyricsPresentation::Fullscreen, foreground, (1.0, 0.0, 0.0)),
            inactive_color(LyricsPresentation::Fullscreen, foreground, (0.0, 0.0, 1.0))
        );
    }

    #[test]
    fn fade_is_opaque_in_center_and_hidden_outside_viewport() {
        assert_eq!(vertical_fade(180.0, 40.0, 400.0), 1.0);
        assert_eq!(vertical_fade(-100.0, 20.0, 400.0), 0.0);
        assert_eq!(vertical_fade(410.0, 20.0, 400.0), 0.0);
    }

    #[test]
    fn cairo_framebuffer_reveal_grows_with_source_progress() {
        fn render_signal(progress: f64) -> u64 {
            let context = pangocairo::FontMap::default().create_context();
            let layout = make_layout(&context, 20, 280, true);
            layout.set_text("逐字歌词");
            let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 320, 100).unwrap();
            let cr = cairo::Context::new(&surface).unwrap();
            draw_range_reveal(
                &cr,
                &layout,
                0..6,
                progress,
                16.0,
                16.0,
                (1.0, 1.0, 1.0, 1.0),
            );
            drop(cr);
            surface.flush();
            surface.data().unwrap().iter().copied().map(u64::from).sum()
        }

        let early = render_signal(0.25);
        let late = render_signal(0.75);
        assert!(
            early > 0,
            "the Cairo karaoke layer must hit real glyph pixels"
        );
        assert!(
            late > early,
            "75% source progress must reveal more Cairo pixels than 25%"
        );
    }

    #[test]
    fn fullscreen_reference_fixture_renders_karaoke_and_translation() {
        const WIDTH: i32 = 800;
        const HEIGHT: i32 = 600;
        let document = Arc::new(
            LyricsDocument::new(
                LyricsSource::Yrc,
                vec![
                    LyricsLine::new(
                        0,
                        2_000,
                        "逐字歌词",
                        Some(Arc::from("karaoke lyrics")),
                        TimingQuality::Source,
                        vec![
                            TimedSpan::new(0..6, 0, 1_000),
                            TimedSpan::new(6..12, 1_000, 2_000),
                        ],
                    )
                    .unwrap(),
                    LyricsLine::new(
                        3_000,
                        5_000,
                        "全屏模式",
                        Some(Arc::from("fullscreen presentation")),
                        TimingQuality::Line,
                        vec![],
                    )
                    .unwrap(),
                ],
            )
            .unwrap(),
        );
        let context = pangocairo::FontMap::default().create_context();
        let mut state = CairoState::new(LyricsPresentation::Fullscreen);
        state.load(document.clone());
        let available_width =
            WIDTH - LyricsPresentation::Fullscreen.horizontal_padding() as i32 * 2;
        state.lines = layout_document(
            &context,
            &document,
            LyricsPresentation::Fullscreen,
            available_width,
        );
        state.cached_width = available_width;
        state.text_color = Some((1.0, 1.0, 1.0, 1.0));
        state.album_color = (0.86, 0.18, 0.42);
        let now = Instant::now();
        let frame = TimedFramePlan {
            position_ms: 750,
            frame: frame_at(&document, 750),
        };
        state.advance(Some(frame), now, HEIGHT);

        let mut surface =
            cairo::ImageSurface::create(cairo::Format::ARgb32, WIDTH, HEIGHT).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        cr.set_source_rgb(0.035, 0.045, 0.09);
        cr.paint().unwrap();
        state.draw(&cr, WIDTH, HEIGHT, (1.0, 1.0, 1.0, 1.0));
        drop(cr);
        surface.flush();
        let stride = surface.stride() as usize;
        let data = surface.data().unwrap();
        let bright_pixels = data
            .chunks_exact(4)
            .filter(|pixel| u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]) > 420)
            .count();
        assert!(
            bright_pixels > 100,
            "the fullscreen reference frame must contain visible lyric glyphs"
        );

        if let Ok(path) = std::env::var("LINN_LYRICS_FULLSCREEN_PNG") {
            let mut rgba = vec![0_u8; WIDTH as usize * HEIGHT as usize * 4];
            for y in 0..HEIGHT as usize {
                for x in 0..WIDTH as usize {
                    let source = y * stride + x * 4;
                    let destination = (y * WIDTH as usize + x) * 4;
                    rgba[destination] = data[source + 2];
                    rgba[destination + 1] = data[source + 1];
                    rgba[destination + 2] = data[source];
                    rgba[destination + 3] = data[source + 3];
                }
            }
            image::save_buffer(
                path,
                &rgba,
                WIDTH as u32,
                HEIGHT as u32,
                image::ColorType::Rgba8,
            )
            .expect("save optional fullscreen reference fixture");
        }
    }
}
