// lyric.rs — Relm4 组件，包装 LyricWidget（GSK 渲染）

use relm4::gtk::glib::subclass::types::ObjectSubclassIsExt;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::get_lryic;
use crate::ui::components::lyric::gsk_widget::LyricWidget;
use crate::ui::components::lyric::lyric_widget::LyricsWidgetState;
use crate::ui::model::LyricLine;
use crate::utils::lyric_parse::parse_lyric;

#[derive(Debug)]
pub enum LyricsMsg {
    GstTick(u64),
    LoadLyrics(Vec<LyricLine>),
    LoadById(u64),
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

        let model = Self { state, widget };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LyricsMsg::GstTick(position) => {
                self.widget.update_time(position);
            }

            LyricsMsg::LoadLyrics(lines) => {
                self.load_with_pango(lines);
            }

            LyricsMsg::SetTextColor(r, g, b, a) => {
                self.widget.set_text_color(r, g, b, a);
            }

            LyricsMsg::LoadById(id) => {
                let sender = sender.clone();
                relm4::gtk::glib::MainContext::default().spawn_local(async move {
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