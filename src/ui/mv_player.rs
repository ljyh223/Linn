//! MV 播放页组件
//!
//! 使用 GStreamer `playbin3` + `gtk4paintablesink` 播放视频，渲染到 `gtk::Picture`。
//! 不走 `gtk::Video`/`gtk::MediaFile`（其内置 GtkGstSink 渲染路径在部分环境存在性能问题）。

use std::cell::Cell;
use std::rc::Rc;

use gst::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::gtk::prelude::*;
use relm4::gtk::{self, gdk, glib};
use relm4::prelude::*;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller};

use crate::api::{Mv, MvDetail, get_mv_comments, get_mv_detail, get_mv_url, get_simi_mv};
use crate::ui::comments::{CommentRow, CommentRowInit};
use crate::ui::components::mv_row::{MvList, MvListInput, MvRowOutput};
use crate::ui::route::AppRoute;

pub struct MvPlayerPage {
    mv_id: u64,
    detail: MvDetail,
    pipeline: gst::Pipeline,
    playbin: gst::Element,
    is_playing: bool,
    position: u64,
    duration: u64,
    progress_scale: gtk::Scale,
    time_label: gtk::Label,
    seek_handler_id: Option<glib::SignalHandlerId>,
    is_seeking: Rc<Cell<bool>>,
    /// 正在进行的 seek 目标位置（毫秒），用于 seek 完成检测
    seek_target: Option<u64>,
    simi_list: Controller<MvList>,
    comments: FactoryVecDeque<CommentRow>,
}

#[derive(Debug)]
pub enum MvPlayerMsg {
    Load(u64),
    Tick,
    TogglePlay,
    Seek(u64),
    SimiMvClicked(u64),
    ArtistClicked,
    /// 占位消息（评论行无输出）
    Noop,
}

#[derive(Debug)]
pub enum MvPlayerCmdMsg {
    Loaded {
        id: u64,
        url: String,
        detail: MvDetail,
        simi: Vec<Mv>,
        comments: Vec<crate::api::Comment>,
    },
    LoadFailed(String),
}

#[derive(Debug)]
pub enum MvPlayerOutput {
    Navigate(AppRoute),
    ShowToast(String),
}

#[relm4::component(pub)]
impl Component for MvPlayerPage {
    type Init = u64;
    type Input = MvPlayerMsg;
    type Output = MvPlayerOutput;
    type CommandOutput = MvPlayerCmdMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,

            // ── 左右结构：左侧视频+评论，右侧详情+相关 MV ──
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 24,
                set_margin_all: 24,
                set_vexpand: true,

                // 左侧：视频 + 控制栏 + 评论
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_hexpand: true,
                    set_vexpand: true,

                    #[name(video_picture)]
                    gtk::Picture {
                        set_hexpand: true,
                        set_height_request: 420,
                        set_overflow: gtk::Overflow::Hidden,
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        add_css_class: "mv-video",
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_halign: gtk::Align::Fill,

                        gtk::Button {
                            #[watch]
                            set_icon_name: if model.is_playing {
                                "media-playback-pause-symbolic"
                            } else {
                                "media-playback-start-symbolic"
                            },
                            add_css_class: "flat",
                            set_size_request: (36, 36),
                            set_tooltip_text: Some("播放/暂停"),
                            connect_clicked => MvPlayerMsg::TogglePlay,
                        },

                        #[name(progress_scale)]
                        gtk::Scale {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_range: (0.0, 100.0),
                            set_draw_value: false,
                            set_hexpand: true,
                            add_css_class: "player-progress",
                        },

                        #[name(time_label)]
                        gtk::Label {
                            set_label: "00:00 / 00:00",
                            add_css_class: "caption",
                            add_css_class: "dim-label",
                        },
                    },

                    // 评论紧贴进度条下方
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_vexpand: true,

                        gtk::Label {
                            set_label: "评论",
                            add_css_class: "heading",
                            set_halign: gtk::Align::Start,
                            set_margin_top: 8,
                            set_margin_bottom: 4,
                        },

                        gtk::ScrolledWindow {
                            set_vexpand: true,
                            set_hscrollbar_policy: gtk::PolicyType::Never,

                            #[local_ref]
                            comments_list -> gtk::ListBox {
                                set_selection_mode: gtk::SelectionMode::None,
                                set_show_separators: true,
                            },
                        },
                    },
                },

                // 右侧：MV 详情 + 相关 MV（拉到最底部）
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_width_request: 360,
                    set_spacing: 8,
                    set_vexpand: true,

                    gtk::Label {
                        #[watch]
                        set_label: &model.detail.name,
                        add_css_class: "title-1",
                        set_wrap: true,
                        set_xalign: 0.0,
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.detail.artists.iter().take(2)
                            .map(|a| a.name.clone())
                            .collect::<Vec<_>>()
                            .join(" / "),
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_selectable: true,
                        add_css_class: "link",
                        add_controller = gtk::GestureClick {
                            connect_released[sender] => move |_, _, _, _| {
                                sender.input(MvPlayerMsg::ArtistClicked);
                            }
                        }
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &format_play_count(model.detail.play_count),
                        add_css_class: "dim-label",
                        set_xalign: 0.0,
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.detail.brief_desc,
                        add_css_class: "caption",
                        set_wrap: true,
                        set_xalign: 0.0,
                    },

                    gtk::Separator {
                        set_margin_top: 8,
                        set_margin_bottom: 8,
                    },

                    gtk::Label {
                        set_label: "相关 MV",
                        add_css_class: "heading",
                        set_halign: gtk::Align::Start,
                        set_margin_bottom: 4,
                    },

                    model.simi_list.widget(),
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let simi_list = MvList::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                MvRowOutput::Clicked(id) => MvPlayerMsg::SimiMvClicked(id),
            });

        let comments = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |_| MvPlayerMsg::Noop);

        // ── GStreamer pipeline ────────────────────────────────
        let pipeline = gst::Pipeline::new();
        let playbin = gst::ElementFactory::make("playbin3")
            .build()
            .expect("playbin3 element not available");
        pipeline.add(&playbin).expect("failed to add playbin3");

        let sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .expect("gtk4paintablesink element not available");
        let paintable: gdk::Paintable = sink.property("paintable");
        playbin.set_property("video-sink", &sink);

        let is_seeking = Rc::new(Cell::new(false));

        let mut model = Self {
            mv_id: init,
            detail: MvDetail::default(),
            pipeline,
            playbin,
            is_playing: false,
            position: 0,
            duration: 0,
            progress_scale: gtk::Scale::default(),
            time_label: gtk::Label::default(),
            seek_handler_id: None,
            is_seeking: is_seeking.clone(),
            seek_target: None,
            simi_list,
            comments,
        };

        let comments_list = model.comments.widget();
        let widgets = view_output!();

        widgets.video_picture.set_paintable(Some(&paintable));

        // 位置轮询：tick 节流到 ~100ms 发一次（随 widget 销毁自动停止）
        let tick_sender = sender.input_sender().clone();
        let last_tick = Rc::new(Cell::new(0i64));
        let last_tick_clone = last_tick.clone();
        widgets.video_picture.add_tick_callback(move |_, clock| {
            let now = clock.frame_time();
            if now - last_tick_clone.get() >= 100_000 {
                last_tick_clone.set(now);
                tick_sender.emit(MvPlayerMsg::Tick);
            }
            gtk::glib::ControlFlow::Continue
        });

        // 进度条拖动
        let scale = widgets.progress_scale.clone();
        let is_seeking_clone = is_seeking.clone();
        let sender_clone = sender.input_sender().clone();
        let seek_handler_id = scale.connect_change_value(move |_, _, val| {
            if !is_seeking_clone.get() {
                sender_clone.emit(MvPlayerMsg::Seek(val as u64));
            }
            gtk::glib::Propagation::Proceed
        });

        model.progress_scale = scale;
        model.time_label = widgets.time_label.clone();
        model.seek_handler_id = Some(seek_handler_id);

        sender.input(MvPlayerMsg::Load(init));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            MvPlayerMsg::Load(id) => {
                self.mv_id = id;
                sender.command(move |out, _shutdown| async move {
                    let url_fut = get_mv_url(id);
                    let detail_fut = get_mv_detail(id);
                    let simi_fut = get_simi_mv(id);
                    let comments_fut = get_mv_comments(id);

                    match futures::future::join4(url_fut, detail_fut, simi_fut, comments_fut).await
                    {
                        (Ok(url), Ok(detail), simi, comments) => {
                            let _ = out.send(MvPlayerCmdMsg::Loaded {
                                id,
                                url,
                                detail,
                                simi: simi.unwrap_or_default(),
                                comments: comments.unwrap_or_default(),
                            });
                        }
                        (Err(e), _, _, _) => {
                            let _ = out.send(MvPlayerCmdMsg::LoadFailed(e.to_string()));
                        }
                        _ => {
                            let _ = out.send(MvPlayerCmdMsg::LoadFailed(
                                "MV 信息加载失败".to_string(),
                            ));
                        }
                    }
                });
            }
            MvPlayerMsg::Tick => {
                // 播放错误（总线消息）
                if let Some(bus) = self.pipeline.bus() {
                    while let Some(msg) = bus.pop() {
                        if let gst::message::MessageView::Error(error) = msg.view() {
                            let err = error.error();
                            sender
                                .output(MvPlayerOutput::ShowToast(err.message().to_string()))
                                .ok();
                        }
                    }
                }

                // 播放状态
                self.is_playing = self.pipeline.current_state() == gst::State::Playing;

                // 位置 / 时长（毫秒）
                self.position = self
                    .playbin
                    .query_position::<gst::ClockTime>()
                    .map_or(0, |t| t.mseconds());
                self.duration = self
                    .playbin
                    .query_duration::<gst::ClockTime>()
                    .map_or(0, |t| t.mseconds());

                // seek 完成检测：当前位置已到达目标附近
                if self.is_seeking.get() && self.seek_target.is_some()
                    && self.position >= self.seek_target.unwrap().saturating_sub(300)
                {
                    self.is_seeking.set(false);
                    self.seek_target = None;
                }

                if self.duration > 0 {
                    self.progress_scale.set_range(0.0, self.duration as f64);
                }
                // seek 期间保留用户拖动的位置，不覆盖
                if !self.is_seeking.get() {
                    if let Some(id) = &self.seek_handler_id {
                        self.progress_scale.block_signal(id);
                    }
                    self.progress_scale.set_value(self.position as f64);
                    if let Some(id) = &self.seek_handler_id {
                        self.progress_scale.unblock_signal(id);
                    }
                }
                self.time_label
                    .set_label(&format_time(self.position, self.duration));
            }
            MvPlayerMsg::TogglePlay => {
                let next = if self.is_playing {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };
                let _ = self.pipeline.set_state(next);
                self.is_playing = next == gst::State::Playing;
            }
            MvPlayerMsg::Seek(ms) => {
                self.is_seeking.set(true);
                self.seek_target = Some(ms);
                // FLUSH：立即清空缓冲快速生效；KEY_UNIT：跳到关键帧（不用 ACCURATE，
                // 否则需精确解码中间帧导致远距离 seek 很慢）。
                // UI 归零问题由 seeking 保护（seek 期间不更新进度条）解决。
                let _ = self.playbin.seek_simple(
                    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                    gst::ClockTime::from_mseconds(ms),
                );
            }
            MvPlayerMsg::SimiMvClicked(id) => {
                sender.input(MvPlayerMsg::Load(id));
            }
            MvPlayerMsg::ArtistClicked => {
                if let Some(artist) = self.detail.artists.first() {
                    sender
                        .output(MvPlayerOutput::Navigate(AppRoute::Artist(artist.id)))
                        .ok();
                }
            }
            MvPlayerMsg::Noop => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            MvPlayerCmdMsg::Loaded {
                id,
                url,
                detail,
                simi,
                comments,
            } => {
                self.mv_id = id;
                self.detail = detail;
                self.position = 0;
                self.duration = 0;
                self.progress_scale.set_value(0.0);
                self.time_label.set_label("00:00 / 00:00");

                // 切换 MV 时先复位，再设置新 URI 并播放
                let _ = self.pipeline.set_state(gst::State::Null);
                self.playbin.set_property("uri", &url);
                let _ = self.pipeline.set_state(gst::State::Playing);
                self.is_playing = true;

                // 相关 MV（ListBox）
                self.simi_list.emit(MvListInput::SetMvs(simi));

                // 评论区（ListBox）
                {
                    let mut guard = self.comments.guard();
                    guard.clear();
                    for c in comments {
                        guard.push_back(CommentRowInit {
                            comment: c,
                            song_id: id,
                        });
                    }
                }
            }
            MvPlayerCmdMsg::LoadFailed(err) => {
                sender.output(MvPlayerOutput::ShowToast(err)).ok();
            }
        }
    }
}

impl Drop for MvPlayerPage {
    fn drop(&mut self) {
        // GStreamer 要求元素销毁前必须回到 NULL 状态，否则 dispose 时会卡死
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

fn format_play_count(count: u64) -> String {
    if count >= 100_000_000 {
        format!("{:.1} 亿次播放", count as f64 / 100_000_000.0)
    } else if count >= 10_000 {
        format!("{:.1} 万次播放", count as f64 / 10_000.0)
    } else {
        format!("{} 次播放", count)
    }
}

fn format_time(position: u64, duration: u64) -> String {
    fn fmt(ms: u64) -> String {
        let secs = ms / 1000;
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
    format!("{} / {}", fmt(position), fmt(duration))
}
