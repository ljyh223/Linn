//! GtkGLArea/glow lyric backend using cached Pango alpha textures.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use glow::HasContext;
use relm4::gtk;
use relm4::gtk::cairo;
use relm4::gtk::prelude::*;

use crate::lyrics::{
    InterludePlan, KaraokePlan, LyricsDocument, LyricsPresentation, LyricsRuntime,
};
use crate::ui::components::gl_context::create_glow_context;
use crate::ui::components::lyric::pango_layout::{
    PangoLineLayout, TRANSLATION_GAP, TextRectangle, layout_document, line_index_at_viewport_y,
    range_rectangles, soft_reveal_geometry, visible_line_range,
};
use crate::ui::components::lyric::viewport::LyricViewport;

const FADE_HEIGHT: f64 = 96.0;
// A broad edge makes karaoke read as a flowing fill instead of a hard clip.
// This remains a shader uniform and does not create additional textures.
const REVEAL_EDGE_PX: f64 = 42.0;
const DOT_RADIUS: f32 = 4.0;
const DOT_SPACING: f32 = 16.0;
const DOT_TEXTURE_SIZE: i32 = 32;
const TEXTURE_PREFETCH_LINES: usize = 2;
const UNIT_QUAD: [f32; 24] = [
    0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 0.0, 1.0, 0.0,
];

const VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_tex_coord;
uniform vec4 u_rect;
uniform vec2 u_viewport;
out vec2 v_tex_coord;
void main() {
    vec2 pixel = u_rect.xy + a_position * u_rect.zw;
    vec2 clip = pixel / u_viewport * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_tex_coord = a_tex_coord;
}
"#;

const DESKTOP_VERTEX_SHADER: &str = r#"#version 330 core
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_tex_coord;
uniform vec4 u_rect;
uniform vec2 u_viewport;
out vec2 v_tex_coord;
void main() {
    vec2 pixel = u_rect.xy + a_position * u_rect.zw;
    vec2 clip = pixel / u_viewport * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_tex_coord = a_tex_coord;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec2 v_tex_coord;
uniform sampler2D u_mask;
uniform vec4 u_color;
uniform float u_reveal_edge;
uniform float u_reveal_softness;
uniform int u_reveal_direction;
out vec4 frag_color;
void main() {
    float coverage = texture(u_mask, v_tex_coord).r;
    float reveal = 1.0;
    if (u_reveal_direction == 1) {
        reveal = 1.0 - smoothstep(
            u_reveal_edge - u_reveal_softness,
            u_reveal_edge + u_reveal_softness,
            gl_FragCoord.x
        );
    } else if (u_reveal_direction == -1) {
        reveal = smoothstep(
            u_reveal_edge - u_reveal_softness,
            u_reveal_edge + u_reveal_softness,
            gl_FragCoord.x
        );
    }
    float alpha = coverage * u_color.a * reveal;
    frag_color = vec4(u_color.rgb, alpha);
}
"#;

const DESKTOP_FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 v_tex_coord;
uniform sampler2D u_mask;
uniform vec4 u_color;
uniform float u_reveal_edge;
uniform float u_reveal_softness;
uniform int u_reveal_direction;
out vec4 frag_color;
void main() {
    float coverage = texture(u_mask, v_tex_coord).r;
    float reveal = 1.0;
    if (u_reveal_direction == 1) {
        reveal = 1.0 - smoothstep(
            u_reveal_edge - u_reveal_softness,
            u_reveal_edge + u_reveal_softness,
            gl_FragCoord.x
        );
    } else if (u_reveal_direction == -1) {
        reveal = smoothstep(
            u_reveal_edge - u_reveal_softness,
            u_reveal_edge + u_reveal_softness,
            gl_FragCoord.x
        );
    }
    float alpha = coverage * u_color.a * reveal;
    frag_color = vec4(u_color.rgb, alpha);
}
"#;

#[derive(Clone)]
pub struct GlLyricsView {
    widget: gtk::GLArea,
    state: Rc<RefCell<GlState>>,
    #[cfg(test)]
    on_failure: Rc<dyn Fn()>,
}

struct GlState {
    presentation: LyricsPresentation,
    document: Option<Arc<LyricsDocument>>,
    cached_width: i32,
    cached_scale: i32,
    lines: Vec<GlLine>,
    layout_generation: u64,
    viewport: LyricViewport,
    text_color: Option<(f64, f64, f64, f64)>,
    album_color: (f64, f64, f64),
    gpu: Option<GpuState>,
    command_buffer: Vec<DrawCommand>,
    failed: bool,
}

struct GlLine {
    pango: PangoLineLayout,
    texture_width: i32,
    texture_height: i32,
    texture_scale: i32,
}

struct AlphaBitmap {
    width: i32,
    height: i32,
    pixels: Vec<u8>,
}

struct GpuState {
    gl: glow::Context,
    renderer: TextureRenderer,
    uploaded_generation: u64,
}

#[derive(Clone, Copy)]
struct DrawCommand {
    texture: TextureSource,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    scissor: Option<Scissor>,
    reveal: Option<Reveal>,
}

#[derive(Clone, Copy)]
enum TextureSource {
    Line(usize),
    Dot,
}

#[derive(Clone, Copy)]
struct Scissor {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct Reveal {
    edge_x: f32,
    rtl: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextureWindow {
    start: usize,
    end: usize,
}

impl TextureWindow {
    fn indices(self) -> std::ops::RangeInclusive<usize> {
        self.start..=self.end
    }

    fn contains(self, index: usize) -> bool {
        self.indices().contains(&index)
    }

    #[cfg(test)]
    fn len(self) -> usize {
        self.end - self.start + 1
    }
}

impl GlLyricsView {
    pub fn new(
        runtime: Rc<RefCell<LyricsRuntime>>,
        presentation: LyricsPresentation,
        epoch: Instant,
        on_seek: impl Fn(u64) + 'static,
        on_failure: impl Fn() + 'static,
    ) -> Self {
        let widget = gtk::GLArea::new();
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        widget.set_auto_render(false);
        widget.set_has_depth_buffer(false);
        widget.set_has_stencil_buffer(false);
        widget.set_required_version(3, 3);

        let state = Rc::new(RefCell::new(GlState::new(presentation)));
        let on_failure: Rc<dyn Fn()> = Rc::new(on_failure);
        widget.connect_realize({
            let state = state.clone();
            let on_failure = on_failure.clone();
            move |area| {
                area.make_current();
                let result = area
                    .error()
                    .map_or_else(create_glow_context, |error| Err(error.to_string()))
                    .and_then(|gl| {
                        let uses_es = area.context().is_some_and(|context| context.uses_es());
                        TextureRenderer::new(&gl, uses_es).map(|renderer| GpuState {
                            gl,
                            renderer,
                            uploaded_generation: u64::MAX,
                        })
                    });
                match result {
                    Ok(gpu) => {
                        state.borrow_mut().gpu = Some(gpu);
                        area.queue_render();
                    }
                    Err(error) => {
                        log::error!("[lyrics][gl] initialization failed: {error}");
                        fail_gl_backend(area, &state, &on_failure);
                    }
                }
            }
        });

        widget.connect_render({
            let state = state.clone();
            let on_failure = on_failure.clone();
            move |area, _| {
                let error = state.borrow_mut().render(area).err();
                if let Some(error) = error {
                    log::error!("[lyrics][gl] render failed: {error}");
                    fail_gl_backend(area, &state, &on_failure);
                }
                gtk::glib::Propagation::Stop
            }
        });

        widget.connect_unrealize({
            let state = state.clone();
            move |area| {
                area.make_current();
                if area.error().is_none()
                    && let Some(mut gpu) = state.borrow_mut().gpu.take()
                {
                    gpu.renderer.destroy(&gpu.gl);
                }
                // CPU masks are deliberately discarded after upload. Force a
                // fresh rasterization if GTK later recreates this GL context.
                state.borrow_mut().invalidate_layouts();
            }
        });

        widget.add_tick_callback({
            let state = state.clone();
            let runtime = runtime.clone();
            move |area, _| {
                let frame = runtime.borrow().frame(epoch.elapsed());
                let mut state = state.borrow_mut();
                let focus = frame.as_ref().and_then(|frame| frame.frame.focus_line);
                let focus_center = focus.and_then(|index| {
                    state
                        .lines
                        .get(index)
                        .map(|line| line.pango.y + line.pango.main_height * 0.5)
                });
                if state
                    .viewport
                    .advance(frame, focus_center, Instant::now(), area.height())
                {
                    area.queue_render();
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
                widget.queue_render();
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
                widget.queue_render();
                gtk::glib::Propagation::Stop
            }
        });
        widget.add_controller(scroll);

        Self {
            widget,
            state,
            #[cfg(test)]
            on_failure,
        }
    }

    pub fn widget(&self) -> &gtk::GLArea {
        &self.widget
    }

    pub fn load_document(&self, document: Arc<LyricsDocument>) {
        self.state.borrow_mut().load(document);
        self.widget.queue_render();
    }

    pub fn clear(&self) {
        self.state.borrow_mut().clear();
        self.widget.queue_render();
    }

    pub fn refresh(&self) {
        self.widget.queue_render();
    }

    pub fn sync_after_seek(&self) {
        self.state.borrow_mut().viewport.reset();
        self.widget.queue_render();
    }

    pub fn animate_after_seek(&self) {
        self.state.borrow_mut().viewport.prepare_animated_seek();
        self.widget.queue_render();
    }

    pub fn set_text_color(&self, r: f64, g: f64, b: f64, a: f64) {
        self.state.borrow_mut().text_color = Some((r, g, b, a));
        self.widget.queue_render();
    }

    pub fn set_album_color(&self, r: f64, g: f64, b: f64) {
        let mut state = self.state.borrow_mut();
        if state.presentation.uses_album_palette() {
            state.album_color = (r, g, b);
            drop(state);
            self.widget.queue_render();
        }
    }

    #[cfg(test)]
    fn force_fallback_for_test(&self) {
        self.widget.make_current();
        fail_gl_backend(&self.widget, &self.state, &self.on_failure);
    }
}

fn fail_gl_backend(area: &gtk::GLArea, state: &Rc<RefCell<GlState>>, on_failure: &Rc<dyn Fn()>) {
    let mut state = state.borrow_mut();
    if state.failed {
        return;
    }
    state.failed = true;
    // Cairo becomes the sole backend after a failure. Do not retain a second
    // song's worth of Pango layouts or live textures behind the hidden GLArea.
    // Delete GL objects only while GTK still reports a usable current context;
    // otherwise the context owner will reclaim them during teardown.
    if area.error().is_none()
        && let Some(mut gpu) = state.gpu.take()
    {
        gpu.renderer.destroy(&gpu.gl);
    } else {
        state.gpu.take();
    }
    state.invalidate_layouts();
    drop(state);
    on_failure();
}

impl GlState {
    fn new(presentation: LyricsPresentation) -> Self {
        Self {
            presentation,
            document: None,
            cached_width: 0,
            cached_scale: 0,
            lines: Vec::new(),
            layout_generation: 0,
            viewport: LyricViewport::new(),
            text_color: None,
            album_color: (0.0, 0.0, 0.0),
            gpu: None,
            command_buffer: Vec::new(),
            failed: false,
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
        self.cached_scale = 0;
        self.layout_generation = self.layout_generation.wrapping_add(1);
        self.viewport.reset();
    }

    fn invalidate_layouts(&mut self) {
        self.cached_width = 0;
        self.cached_scale = 0;
        self.lines.clear();
        self.viewport.invalidate_layout();
    }

    fn ensure_lines(&mut self, area: &gtk::GLArea) -> Result<(), String> {
        let scale = area.scale_factor().max(1);
        let padding = self.presentation.horizontal_padding() as i32;
        let available_width = (area.width() - padding * 2).max(100);
        if self.cached_width == available_width && self.cached_scale == scale {
            return Ok(());
        }
        let Some(document) = self.document.as_ref() else {
            return Ok(());
        };

        let layouts = layout_document(
            &area.pango_context(),
            document,
            self.presentation,
            available_width,
        );
        self.lines = layouts
            .into_iter()
            .map(|pango| {
                let (texture_width, texture_height) =
                    bitmap_dimensions(&pango, available_width, scale);
                GlLine {
                    pango,
                    texture_width,
                    texture_height,
                    texture_scale: scale,
                }
            })
            .collect();
        self.cached_width = available_width;
        self.cached_scale = scale;
        self.layout_generation = self.layout_generation.wrapping_add(1);
        self.viewport.invalidate_layout();
        Ok(())
    }

    fn render(&mut self, area: &gtk::GLArea) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        self.ensure_lines(area)?;

        let scale = area.scale_factor().max(1);
        let viewport_width = area.width().max(1);
        let viewport_height = area.height().max(1);
        let focus = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.focus_line);
        let focus_center = focus.and_then(|index| {
            self.lines.get(index).map(|line| {
                line.pango.y + line.pango.main_height * 0.5 + self.viewport.line_offset(index)
            })
        });
        self.viewport.initialize(focus_center, viewport_height);
        let widget_color = area.color();
        let resolved = self.text_color.unwrap_or((
            widget_color.red() as f64,
            widget_color.green() as f64,
            widget_color.blue() as f64,
            widget_color.alpha() as f64,
        ));
        let mut commands = std::mem::take(&mut self.command_buffer);
        commands.clear();
        self.append_draw_commands(&mut commands, viewport_height, resolved);
        let wanted_textures = wanted_texture_window(&commands, self.lines.len());

        let Some(gpu) = self.gpu.as_mut() else {
            self.command_buffer = commands;
            return Ok(());
        };
        if gpu.uploaded_generation != self.layout_generation {
            gpu.renderer.clear_line_textures(&gpu.gl);
            gpu.uploaded_generation = self.layout_generation;
        }
        let mut uploads = Vec::new();
        let mut upload_bytes = 0_usize;
        if let Some(window) = wanted_textures {
            for index in window.indices() {
                if gpu.renderer.has_line_texture(index) {
                    continue;
                }
                let bitmap = rasterize_line(
                    &self.lines[index].pango,
                    self.cached_width,
                    self.cached_scale,
                )?;
                upload_bytes += bitmap.pixels.len();
                uploads.push((index, bitmap));
            }
        }
        let uploaded_count = uploads.len();
        if let Err(error) = gpu
            .renderer
            .sync_line_textures(&gpu.gl, wanted_textures, uploads)
        {
            self.command_buffer = commands;
            return Err(error);
        }
        if upload_bytes > 0 {
            log::debug!(
                "[lyrics][gl] uploaded {} visible line masks ({} KiB), cached={}, generation={}",
                uploaded_count,
                upload_bytes.div_ceil(1024),
                gpu.renderer.line_texture_count(),
                self.layout_generation
            );
        }
        gpu.renderer.draw(
            &gpu.gl,
            viewport_width * scale,
            viewport_height * scale,
            scale,
            &commands,
        );
        self.command_buffer = commands;
        Ok(())
    }

    fn append_draw_commands(
        &self,
        commands: &mut Vec<DrawCommand>,
        viewport_height: i32,
        resolved: (f64, f64, f64, f64),
    ) {
        let inactive = inactive_color(self.presentation, resolved, self.album_color);
        let active = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.active_line);
        let focus = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.focus_line);
        let padding = self.presentation.horizontal_padding();
        let interlude = self
            .viewport
            .frame()
            .and_then(|frame| frame.frame.interlude.clone());

        let scroll_y = self.viewport.scroll_y();
        let visible_lines = visible_line_range(
            self.lines.len(),
            scroll_y,
            self.viewport.interlude_push(),
            viewport_height,
            |index| {
                let line = &self.lines[index];
                (line.pango.y, line.pango.total_height)
            },
        );
        for index in visible_lines {
            let line = &self.lines[index];
            let y = line.pango.y + self.viewport.line_offset(index) - scroll_y;
            if y + line.pango.total_height < -8.0 || y > viewport_height as f64 + 8.0 {
                continue;
            }
            let fade = vertical_fade(y, line.pango.total_height, viewport_height as f64);
            if fade <= 0.01 {
                continue;
            }
            let focus_mix = self.viewport.line_focus_mix(index);
            let scale = self.presentation.line_scale(focus_mix) as f32;
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
                    * resolved.3;
            let texture_width = line.texture_width as f32 / line.texture_scale as f32;
            let texture_height = line.texture_height as f32 / line.texture_scale as f32;
            let line_command_start = commands.len();
            commands.push(rect_command(
                index,
                padding,
                y as f32,
                texture_width,
                texture_height,
                [
                    inactive.0 as f32,
                    inactive.1 as f32,
                    inactive.2 as f32,
                    base_alpha as f32,
                ],
                Scissor {
                    x: padding,
                    y: y as f32,
                    width: texture_width,
                    height: line.pango.main_height as f32,
                },
            ));

            if line.pango.translation.is_some() {
                let translation_alpha =
                    self.presentation
                        .translation_opacity(index, focus, focus_mix) as f64;
                commands.push(rect_command(
                    index,
                    padding,
                    y as f32,
                    texture_width,
                    texture_height,
                    [
                        resolved.0 as f32,
                        resolved.1 as f32,
                        resolved.2 as f32,
                        (resolved.3 * translation_alpha * fade) as f32,
                    ],
                    Scissor {
                        x: padding,
                        y: y as f32 + line.pango.main_height as f32,
                        width: texture_width,
                        height: (line.pango.total_height - line.pango.main_height) as f32,
                    },
                ));
            }

            if active == Some(index) {
                let overlay_alpha = self.presentation.active_overlay_opacity(focus_mix) as f64;
                let color = [
                    resolved.0 as f32,
                    resolved.1 as f32,
                    resolved.2 as f32,
                    (resolved.3 * fade * overlay_alpha) as f32,
                ];
                if let Some(karaoke) = karaoke {
                    append_karaoke_commands(
                        commands, index, line, karaoke, padding, y as f32, color,
                    );
                } else {
                    commands.push(rect_command(
                        index,
                        padding,
                        y as f32,
                        texture_width,
                        texture_height,
                        color,
                        Scissor {
                            x: padding,
                            y: y as f32,
                            width: texture_width,
                            height: line.pango.main_height as f32,
                        },
                    ));
                }
            }

            transform_line_commands(
                &mut commands[line_command_start..],
                padding,
                y as f32 + line.pango.main_height as f32 * 0.78,
                scale,
            );
        }
        if let Some(interlude) = interlude.as_ref()
            && let Some(center_y) = interlude_center_y(
                &self.lines,
                interlude,
                self.viewport.interlude_push(),
                self.viewport.scroll_y(),
            )
        {
            append_interlude_commands(
                commands,
                padding,
                center_y as f32,
                interlude,
                resolved,
                viewport_height,
            );
        }
    }

    #[cfg(test)]
    fn draw_commands(
        &self,
        viewport_height: i32,
        resolved: (f64, f64, f64, f64),
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        self.append_draw_commands(&mut commands, viewport_height, resolved);
        commands
    }

    fn line_at_y(&self, viewport_y: f64) -> Option<usize> {
        line_index_at_viewport_y(
            self.lines.len(),
            viewport_y,
            self.viewport.scroll_y(),
            |index| {
                let line = &self.lines[index];
                (
                    line.pango.y + self.viewport.line_offset(index),
                    line.pango.total_height,
                )
            },
        )
    }
}

fn rasterize_line(
    line: &PangoLineLayout,
    available_width: i32,
    scale: i32,
) -> Result<AlphaBitmap, String> {
    let (width, height) = bitmap_dimensions(line, available_width, scale);
    let mut surface = cairo::ImageSurface::create(cairo::Format::A8, width, height)
        .map_err(|error| error.to_string())?;
    {
        let context = cairo::Context::new(&surface).map_err(|error| error.to_string())?;
        context.set_operator(cairo::Operator::Clear);
        context.paint().map_err(|error| error.to_string())?;
        context.set_operator(cairo::Operator::Over);
        context.scale(scale as f64, scale as f64);
        context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        context.move_to(0.0, 0.0);
        pangocairo::functions::show_layout(&context, &line.layout);
        if let Some(translation) = &line.translation {
            context.move_to(0.0, line.main_height + TRANSLATION_GAP);
            pangocairo::functions::show_layout(&context, translation);
        }
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let row_width = width as usize;
    let surface_data = surface.data().map_err(|error| error.to_string())?;
    let mut pixels = Vec::with_capacity(row_width * height as usize);
    for row in 0..height as usize {
        let start = row * stride;
        pixels.extend_from_slice(&surface_data[start..start + row_width]);
    }
    Ok(AlphaBitmap {
        width,
        height,
        pixels,
    })
}

fn bitmap_dimensions(line: &PangoLineLayout, available_width: i32, scale: i32) -> (i32, i32) {
    let logical_width = line
        .translation
        .as_ref()
        .map_or_else(
            || layout_ink_right(&line.layout),
            |translation| layout_ink_right(&line.layout).max(layout_ink_right(translation)),
        )
        .saturating_add(2)
        .clamp(1, available_width.max(1));
    let width = (logical_width * scale).max(1);
    let height = ((line.total_height.ceil() as i32).max(1) * scale).max(1);
    (width, height)
}

fn layout_ink_right(layout: &pangocairo::pango::Layout) -> i32 {
    let (ink, _) = layout.pixel_extents();
    ink.x().saturating_add(ink.width()).max(1)
}

fn wanted_texture_window(commands: &[DrawCommand], line_count: usize) -> Option<TextureWindow> {
    if line_count == 0 {
        return None;
    }
    let mut visible = commands.iter().filter_map(|command| match command.texture {
        TextureSource::Line(index) => Some(index),
        TextureSource::Dot => None,
    });
    let first = visible.next()?;
    let (minimum, maximum) = visible.fold((first, first), |(minimum, maximum), index| {
        (minimum.min(index), maximum.max(index))
    });
    Some(TextureWindow {
        start: minimum.saturating_sub(TEXTURE_PREFETCH_LINES),
        end: maximum
            .saturating_add(TEXTURE_PREFETCH_LINES)
            .min(line_count - 1),
    })
}

fn interlude_center_y(
    lines: &[GlLine],
    interlude: &InterludePlan,
    push: f64,
    scroll_y: f64,
) -> Option<f64> {
    let base_center = match interlude.after_line {
        None => lines.first().map(|line| line.pango.y * 0.5)?,
        Some(index) => {
            let current = lines.get(index)?;
            let next = lines.get(index + 1)?;
            (current.pango.y + current.pango.total_height + next.pango.y) * 0.5
        }
    };
    Some(base_center + push * 0.5 - scroll_y)
}

fn append_interlude_commands(
    commands: &mut Vec<DrawCommand>,
    x: f32,
    center_y: f32,
    interlude: &InterludePlan,
    color: (f64, f64, f64, f64),
    viewport_height: i32,
) {
    let fade = vertical_fade(
        (center_y - DOT_RADIUS) as f64,
        (DOT_RADIUS * 2.0) as f64,
        viewport_height as f64,
    ) as f32;
    let alpha = interlude.alpha * color.3 as f32 * fade * 0.6;
    if alpha <= 0.001 || interlude.scale <= 0.001 {
        return;
    }

    let group_center = x + DOT_RADIUS + DOT_SPACING;
    let reveal_right = x + (DOT_RADIUS * 2.0 + DOT_SPACING * 2.0) * interlude.reveal;
    for (index, dot_alpha) in interlude.dot_alphas.iter().enumerate() {
        let original_center = x + DOT_RADIUS + index as f32 * DOT_SPACING;
        let center_x = group_center + (original_center - group_center) * interlude.scale;
        let radius = DOT_RADIUS * interlude.scale;
        let reveal_alpha =
            ((reveal_right - (center_x - radius)) / (radius * 2.0).max(0.01)).clamp(0.0, 1.0);
        let final_alpha = alpha * *dot_alpha * reveal_alpha;
        if final_alpha <= 0.001 {
            continue;
        }
        commands.push(DrawCommand {
            texture: TextureSource::Dot,
            x: center_x - radius,
            y: center_y - radius,
            width: radius * 2.0,
            height: radius * 2.0,
            color: [color.0 as f32, color.1 as f32, color.2 as f32, final_alpha],
            scissor: None,
            reveal: None,
        });
    }
}

fn append_karaoke_commands(
    commands: &mut Vec<DrawCommand>,
    texture_index: usize,
    line: &GlLine,
    karaoke: &KaraokePlan,
    x: f32,
    y: f32,
    color: [f32; 4],
) {
    let texture_width = line.texture_width as f32 / line.texture_scale as f32;
    let texture_height = line.texture_height as f32 / line.texture_scale as f32;
    if karaoke.completed_text_end > 0 {
        for rectangle in range_rectangles(&line.pango.layout, 0..karaoke.completed_text_end) {
            commands.push(clipped_command(
                texture_index,
                x,
                y,
                (texture_width, texture_height),
                rectangle,
                color,
                None,
            ));
        }
    }

    let (range, span_progress) = if let Some(range) = karaoke.active_text_range.as_ref() {
        (range, karaoke.active_span_progress)
    } else if let Some(range) = karaoke.upcoming_text_range.as_ref() {
        (range, 0.0)
    } else {
        return;
    };
    let rectangles = range_rectangles(&line.pango.layout, range.clone());
    let layout_width = line.pango.layout.pixel_size().0.max(1) as f64;
    let total_width: f64 = rectangles.iter().map(|rectangle| rectangle.width).sum();
    let mut revealed_width = total_width * span_progress as f64;
    for (index, rectangle) in rectangles.into_iter().enumerate() {
        if index > 0 && revealed_width <= 0.0 {
            break;
        }
        let local_progress = (revealed_width / rectangle.width).clamp(0.0, 1.0);
        revealed_width = (revealed_width - rectangle.width).max(0.0);
        let reveal = soft_reveal_geometry(rectangle, local_progress, REVEAL_EDGE_PX, layout_width);
        commands.push(clipped_command(
            texture_index,
            x,
            y,
            (texture_width, texture_height),
            reveal.clip,
            color,
            Some(Reveal {
                edge_x: x + reveal.edge_x as f32,
                rtl: rectangle.rtl,
            }),
        ));
        if local_progress < 1.0 {
            break;
        }
    }
}

fn clipped_command(
    texture_index: usize,
    x: f32,
    y: f32,
    texture_size: (f32, f32),
    rectangle: TextRectangle,
    color: [f32; 4],
    reveal: Option<Reveal>,
) -> DrawCommand {
    DrawCommand {
        texture: TextureSource::Line(texture_index),
        x,
        y,
        width: texture_size.0,
        height: texture_size.1,
        color,
        scissor: Some(Scissor {
            x: x + rectangle.x as f32,
            y: y + rectangle.y as f32,
            width: rectangle.width as f32,
            height: rectangle.height as f32,
        }),
        reveal,
    }
}

fn rect_command(
    texture_index: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    scissor: Scissor,
) -> DrawCommand {
    DrawCommand {
        texture: TextureSource::Line(texture_index),
        x,
        y,
        width,
        height,
        color,
        scissor: Some(scissor),
        reveal: None,
    }
}

fn transform_line_commands(commands: &mut [DrawCommand], anchor_x: f32, anchor_y: f32, scale: f32) {
    if (scale - 1.0).abs() <= f32::EPSILON {
        return;
    }
    for command in commands {
        command.x = anchor_x + (command.x - anchor_x) * scale;
        command.y = anchor_y + (command.y - anchor_y) * scale;
        command.width *= scale;
        command.height *= scale;
        if let Some(scissor) = command.scissor.as_mut() {
            scissor.x = anchor_x + (scissor.x - anchor_x) * scale;
            scissor.y = anchor_y + (scissor.y - anchor_y) * scale;
            scissor.width *= scale;
            scissor.height *= scale;
        }
        if let Some(reveal) = command.reveal.as_mut() {
            reveal.edge_x = anchor_x + (reveal.edge_x - anchor_x) * scale;
        }
    }
}

struct TextureRenderer {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    mask_uniform: glow::UniformLocation,
    color_uniform: glow::UniformLocation,
    reveal_edge_uniform: glow::UniformLocation,
    reveal_softness_uniform: glow::UniformLocation,
    reveal_direction_uniform: glow::UniformLocation,
    rect_uniform: glow::UniformLocation,
    viewport_uniform: glow::UniformLocation,
    dot_texture: glow::Texture,
    textures: HashMap<usize, glow::Texture>,
}

impl TextureRenderer {
    fn new(gl: &glow::Context, uses_es: bool) -> Result<Self, String> {
        unsafe {
            let (vertex_shader, fragment_shader) = if uses_es {
                (VERTEX_SHADER, FRAGMENT_SHADER)
            } else {
                (DESKTOP_VERTEX_SHADER, DESKTOP_FRAGMENT_SHADER)
            };
            let program = create_program(gl, vertex_shader, fragment_shader)?;
            let mask_uniform = required_uniform(gl, program, "u_mask")?;
            let color_uniform = required_uniform(gl, program, "u_color")?;
            let reveal_edge_uniform = required_uniform(gl, program, "u_reveal_edge")?;
            let reveal_softness_uniform = required_uniform(gl, program, "u_reveal_softness")?;
            let reveal_direction_uniform = required_uniform(gl, program, "u_reveal_direction")?;
            let rect_uniform = required_uniform(gl, program, "u_rect")?;
            let viewport_uniform = required_uniform(gl, program, "u_viewport")?;
            let vao = gl
                .create_vertex_array()
                .map_err(|error| error.to_string())?;
            let vbo = gl.create_buffer().map_err(|error| error.to_string())?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, f32_bytes(&UNIT_QUAD), glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);
            gl.enable_vertex_attrib_array(1);
            gl.bind_vertex_array(None);
            let dot_texture = create_dot_texture(gl)?;
            Ok(Self {
                program,
                vao,
                vbo,
                mask_uniform,
                color_uniform,
                reveal_edge_uniform,
                reveal_softness_uniform,
                reveal_direction_uniform,
                rect_uniform,
                viewport_uniform,
                dot_texture,
                textures: HashMap::new(),
            })
        }
    }

    fn has_line_texture(&self, index: usize) -> bool {
        self.textures.contains_key(&index)
    }

    fn line_texture_count(&self) -> usize {
        self.textures.len()
    }

    fn clear_line_textures(&mut self, gl: &glow::Context) {
        unsafe {
            for (_, texture) in self.textures.drain() {
                gl.delete_texture(texture);
            }
        }
    }

    fn sync_line_textures(
        &mut self,
        gl: &glow::Context,
        wanted: Option<TextureWindow>,
        uploads: Vec<(usize, AlphaBitmap)>,
    ) -> Result<(), String> {
        unsafe {
            self.textures.retain(|index, texture| {
                let keep = wanted.is_some_and(|window| window.contains(*index));
                if !keep {
                    gl.delete_texture(*texture);
                }
                keep
            });
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            for (index, bitmap) in uploads {
                let texture = gl.create_texture().map_err(|error| error.to_string())?;
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::R8 as i32,
                    bitmap.width,
                    bitmap.height,
                    0,
                    glow::RED,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&bitmap.pixels)),
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );
                self.textures.insert(index, texture);
            }
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        Ok(())
    }

    fn draw(
        &mut self,
        gl: &glow::Context,
        viewport_width: i32,
        viewport_height: i32,
        scale: i32,
        commands: &[DrawCommand],
    ) {
        unsafe {
            gl.viewport(0, 0, viewport_width, viewport_height);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.program));
            gl.uniform_1_i32(Some(&self.mask_uniform), 0);
            gl.uniform_2_f32(
                Some(&self.viewport_uniform),
                viewport_width as f32,
                viewport_height as f32,
            );
            gl.bind_vertex_array(Some(self.vao));
            gl.active_texture(glow::TEXTURE0);

            for command in commands {
                let texture = match command.texture {
                    TextureSource::Line(index) => {
                        let Some(texture) = self.textures.get(&index).copied() else {
                            continue;
                        };
                        texture
                    }
                    TextureSource::Dot => self.dot_texture,
                };
                if let Some(scissor) = command.scissor {
                    let sx = (scissor.x * scale as f32).floor() as i32;
                    let sy = viewport_height
                        - ((scissor.y + scissor.height) * scale as f32).ceil() as i32;
                    let sw = (scissor.width * scale as f32).ceil().max(1.0) as i32;
                    let sh = (scissor.height * scale as f32).ceil().max(1.0) as i32;
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(sx.max(0), sy.max(0), sw, sh);
                } else {
                    gl.disable(glow::SCISSOR_TEST);
                }

                let reveal_direction = command
                    .reveal
                    .map_or(0, |reveal| if reveal.rtl { -1 } else { 1 });
                let reveal_edge = command
                    .reveal
                    .map_or(0.0, |reveal| reveal.edge_x * scale as f32);
                gl.uniform_1_i32(Some(&self.reveal_direction_uniform), reveal_direction);
                gl.uniform_1_f32(Some(&self.reveal_edge_uniform), reveal_edge);
                gl.uniform_1_f32(
                    Some(&self.reveal_softness_uniform),
                    REVEAL_EDGE_PX as f32 * scale as f32,
                );
                gl.uniform_4_f32(
                    Some(&self.color_uniform),
                    command.color[0],
                    command.color[1],
                    command.color[2],
                    command.color[3],
                );

                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.uniform_4_f32(
                    Some(&self.rect_uniform),
                    command.x * scale as f32,
                    command.y * scale as f32,
                    command.width * scale as f32,
                    command.height * scale as f32,
                );
                gl.draw_arrays(glow::TRIANGLES, 0, 6);
            }
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            for (_, texture) in self.textures.drain() {
                gl.delete_texture(texture);
            }
            gl.delete_texture(self.dot_texture);
            gl.delete_buffer(self.vbo);
            gl.delete_vertex_array(self.vao);
            gl.delete_program(self.program);
        }
    }
}

fn create_dot_texture(gl: &glow::Context) -> Result<glow::Texture, String> {
    let radius = DOT_TEXTURE_SIZE as f32 * 0.5 - 1.0;
    let center = (DOT_TEXTURE_SIZE as f32 - 1.0) * 0.5;
    let mut pixels = Vec::with_capacity((DOT_TEXTURE_SIZE * DOT_TEXTURE_SIZE) as usize);
    for y in 0..DOT_TEXTURE_SIZE {
        for x in 0..DOT_TEXTURE_SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let coverage = (radius + 0.5 - dx.hypot(dy)).clamp(0.0, 1.0);
            pixels.push((coverage * 255.0).round() as u8);
        }
    }

    unsafe {
        let texture = gl.create_texture().map_err(|error| error.to_string())?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8 as i32,
            DOT_TEXTURE_SIZE,
            DOT_TEXTURE_SIZE,
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&pixels)),
        );
        for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::LINEAR as i32);
        }
        for parameter in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::CLAMP_TO_EDGE as i32);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(texture)
    }
}

fn required_uniform(
    gl: &glow::Context,
    program: glow::Program,
    name: &str,
) -> Result<glow::UniformLocation, String> {
    unsafe {
        gl.get_uniform_location(program, name)
            .ok_or_else(|| format!("required lyric shader uniform `{name}` was optimized out"))
    }
}

unsafe fn create_program(
    gl: &glow::Context,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<glow::Program, String> {
    unsafe {
        let vertex = compile_shader(gl, glow::VERTEX_SHADER, vertex_source)?;
        let fragment = match compile_shader(gl, glow::FRAGMENT_SHADER, fragment_source) {
            Ok(shader) => shader,
            Err(error) => {
                gl.delete_shader(vertex);
                return Err(error);
            }
        };
        let program = gl.create_program().map_err(|error| error.to_string())?;
        gl.attach_shader(program, vertex);
        gl.attach_shader(program, fragment);
        gl.link_program(program);
        let linked = gl.get_program_link_status(program);
        let log = gl.get_program_info_log(program);
        gl.detach_shader(program, vertex);
        gl.detach_shader(program, fragment);
        gl.delete_shader(vertex);
        gl.delete_shader(fragment);
        if !linked {
            gl.delete_program(program);
            return Err(format!("program link failed: {log}"));
        }
        Ok(program)
    }
}

unsafe fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl
            .create_shader(shader_type)
            .map_err(|error| error.to_string())?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(format!("shader compile failed: {log}"));
        }
        Ok(shader)
    }
}

fn f32_bytes(values: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
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

fn vertical_fade(y: f64, line_height: f64, viewport_height: f64) -> f64 {
    let center = y + line_height * 0.5;
    let top = (center / FADE_HEIGHT).clamp(0.0, 1.0);
    let bottom = ((viewport_height - center) / FADE_HEIGHT).clamp(0.0, 1.0);
    top.min(bottom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::time::Duration;

    use crate::lyrics::{
        LyricsLine, LyricsSource, RawLyrics, TimedSpan, TimingQuality, frame_at, parse_document,
    };
    use crate::ui::components::lyric::cairo_view::CairoLyricsView;
    use pangocairo::pango::prelude::FontMapExt;

    #[test]
    fn static_quad_covers_the_rect_with_matching_texture_coordinates() {
        assert_eq!(&UNIT_QUAD[0..4], &[0.0, 1.0, 0.0, 1.0]);
        assert_eq!(&UNIT_QUAD[20..24], &[1.0, 0.0, 1.0, 0.0]);
        assert_eq!(UNIT_QUAD.len(), 6 * 4);
    }

    #[test]
    fn line_transition_scales_texture_clip_and_reveal_from_one_anchor() {
        let mut command = DrawCommand {
            texture: TextureSource::Line(0),
            x: 20.0,
            y: 40.0,
            width: 200.0,
            height: 80.0,
            color: [1.0; 4],
            scissor: Some(Scissor {
                x: 40.0,
                y: 50.0,
                width: 100.0,
                height: 30.0,
            }),
            reveal: Some(Reveal {
                edge_x: 100.0,
                rtl: false,
            }),
        };

        transform_line_commands(std::slice::from_mut(&mut command), 20.0, 100.0, 0.9);

        assert!((command.x - 20.0).abs() < f32::EPSILON);
        assert!((command.y - 46.0).abs() < f32::EPSILON);
        assert!((command.width - 180.0).abs() < f32::EPSILON);
        let scissor = command.scissor.expect("scaled clip");
        assert!((scissor.x - 38.0).abs() < f32::EPSILON);
        assert!((scissor.y - 55.0).abs() < f32::EPSILON);
        assert!((scissor.width - 90.0).abs() < f32::EPSILON);
        assert!((command.reveal.expect("scaled reveal").edge_x - 92.0).abs() < f32::EPSILON);
    }

    #[test]
    fn texture_working_set_is_bounded_around_visible_lines() {
        let commands = [DrawCommand {
            texture: TextureSource::Line(10),
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            color: [1.0; 4],
            scissor: None,
            reveal: None,
        }];
        assert_eq!(
            wanted_texture_window(&commands, 100),
            Some(TextureWindow { start: 8, end: 12 })
        );

        let first_line = [DrawCommand {
            texture: TextureSource::Line(0),
            ..commands[0]
        }];
        assert_eq!(
            wanted_texture_window(&first_line, 100),
            Some(TextureWindow { start: 0, end: 2 })
        );
    }

    #[test]
    fn visible_line_lookup_is_logarithmic_and_bounded_to_the_viewport() {
        let geometry_calls = Cell::new(0_usize);
        let range = visible_line_range(10_000, 320_000.0, 44.0, 480, |index| {
            geometry_calls.set(geometry_calls.get() + 1);
            (index as f64 * 64.0, 48.0)
        });

        assert!(
            range.len() <= 10,
            "only viewport-adjacent lines are scanned"
        );
        assert!(range.contains(&5_000));
        assert!(
            geometry_calls.get() <= 32,
            "two binary searches should not become a whole-song scan"
        );
    }

    #[test]
    fn visible_line_lookup_conservatively_covers_interlude_shift() {
        let geometry = [(0.0, 40.0), (60.0, 80.0), (160.0, 40.0), (220.0, 40.0)];
        let range = visible_line_range(geometry.len(), 155.0, 44.0, 60, |index| geometry[index]);

        // Line 1 would be above the viewport without the shared interlude
        // push, but it can still move down by 44 px and must remain eligible.
        assert!(range.contains(&1));
        assert!(range.contains(&2));
        assert!(range.contains(&3));
    }

    #[test]
    fn sidebar_texture_color_never_uses_album_palette() {
        let foreground = (0.8, 0.7, 0.6, 1.0);
        assert_eq!(
            inactive_color(LyricsPresentation::Sidebar, foreground, (1.0, 0.0, 0.0)),
            inactive_color(LyricsPresentation::Sidebar, foreground, (0.0, 0.0, 1.0))
        );
    }

    #[test]
    fn karaoke_command_uses_the_shared_source_range_and_soft_edge() {
        let document = LyricsDocument::new(
            LyricsSource::Yrc,
            vec![
                LyricsLine::new(
                    0,
                    2_000,
                    "逐字歌词",
                    None,
                    TimingQuality::Source,
                    vec![
                        TimedSpan::new(0..6, 0, 1_000),
                        TimedSpan::new(6..12, 1_000, 2_000),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let context = pangocairo::FontMap::default().create_context();
        let mut layouts = layout_document(&context, &document, LyricsPresentation::Sidebar, 400);
        let pango = layouts.remove(0);
        let bitmap = rasterize_line(&pango, 400, 1).unwrap();
        let line = GlLine {
            pango,
            texture_width: bitmap.width,
            texture_height: bitmap.height,
            texture_scale: 1,
        };
        let karaoke = frame_at(&document, 500).karaoke.unwrap();
        let source_rectangle = range_rectangles(&line.pango.layout, 0..6)
            .into_iter()
            .next()
            .expect("first source-authored range");
        let mut commands = Vec::new();

        append_karaoke_commands(&mut commands, 0, &line, &karaoke, 24.0, 48.0, [1.0; 4]);

        assert_eq!(commands.len(), 1);
        let reveal = commands[0].reveal.expect("active range soft reveal");
        assert!(!reveal.rtl);
        let expected_edge = 24.0
            + source_rectangle.x as f32
            + source_rectangle.width as f32 * karaoke.active_span_progress;
        assert!((reveal.edge_x - expected_edge).abs() < 0.01);
        assert!(
            commands[0]
                .scissor
                .is_some_and(|clip| clip.width > source_rectangle.width as f32)
        );
        assert!(FRAGMENT_SHADER.contains("smoothstep"));
        assert!(DESKTOP_FRAGMENT_SHADER.contains("smoothstep"));
    }

    #[test]
    fn source_fixture_matrix_rasterizes_real_pango_masks() {
        let fixtures = [
            RawLyrics {
                lyric: Some(include_str!("../../../../tests/fixtures/lyrics/basic.lrc")),
                translation: None,
                word_synced: None,
                word_translation: None,
                word_source: LyricsSource::Unknown,
                is_pure_music: false,
            },
            RawLyrics {
                lyric: Some(include_str!("../../../../tests/fixtures/lyrics/basic.lrc")),
                translation: None,
                word_synced: Some(include_str!(
                    "../../../../tests/fixtures/lyrics/word-timed.yrc"
                )),
                word_translation: None,
                word_source: LyricsSource::Yrc,
                is_pure_music: false,
            },
            RawLyrics {
                lyric: Some(include_str!("../../../../tests/fixtures/lyrics/basic.lrc")),
                translation: None,
                word_synced: Some(include_str!(
                    "../../../../tests/fixtures/lyrics/word-timed.qrc"
                )),
                word_translation: None,
                word_source: LyricsSource::Qrc,
                is_pure_music: false,
            },
            RawLyrics {
                lyric: Some(include_str!(
                    "../../../../tests/fixtures/lyrics/karaoke.ttml"
                )),
                translation: None,
                word_synced: None,
                word_translation: None,
                word_source: LyricsSource::Ttml,
                is_pure_music: false,
            },
        ];
        let context = pangocairo::FontMap::default().create_context();

        for raw in fixtures {
            let document = parse_document(raw).unwrap().unwrap();
            for (presentation, width) in [
                (LyricsPresentation::Sidebar, 320),
                (LyricsPresentation::Fullscreen, 720),
            ] {
                let layouts = layout_document(&context, &document, presentation, width);
                for (index, line) in layouts.iter().enumerate() {
                    let bitmap = rasterize_line(line, width, 2).unwrap();
                    assert!(
                        bitmap.pixels.iter().any(|coverage| *coverage > 0),
                        "{:?} {presentation:?} line {index} produced an empty Pango mask",
                        document.source
                    );
                    assert!(bitmap.width <= width * 2);
                }
            }
        }
    }

    #[test]
    #[ignore = "manual GL texture memory baseline; run with --release and --nocapture"]
    fn gl_texture_memory_baseline() {
        let lines = (0..300_u64)
            .map(|index| {
                let start = index * 4_000;
                LyricsLine::new(
                    start,
                    start + 3_000,
                    "这是一句用于纹理内存基线的逐字歌词",
                    Some(Arc::from("A compact translated lyric line")),
                    TimingQuality::Line,
                    vec![],
                )
                .unwrap()
            })
            .collect();
        let document = LyricsDocument::new(LyricsSource::Lrc, lines).unwrap();
        let context = pangocairo::FontMap::default().create_context();
        let started = Instant::now();
        let layouts = layout_document(&context, &document, LyricsPresentation::Fullscreen, 600);
        let mut all_compact_bytes = 0_usize;
        let mut full_width_bytes = 0_usize;
        let mut total_texture_width = 0_usize;
        for line in &layouts {
            let (width, height) = bitmap_dimensions(line, 600, 2);
            all_compact_bytes += width as usize * height as usize;
            full_width_bytes += 1_200 * height as usize;
            total_texture_width += width as usize;
        }
        let visible_commands: Vec<_> = (146..=153)
            .map(|index| DrawCommand {
                texture: TextureSource::Line(index),
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                color: [1.0; 4],
                scissor: None,
                reveal: None,
            })
            .collect();
        let wanted = wanted_texture_window(&visible_commands, layouts.len()).unwrap();
        let resident_bytes: usize = wanted
            .indices()
            .map(|index| {
                rasterize_line(&layouts[index], 600, 2)
                    .unwrap()
                    .pixels
                    .len()
            })
            .sum();
        let elapsed = started.elapsed();

        assert!(all_compact_bytes < full_width_bytes);
        assert!(resident_bytes * 10 < all_compact_bytes);
        println!(
            "lyrics GL masks: {} of 300 lines resident at 2x in {:.3}s, {:.2} MiB resident vs {:.2} MiB all compact / {:.2} MiB full-width, {:.0}px average texture width",
            wanted.len(),
            elapsed.as_secs_f64(),
            resident_bytes as f64 / (1024.0 * 1024.0),
            all_compact_bytes as f64 / (1024.0 * 1024.0),
            full_width_bytes as f64 / (1024.0 * 1024.0),
            total_texture_width as f64 / layouts.len() as f64,
        );
    }

    #[test]
    #[ignore = "requires a graphical session and an OpenGL-capable GTK backend"]
    fn live_glarea_compiles_shaders_and_uploads_pango_texture() {
        gtk::init().expect("GTK graphical session");
        let epoch = Instant::now();
        let runtime = Rc::new(RefCell::new(LyricsRuntime::new(Duration::ZERO)));
        let mut source_lines = vec![
            LyricsLine::new(
                0,
                2_000,
                "逐字歌词",
                Some(Arc::from("karaoke")),
                TimingQuality::Source,
                vec![
                    TimedSpan::new(0..6, 0, 1_000),
                    TimedSpan::new(6..12, 1_000, 2_000),
                ],
            )
            .unwrap(),
            LyricsLine::new(7_000, 9_000, "间奏之后", None, TimingQuality::Line, vec![]).unwrap(),
        ];
        source_lines.extend((2..20_u64).map(|index| {
            let start = 7_000 + index * 3_000;
            LyricsLine::new(
                start,
                start + 2_000,
                format!("缓存窗口测试 {index}"),
                None,
                TimingQuality::Line,
                vec![],
            )
            .unwrap()
        }));
        let document = Arc::new(LyricsDocument::new(LyricsSource::Yrc, source_lines).unwrap());
        runtime.borrow_mut().load(document.clone());
        runtime.borrow_mut().seek(3_500, Duration::ZERO);
        runtime.borrow_mut().set_playing(true, Duration::ZERO);

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        let fallback =
            CairoLyricsView::new(runtime.clone(), LyricsPresentation::Sidebar, epoch, |_| {});
        stack.add_named(fallback.widget(), Some("cairo"));

        let failed = Rc::new(Cell::new(false));
        let failed_callback = failed.clone();
        let failure_stack = stack.clone();
        let view = GlLyricsView::new(
            runtime.clone(),
            LyricsPresentation::Sidebar,
            epoch,
            |_| {},
            move || {
                failed_callback.set(true);
                failure_stack.set_visible_child_name("cairo");
            },
        );
        stack.add_named(view.widget(), Some("gl"));
        stack.set_visible_child_name("gl");
        const SNAPSHOT_FOREGROUND: (f64, f64, f64, f64) = (0.12, 0.12, 0.12, 1.0);
        const SNAPSHOT_BACKGROUND: [u8; 3] = [247, 247, 247];
        view.set_text_color(
            SNAPSHOT_FOREGROUND.0,
            SNAPSHOT_FOREGROUND.1,
            SNAPSHOT_FOREGROUND.2,
            SNAPSHOT_FOREGROUND.3,
        );
        fallback.load_document(document.clone());
        view.load_document(document);
        let window = gtk::Window::new();
        window.set_title(Some("Linn Lyrics V2 GL Smoke"));
        window.set_default_size(480, 360);
        window.set_resizable(false);
        window.set_child(Some(&stack));
        window.present();

        let context = gtk::glib::MainContext::default();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline
            && (view.state.borrow().gpu.is_none()
                || view
                    .state
                    .borrow()
                    .gpu
                    .as_ref()
                    .is_some_and(|gpu| gpu.uploaded_generation == u64::MAX))
            && !failed.get()
        {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(!failed.get(), "GL backend fell back to Cairo");
        view.widget().make_current();
        let state = view.state.borrow();
        let gpu = state.gpu.as_ref().expect("GLArea was not realized");
        assert_eq!(gpu.uploaded_generation, state.layout_generation);
        assert!(!gpu.renderer.textures.is_empty());
        let wanted = wanted_texture_window(&state.command_buffer, state.lines.len())
            .expect("visible lyric texture window");
        assert_eq!(gpu.renderer.line_texture_count(), wanted.len());
        assert!(
            wanted
                .indices()
                .all(|index| gpu.renderer.has_line_texture(index)),
            "GPU cache must exactly follow the visible and prefetched line window"
        );
        let generation = state.layout_generation;
        assert!(
            state.lines[0].texture_width < state.cached_width * state.cached_scale,
            "short lines must not allocate a full-width texture"
        );
        let scale = view.widget().scale_factor().max(1);
        let framebuffer_width = view.widget().width().max(1) * scale;
        let framebuffer_height = view.widget().height().max(1) * scale;
        drop(state);

        view.widget().attach_buffers();
        view.state
            .borrow_mut()
            .render(view.widget())
            .expect("render into the attached GLArea framebuffer");
        let state = view.state.borrow();
        let gpu = state.gpu.as_ref().expect("GLArea renderer");
        let dot = state
            .draw_commands(view.widget().height(), (1.0, 1.0, 1.0, 1.0))
            .into_iter()
            .find(|command| matches!(command.texture, TextureSource::Dot))
            .expect("the GL backend must render the shared interlude plan");
        let mut pixels = vec![0_u8; framebuffer_width as usize * framebuffer_height as usize * 4];
        unsafe {
            gpu.gl.finish();
            gpu.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gpu.gl.read_pixels(
                0,
                0,
                framebuffer_width,
                framebuffer_height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        assert!(
            region_has_rgb_signal(&pixels, framebuffer_width, framebuffer_height, scale, dot,),
            "the interlude command must produce visible framebuffer pixels"
        );
        if let Ok(path) = std::env::var("LINN_LYRICS_SMOKE_PNG") {
            save_readback_png(
                &pixels,
                framebuffer_width,
                framebuffer_height,
                SNAPSHOT_BACKGROUND,
                &path,
            )
            .expect("save optional GL smoke snapshot");
        }
        assert_eq!(unsafe { gpu.gl.get_error() }, glow::NO_ERROR);
        let command_buffer_ptr = state.command_buffer.as_ptr();
        let command_buffer_capacity = state.command_buffer.capacity();
        assert!(command_buffer_capacity > 0);
        drop(state);

        view.refresh();
        while context.pending() {
            context.iteration(false);
        }
        assert_eq!(
            view.state.borrow().layout_generation,
            generation,
            "a normal frame must reuse Pango layouts and textures"
        );
        assert_eq!(
            view.state.borrow().command_buffer.as_ptr(),
            command_buffer_ptr,
            "a normal frame must reuse the GL command allocation"
        );
        assert_eq!(
            view.state.borrow().command_buffer.capacity(),
            command_buffer_capacity
        );

        view.widget().make_current();
        view.widget().attach_buffers();
        view.state
            .borrow_mut()
            .render(view.widget())
            .expect("render the stable pre-theme frame");
        let state = view.state.borrow();
        let gpu = state.gpu.as_ref().expect("GLArea renderer");
        let textures_before_theme = gpu.renderer.textures.clone();
        unsafe {
            gpu.gl.finish();
            gpu.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gpu.gl.read_pixels(
                0,
                0,
                framebuffer_width,
                framebuffer_height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        drop(state);
        let dark_rgb_sum: u64 = pixels
            .chunks_exact(4)
            .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
            .sum();
        view.set_text_color(1.0, 1.0, 1.0, 1.0);
        view.widget().make_current();
        view.widget().attach_buffers();
        view.state
            .borrow_mut()
            .render(view.widget())
            .expect("render after a live theme foreground change");
        let state = view.state.borrow();
        let gpu = state.gpu.as_ref().expect("GLArea renderer");
        let mut light_pixels = vec![0_u8; pixels.len()];
        unsafe {
            gpu.gl.finish();
            gpu.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gpu.gl.read_pixels(
                0,
                0,
                framebuffer_width,
                framebuffer_height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut light_pixels)),
            );
        }
        let light_rgb_sum: u64 = light_pixels
            .chunks_exact(4)
            .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
            .sum();
        assert!(
            light_rgb_sum > dark_rgb_sum,
            "a live sidebar theme change must update framebuffer colour"
        );
        assert_eq!(
            state.layout_generation, generation,
            "a colour-only theme change must reuse Pango layouts and line textures"
        );
        assert_eq!(gpu.uploaded_generation, generation);
        assert_eq!(
            gpu.renderer.textures, textures_before_theme,
            "a colour-only theme change must not replace line texture objects"
        );
        assert_eq!(unsafe { gpu.gl.get_error() }, glow::NO_ERROR);
        drop(state);
        view.set_text_color(
            SNAPSHOT_FOREGROUND.0,
            SNAPSHOT_FOREGROUND.1,
            SNAPSHOT_FOREGROUND.2,
            SNAPSHOT_FOREGROUND.3,
        );

        runtime.borrow_mut().set_playing(false, epoch.elapsed());
        let (early_pixels, early_region) = render_test_position(
            &view,
            &runtime,
            epoch,
            250,
            framebuffer_width,
            framebuffer_height,
        );
        let (late_pixels, late_region) = render_test_position(
            &view,
            &runtime,
            epoch,
            750,
            framebuffer_width,
            framebuffer_height,
        );
        let early_signal = region_rgb_sum(
            &early_pixels,
            framebuffer_width,
            framebuffer_height,
            scale,
            early_region,
        );
        let late_signal = region_rgb_sum(
            &late_pixels,
            framebuffer_width,
            framebuffer_height,
            scale,
            late_region,
        );
        assert!(
            late_signal > early_signal,
            "the real GL framebuffer must contain more highlighted RGB coverage at 75% than 25%"
        );

        // Exercise the viewport lookup and GPU cache away from the beginning of
        // the document. This catches a binary-search boundary error that a
        // first-line-only framebuffer test cannot observe.
        runtime.borrow_mut().seek(37_500, epoch.elapsed());
        view.widget().make_current();
        view.widget().attach_buffers();
        let mut state = view.state.borrow_mut();
        let frame = runtime.borrow().frame(epoch.elapsed());
        let focus = frame.as_ref().and_then(|frame| frame.frame.focus_line);
        assert_eq!(focus, Some(10));
        let focus_center = focus.and_then(|index| {
            state
                .lines
                .get(index)
                .map(|line| line.pango.y + line.pango.main_height * 0.5)
        });
        state.viewport.reset();
        state
            .viewport
            .advance(frame, focus_center, Instant::now(), view.widget().height());
        state
            .render(view.widget())
            .expect("render a middle-of-document GL frame");
        assert!(
            state
                .command_buffer
                .iter()
                .any(|command| matches!(command.texture, TextureSource::Line(10))),
            "the focused middle line must survive the bounded visible-range lookup"
        );
        let middle_wanted = wanted_texture_window(&state.command_buffer, state.lines.len())
            .expect("middle lyric texture window");
        assert!(middle_wanted.start > 0);
        let gpu = state.gpu.as_ref().expect("GLArea renderer");
        assert_eq!(gpu.renderer.line_texture_count(), middle_wanted.len());
        assert!(
            middle_wanted
                .indices()
                .all(|index| gpu.renderer.has_line_texture(index)),
            "GPU textures must page to the middle viewport window"
        );
        assert_eq!(unsafe { gpu.gl.get_error() }, glow::NO_ERROR);
        drop(state);

        if let Some(hold_ms) = std::env::var("LINN_LYRICS_SMOKE_HOLD_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            let hold_until = Instant::now() + Duration::from_millis(hold_ms);
            while Instant::now() < hold_until {
                while context.pending() {
                    context.iteration(false);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        view.force_fallback_for_test();
        assert!(
            failed.get(),
            "a GL failure must activate the Cairo fallback"
        );
        assert_eq!(stack.visible_child_name().as_deref(), Some("cairo"));
        assert!(
            runtime.borrow().document().is_some(),
            "fallback activation must preserve the shared lyric document"
        );
        let state = view.state.borrow();
        assert!(state.failed);
        assert!(
            state.gpu.is_none(),
            "the failed GL state must release its GPU cache"
        );
        assert!(
            state.lines.is_empty(),
            "the hidden failed backend must release its Pango layouts"
        );
        assert_eq!(state.cached_width, 0);
        assert_eq!(state.cached_scale, 0);
        drop(state);

        window.close();
        while context.pending() {
            context.iteration(false);
        }
    }

    fn render_test_position(
        view: &GlLyricsView,
        runtime: &Rc<RefCell<LyricsRuntime>>,
        epoch: Instant,
        position_ms: u64,
        framebuffer_width: i32,
        framebuffer_height: i32,
    ) -> (Vec<u8>, Scissor) {
        runtime.borrow_mut().seek(position_ms, epoch.elapsed());
        view.widget().make_current();
        view.widget().attach_buffers();
        let mut state = view.state.borrow_mut();
        let frame = runtime.borrow().frame(epoch.elapsed());
        let focus = frame.as_ref().and_then(|frame| frame.frame.focus_line);
        let focus_center = focus.and_then(|index| {
            state
                .lines
                .get(index)
                .map(|line| line.pango.y + line.pango.main_height * 0.5)
        });
        state.viewport.reset();
        state
            .viewport
            .advance(frame, focus_center, Instant::now(), view.widget().height());
        state
            .render(view.widget())
            .expect("render deterministic karaoke test position");
        let region = state
            .command_buffer
            .iter()
            .find(|command| matches!(command.texture, TextureSource::Line(0)))
            .and_then(|command| command.scissor)
            .expect("active test line must have a visible main-text region");
        let gpu = state.gpu.as_ref().expect("GLArea renderer");
        let mut pixels = vec![0_u8; framebuffer_width as usize * framebuffer_height as usize * 4];
        unsafe {
            gpu.gl.finish();
            gpu.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gpu.gl.read_pixels(
                0,
                0,
                framebuffer_width,
                framebuffer_height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        (pixels, region)
    }

    fn region_rgb_sum(
        pixels: &[u8],
        framebuffer_width: i32,
        framebuffer_height: i32,
        scale: i32,
        region: Scissor,
    ) -> u64 {
        let left = (region.x * scale as f32).floor().max(0.0) as i32;
        let right = ((region.x + region.width) * scale as f32)
            .ceil()
            .min(framebuffer_width as f32) as i32;
        let top = (region.y * scale as f32).floor().max(0.0) as i32;
        let bottom = ((region.y + region.height) * scale as f32)
            .ceil()
            .min(framebuffer_height as f32) as i32;
        (top..bottom)
            .flat_map(|top_down_y| {
                let framebuffer_y = framebuffer_height - 1 - top_down_y;
                (left..right).filter_map(move |x| {
                    let index = ((framebuffer_y * framebuffer_width + x) * 4) as usize;
                    pixels
                        .get(index..index + 3)
                        .map(|rgb| rgb.iter().copied().map(u64::from).sum::<u64>())
                })
            })
            .sum()
    }

    fn region_has_rgb_signal(
        pixels: &[u8],
        framebuffer_width: i32,
        framebuffer_height: i32,
        scale: i32,
        command: DrawCommand,
    ) -> bool {
        let left = (command.x * scale as f32).floor().max(0.0) as i32;
        let right = ((command.x + command.width) * scale as f32)
            .ceil()
            .min(framebuffer_width as f32) as i32;
        let top = (command.y * scale as f32).floor().max(0.0) as i32;
        let bottom = ((command.y + command.height) * scale as f32)
            .ceil()
            .min(framebuffer_height as f32) as i32;
        (top..bottom).any(|top_down_y| {
            let framebuffer_y = framebuffer_height - 1 - top_down_y;
            (left..right).any(|x| {
                let index = ((framebuffer_y * framebuffer_width + x) * 4) as usize;
                pixels
                    .get(index..index + 3)
                    .is_some_and(|rgb| rgb.iter().any(|channel| *channel > 0))
            })
        })
    }

    fn save_readback_png(
        pixels: &[u8],
        width: i32,
        height: i32,
        background: [u8; 3],
        path: &str,
    ) -> Result<(), image::ImageError> {
        let row_bytes = width as usize * 4;
        let mut top_down = vec![0_u8; pixels.len()];
        for row in 0..height as usize {
            let source = row * row_bytes;
            let destination = (height as usize - 1 - row) * row_bytes;
            top_down[destination..destination + row_bytes]
                .copy_from_slice(&pixels[source..source + row_bytes]);
        }
        // GL blends the glyph colour into a transparent framebuffer, so the readback RGB is
        // premultiplied. Composite it onto the fixture's solid sidebar surface before saving.
        for pixel in top_down.chunks_exact_mut(4) {
            let inverse_alpha = 255_u16 - pixel[3] as u16;
            for channel in 0..3 {
                pixel[channel] = (pixel[channel] as u16
                    + background[channel] as u16 * inverse_alpha / 255)
                    .min(255) as u8;
            }
            pixel[3] = 255;
        }
        image::save_buffer(
            path,
            &top_down,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        )
    }
}
