// lyric.rs — Relm4 lyric host with legacy, reference and accelerated backends.

use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use std::{cell::RefCell, rc::Rc};

use crate::api::{Song, get_lyric_for_song};
use crate::lyrics::{
    LyricsDocument, LyricsPresentation, LyricsRuntime, LyricsV2Mode, RawLyrics, parse_document,
};
use crate::ui::components::lyric::{
    cairo_view::CairoLyricsView, gl_view::GlLyricsView, gsk_widget::LyricWidget,
};
use crate::ui::model::LyricLine;
use crate::ui::model::LyricLineKind;
use crate::ui::setting::keys;
use crate::utils::lyric_parse::parse_lyric;

static PROCESS_LYRICS_MODE: OnceLock<LyricsV2Mode> = OnceLock::new();

fn process_lyrics_mode() -> LyricsV2Mode {
    *PROCESS_LYRICS_MODE.get_or_init(|| {
        let experimental_lyrics = relm4::gtk::gio::Settings::new(crate::APPLICATION_ID)
            .boolean(keys::EXPERIMENTAL_LYRICS);
        LyricsV2Mode::from_environment(experimental_lyrics)
    })
}

#[derive(Debug)]
pub enum LyricsMsg {
    GstTick {
        position: u64,
        duration: u64,
    },
    PlaybackChanged(bool),
    ExternalSeek(u64),
    SeekRequested(u64),
    LoadLyrics {
        song_id: u64,
        lines: Vec<LyricLine>,
    },
    LoadV2Lyrics {
        song_id: u64,
        document: Arc<LyricsDocument>,
    },
    LoadBySong(Song),
    PreloadSong(Song),
    SetTextColor(f64, f64, f64, f64),
    SetBgColor(f64, f64, f64),
}

#[derive(Debug)]
pub enum LyricsOutput {
    Seek(u64),
}

pub struct LyricPage {
    legacy_widget: Option<LyricWidget>,
    cairo_widget: Option<CairoLyricsView>,
    gl_widget: Option<GlLyricsView>,
    gl_fallback: Option<CairoLyricsView>,
    current_song_id: Option<u64>,
    v2_mode: LyricsV2Mode,
    v2_runtime: Option<Rc<RefCell<LyricsRuntime>>>,
    v2_epoch: Instant,
}

#[relm4::component(pub)]
impl SimpleComponent for LyricPage {
    type Input = LyricsMsg;
    type Output = LyricsOutput;
    type Init = LyricsPresentation;

    view! {
        relm4::gtk::ScrolledWindow {
            set_hscrollbar_policy: relm4::gtk::PolicyType::Never,
            set_vscrollbar_policy: relm4::gtk::PolicyType::Never,
            set_hexpand: true,
            set_vexpand: true,
        }
    }

    fn init(
        presentation: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let v2_mode = process_lyrics_mode();
        let v2_epoch = Instant::now();
        let v2_runtime = v2_mode
            .runs_v2()
            .then(|| Rc::new(RefCell::new(LyricsRuntime::new(v2_epoch.elapsed()))));
        let (legacy_widget, cairo_widget, gl_widget, gl_fallback) = match v2_mode {
            LyricsV2Mode::Cairo => {
                let seek_sender = sender.input_sender().clone();
                let view = CairoLyricsView::new(
                    v2_runtime
                        .as_ref()
                        .expect("Cairo mode must create the V2 runtime")
                        .clone(),
                    presentation,
                    v2_epoch,
                    move |ms| seek_sender.emit(LyricsMsg::SeekRequested(ms)),
                );
                root.set_child(Some(view.widget()));
                (None, Some(view), None, None)
            }
            LyricsV2Mode::OpenGl => {
                let runtime = v2_runtime
                    .as_ref()
                    .expect("OpenGL mode must create the V2 runtime")
                    .clone();
                let stack = relm4::gtk::Stack::new();
                stack.set_hexpand(true);
                stack.set_vexpand(true);
                let fallback_sender = sender.input_sender().clone();
                let fallback =
                    CairoLyricsView::new(runtime.clone(), presentation, v2_epoch, move |ms| {
                        fallback_sender.emit(LyricsMsg::SeekRequested(ms))
                    });
                stack.add_named(fallback.widget(), Some("cairo"));

                let gl_sender = sender.input_sender().clone();
                let fallback_stack = stack.clone();
                let view = GlLyricsView::new(
                    runtime,
                    presentation,
                    v2_epoch,
                    move |ms| gl_sender.emit(LyricsMsg::SeekRequested(ms)),
                    move || fallback_stack.set_visible_child_name("cairo"),
                );
                stack.add_named(view.widget(), Some("gl"));
                stack.set_visible_child_name("gl");
                root.set_child(Some(&stack));
                (None, None, Some(view), Some(fallback))
            }
            LyricsV2Mode::Legacy | LyricsV2Mode::Shadow => {
                let seek_sender = sender.input_sender().clone();
                let widget = LyricWidget::new(move |ms| {
                    seek_sender.emit(LyricsMsg::SeekRequested(ms));
                });
                root.set_child(Some(&widget));
                (Some(widget), None, None, None)
            }
        };

        if v2_mode.runs_v2() {
            log::info!("[lyrics][v2] mode={v2_mode:?} presentation={presentation:?}");
        }

        if v2_mode == LyricsV2Mode::Shadow
            && let Some(runtime) = &v2_runtime
            && let Some(widget) = &legacy_widget
        {
            let runtime = runtime.clone();
            let legacy_state = widget.state();
            let epoch = v2_epoch;
            let last_focus = Rc::new(std::cell::Cell::new(None));
            widget.add_tick_callback(move |_, _| {
                let Some(plan) = runtime.borrow().frame(epoch.elapsed()) else {
                    return relm4::gtk::glib::ControlFlow::Continue;
                };
                if last_focus.get() != plan.frame.focus_line {
                    let legacy = legacy_state.borrow().active_line_index();
                    log::debug!(
                        "[lyrics][v2][shadow] position={} focus={:?} active={:?} legacy={:?}",
                        plan.position_ms,
                        plan.frame.focus_line,
                        plan.frame.active_line,
                        legacy
                    );
                    last_focus.set(plan.frame.focus_line);
                }
                relm4::gtk::glib::ControlFlow::Continue
            });
        }

        let model = Self {
            legacy_widget,
            cairo_widget,
            gl_widget,
            gl_fallback,
            current_song_id: None,
            v2_mode,
            v2_runtime,
            v2_epoch,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LyricsMsg::GstTick { position, duration } => {
                if let Some(widget) = &self.legacy_widget {
                    widget.update_time(position);
                }
                if let Some(runtime) = &self.v2_runtime {
                    runtime.borrow_mut().observe_position(
                        position,
                        Some(duration),
                        self.v2_epoch.elapsed(),
                    );
                }
                if let Some(widget) = &self.cairo_widget {
                    widget.refresh();
                }
                self.refresh_active_gl_view();
            }

            LyricsMsg::PlaybackChanged(playing) => {
                if let Some(runtime) = &self.v2_runtime {
                    runtime
                        .borrow_mut()
                        .set_playing(playing, self.v2_epoch.elapsed());
                }
                if let Some(widget) = &self.cairo_widget {
                    widget.refresh();
                }
                self.refresh_active_gl_view();
            }

            LyricsMsg::ExternalSeek(position) => {
                if let Some(runtime) = &self.v2_runtime {
                    runtime.borrow_mut().seek(position, self.v2_epoch.elapsed());
                }
                self.sync_v2_after_seek(false);
            }

            LyricsMsg::SeekRequested(position) => {
                if let Some(runtime) = &self.v2_runtime {
                    runtime.borrow_mut().seek(position, self.v2_epoch.elapsed());
                }
                self.sync_v2_after_seek(true);
                sender.output(LyricsOutput::Seek(position)).ok();
            }

            LyricsMsg::LoadLyrics { song_id, lines } => {
                if self.current_song_id == Some(song_id) {
                    self.load_with_pango(lines);
                } else {
                    log::debug!(
                        "[lyrics][ui] ignored stale lyric result song_id={} current_song_id={:?}",
                        song_id,
                        self.current_song_id
                    );
                }
            }

            LyricsMsg::LoadV2Lyrics { song_id, document } => {
                if self.current_song_id == Some(song_id)
                    && let Some(runtime) = &self.v2_runtime
                {
                    log::info!(
                        "[lyrics][v2] loaded song_id={} source={:?} lines={}",
                        song_id,
                        document.source,
                        document.lines.len()
                    );
                    runtime.borrow_mut().load(document.clone());
                    if let Some(widget) = &self.cairo_widget {
                        widget.load_document(document.clone());
                    }
                    if let Some(widget) = &self.gl_widget {
                        widget.load_document(document.clone());
                    }
                    if let Some(widget) = &self.gl_fallback {
                        widget.load_document(document);
                    }
                }
            }

            LyricsMsg::SetTextColor(r, g, b, a) => {
                if let Some(widget) = &self.legacy_widget {
                    widget.set_text_color(r, g, b, a);
                }
                if let Some(widget) = &self.cairo_widget {
                    widget.set_text_color(r, g, b, a);
                }
                if let Some(widget) = &self.gl_widget {
                    widget.set_text_color(r, g, b, a);
                }
                if let Some(widget) = &self.gl_fallback {
                    widget.set_text_color(r, g, b, a);
                }
            }

            LyricsMsg::LoadBySong(song) => {
                self.current_song_id = Some(song.id);
                // Do not keep the previous song visible while a new request is
                // in flight (or forever when the new track is pure music).
                self.load_with_pango(Vec::new());
                if let Some(runtime) = &self.v2_runtime {
                    let mut runtime = runtime.borrow_mut();
                    runtime.clear();
                    runtime.seek(0, self.v2_epoch.elapsed());
                }
                if let Some(widget) = &self.cairo_widget {
                    widget.clear();
                }
                if let Some(widget) = &self.gl_widget {
                    widget.clear();
                }
                if let Some(widget) = &self.gl_fallback {
                    widget.clear();
                }
                log::debug!("[lyrics][ui] loading song_id={}", song.id);
                let sender = sender.clone();
                let run_v2 = self.v2_mode.runs_v2();
                let run_legacy = self.v2_mode.runs_legacy();
                relm4::gtk::glib::MainContext::default().spawn_local(async move {
                    match get_lyric_for_song(&song).await {
                        Ok(selected) => {
                            let lyric = selected.detail;
                            if lyric.is_pure_music {
                                return;
                            }
                            if run_v2 {
                                match parse_document(RawLyrics {
                                    lyric: lyric.lyric.as_deref(),
                                    translation: lyric.tlyric.as_deref(),
                                    word_synced: lyric.yrc.as_deref(),
                                    word_translation: lyric.ytlrc.as_deref(),
                                    word_source: selected.source,
                                    is_pure_music: lyric.is_pure_music,
                                }) {
                                    Ok(Some(document)) => sender.input(LyricsMsg::LoadV2Lyrics {
                                        song_id: song.id,
                                        document: Arc::new(document),
                                    }),
                                    Ok(None) => log::debug!("[lyrics][v2] no parsed document song_id={}", song.id),
                                    Err(error) => log::warn!("[lyrics][v2] parse failed song_id={} error={error}", song.id),
                                }
                            }
                            if run_legacy && let Some(lines) = parse_lyric(&lyric) {
                                let verbatim_lines = lines
                                    .iter()
                                    .filter(|line| matches!(&line.kind, LyricLineKind::Verbatim(_)))
                                    .count();
                                let verbatim_chars: usize = lines
                                    .iter()
                                    .map(|line| match &line.kind {
                                        LyricLineKind::Verbatim(chars) => chars.len(),
                                        LyricLineKind::Plain => 0,
                                    })
                                    .sum();
                                let sample = lines
                                    .iter()
                                    .find_map(|line| match &line.kind {
                                        LyricLineKind::Verbatim(chars) if !chars.is_empty() => {
                                            Some(
                                                chars
                                                    .iter()
                                                    .take(3)
                                                    .map(|ch| format!("{}@{}+{}", ch.ch, ch.start, ch.duration))
                                                    .collect::<Vec<_>>()
                                                    .join(","),
                                            )
                                        }
                                        _ => None,
                                    })
                                    .unwrap_or_else(|| "none".into());
                                log::info!(
                                    "[lyrics][ui] loaded song_id={} lines={} verbatim_lines={} verbatim_chars={} sample={:?}",
                                    song.id,
                                    lines.len(),
                                    verbatim_lines,
                                    verbatim_chars,
                                    sample
                                );
                                sender.input(LyricsMsg::LoadLyrics {
                                    song_id: song.id,
                                    lines,
                                });
                            } else if run_legacy {
                                log::warn!(
                                    "[lyrics][ui] source returned no parsed lines song_id={}",
                                    song.id
                                );
                            }
                        }
                        Err(e) => log::error!("获取歌词失败: {}", e),
                    }
                });
            }

            LyricsMsg::PreloadSong(song) => {
                log::debug!("[lyrics][ui] preloading next song_id={}", song.id);
                relm4::gtk::glib::MainContext::default().spawn_local(async move {
                    match get_lyric_for_song(&song).await {
                        Ok(_) => {
                            log::info!("[lyrics][ui] preloaded song_id={}", song.id);
                        }
                        Err(error) => {
                            log::debug!(
                                "[lyrics][ui] preload failed song_id={} error={error}",
                                song.id
                            );
                        }
                    }
                });
            }

            LyricsMsg::SetBgColor(r, g, b) => {
                if let Some(widget) = &self.legacy_widget {
                    widget.set_bg_color(r, g, b);
                }
                if let Some(widget) = &self.cairo_widget {
                    widget.set_album_color(r, g, b);
                }
                if let Some(widget) = &self.gl_widget {
                    widget.set_album_color(r, g, b);
                }
                if let Some(widget) = &self.gl_fallback {
                    widget.set_album_color(r, g, b);
                }
            }
        }
    }
}

impl LyricPage {
    fn sync_v2_after_seek(&self, animate: bool) {
        if let Some(widget) = &self.cairo_widget {
            if animate {
                widget.animate_after_seek();
            } else {
                widget.sync_after_seek();
            }
        }
        if let Some(widget) = &self.gl_widget {
            if animate {
                widget.animate_after_seek();
            } else {
                widget.sync_after_seek();
            }
        }
        if let Some(widget) = &self.gl_fallback {
            if animate {
                widget.animate_after_seek();
            } else {
                widget.sync_after_seek();
            }
        }
    }

    fn refresh_active_gl_view(&self) {
        if let Some(widget) = &self.gl_widget {
            widget.refresh();
        }
        // The fallback keeps document, colour and seek state synchronized while
        // hidden, but its own tick callback drives drawing once the stack makes
        // it visible. Queuing it on every player tick would keep a second
        // rendering backend needlessly active in the normal GL path.
    }

    fn load_with_pango(&self, lines: Vec<LyricLine>) {
        let Some(widget) = &self.legacy_widget else {
            return;
        };
        let raw_w = widget.width();
        let available_width = if raw_w > 0 {
            (raw_w as f64 - 48.0).max(100.0) as i32
        } else {
            300
        };
        widget.load_lines(lines, available_width);
    }
}
