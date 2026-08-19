// lyric.rs — Relm4 组件，包装 LyricWidget（GSK 渲染）

use relm4::gtk::glib::subclass::types::ObjectSubclassIsExt;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::{Song, get_lyric_for_song};
use crate::ui::components::lyric::gsk_widget::LyricWidget;
use crate::ui::components::lyric::lyric_widget::LyricsWidgetState;
use crate::ui::model::LyricLine;
use crate::ui::model::LyricLineKind;
use crate::utils::lyric_parse::parse_lyric;

#[derive(Debug)]
pub enum LyricsMsg {
    GstTick(u64),
    LoadLyrics { song_id: u64, lines: Vec<LyricLine> },
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
    state: Rc<RefCell<LyricsWidgetState>>,
    widget: LyricWidget,
    current_song_id: Option<u64>,
}

#[relm4::component(pub)]
impl SimpleComponent for LyricPage {
    type Input = LyricsMsg;
    type Output = LyricsOutput;
    type Init = ();

    view! {
        relm4::gtk::ScrolledWindow {
            set_hscrollbar_policy: relm4::gtk::PolicyType::Never,
            set_vscrollbar_policy: relm4::gtk::PolicyType::Never,
            set_hexpand: true,
            set_vexpand: true,
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let seek_sender = sender.output_sender().clone();
        let widget = LyricWidget::new(move |ms| {
            seek_sender.emit(LyricsOutput::Seek(ms));
        });

        let state = widget.state();
        root.set_child(Some(&widget));

        let model = Self {
            state,
            widget,
            current_song_id: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LyricsMsg::GstTick(position) => {
                self.widget.update_time(position);
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

            LyricsMsg::SetTextColor(r, g, b, a) => {
                self.widget.set_text_color(r, g, b, a);
            }

            LyricsMsg::LoadBySong(song) => {
                self.current_song_id = Some(song.id);
                log::debug!("[lyrics][ui] loading song_id={}", song.id);
                eprintln!("[lyrics] UI loading song_id={}", song.id);
                let sender = sender.clone();
                relm4::gtk::glib::MainContext::default().spawn_local(async move {
                    match get_lyric_for_song(&song).await {
                        Ok(lyric) => {
                            if lyric.is_pure_music {
                                return;
                            }
                            if let Some(lines) = parse_lyric(&lyric) {
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
                                eprintln!(
                                    "[lyrics] UI loaded song_id={} lines={} verbatim_lines={} verbatim_chars={} sample={:?}",
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
                            } else {
                                log::warn!(
                                    "[lyrics][ui] source returned no parsed lines song_id={}",
                                    song.id
                                );
                                eprintln!("[lyrics] UI parsed zero lines song_id={}", song.id);
                            }
                        }
                        Err(e) => log::error!("获取歌词失败: {}", e),
                    }
                });
            }

            LyricsMsg::PreloadSong(song) => {
                log::debug!("[lyrics][ui] preloading next song_id={}", song.id);
                eprintln!("[lyrics] preloading next song_id={}", song.id);
                relm4::gtk::glib::MainContext::default().spawn_local(async move {
                    match get_lyric_for_song(&song).await {
                        Ok(_) => {
                            log::info!("[lyrics][ui] preloaded song_id={}", song.id);
                            eprintln!("[lyrics] preloaded song_id={}", song.id);
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
                self.widget.set_bg_color(r, g, b);
            }
        }
    }
}

impl LyricPage {
    fn load_with_pango(&self, lines: Vec<LyricLine>) {
        let raw_w = self.widget.width();
        let available_width = if raw_w > 0 {
            (raw_w as f64 - 48.0).max(100.0) as i32
        } else {
            300
        };
        self.widget.load_lines(lines, available_width);
    }
}
