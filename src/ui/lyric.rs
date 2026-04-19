// component.rs

use pangocairo::pango;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::get_lryic;
use crate::ui::components::lyric_widget::{create_lyrics_widget, LyricsWidgetState};
use crate::ui::model::LyricLine;
use crate::utils::lyric_parse::parse_lyric;

#[derive(Debug)]
pub enum LyricsMsg {
    GstTick(u64),
    LoadLyrics(Vec<LyricLine>),
    LoadById(u64),
}

#[derive(Debug)]
pub enum LyricsOutput {
    Seek(u64),
}

pub struct LyricPage {
    state: Rc<RefCell<LyricsWidgetState>>,
    drawing_area: relm4::gtk::DrawingArea,
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
        let state = Rc::new(RefCell::new(LyricsWidgetState::new()));

        let seek_sender = sender.output_sender().clone();
        let drawing_area = create_lyrics_widget(state.clone(), move |ms| {
            seek_sender.emit(LyricsOutput::Seek(ms));
        });

        root.set_child(Some(&drawing_area));

        let model = Self { state, drawing_area };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LyricsMsg::GstTick(position) => {
                self.state.borrow_mut().update_time(position);
            }

            LyricsMsg::LoadLyrics(lines) => {
                self.load_with_pango(lines);
            }

            LyricsMsg::LoadById(id) => {
                let sender = sender.clone();
                gtk::glib::MainContext::default().spawn_local(async move {
                    match get_lryic(id).await {
                        Ok(lyric) => {
                            if lyric.is_pure_music { return; }
                            if let Some(lines) = parse_lyric(&lyric) {
                                sender.input(LyricsMsg::LoadLyrics(lines));
                            }
                        }
                        Err(e) => log::error!("获取歌词失败: {}", e),
                    }
                });
            }
        }
    }
}

impl LyricPage {
    fn load_with_pango(&self, lines: Vec<LyricLine>) {
        let pango_ctx: pango::Context = self.drawing_area.pango_context();
        let raw_w = self.drawing_area.width();
        let available_width = if raw_w > 0 {
            (raw_w as f64 - 48.0).max(100.0) as i32
        } else {
            300
        };
        self.state.borrow_mut().load_lines(lines, &pango_ctx, available_width);
        self.drawing_area.queue_draw();
    }
}