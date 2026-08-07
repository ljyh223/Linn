//! 全屏歌词页组件

use std::cell::RefCell;
use std::rc::Rc;

use relm4::gtk::prelude::*;
use relm4::prelude::*;
use relm4::gtk;

use crate::api::Song;
use crate::ui::components::gl_bg::mesh_renderer::MeshGradientRenderer;
use crate::ui::components::image::AsyncImage;
use crate::ui::lyric::{LyricPage, LyricsMsg, LyricsOutput};


pub const FULLSCREEN_CSS: &str = "
/* 左侧面板：所有子组件强制白色，继承到 label / icon */
.left-panel-white,
.left-panel-white label,
.left-panel-white button,
.left-panel-white image {
    color: white;
}
/* flat 按钮 hover 状态轻微高亮而不是灰色 */
.left-panel-white button.flat:hover {
    background: alpha(white, 0.12);
}
.left-panel-white button.flat:active {
    background: alpha(white, 0.20);
}

/* 进度条：纯线条，无小圆点，hover 放大 */
.fullscreen-root .player-progress {
    min-height: 4px;
    padding: 0;
    margin: 0;
}
.fullscreen-root .player-progress trough {
    min-height: 4px;
    margin: 0;
    padding: 0;
    background: alpha(white, 0.20);
    border-radius: 2px;
    border: none;
    outline: none;
}
.fullscreen-root .player-progress highlight {
    background: white;
    border-radius: 2px;
}
.fullscreen-root .player-progress slider {
    min-width: 0;
    min-height: 0;
    opacity: 0;
    padding: 0;
    margin: 0;
}

/* 控制区：半透明胶囊，无原生 osd 阴影 */
.player-controls-capsule {
    background: alpha(@window_fg_color, 0.12);
    border-radius: 999px;
    padding: 8px 16px;
}
.cover-shadow {
    background: transparent;
    box-shadow: 0 20px 60px rgba(0,0,0,0.45), 0 8px 20px rgba(0,0,0,0.28);
    border-radius: 16px;
}
";

// ─── 消息 / 输出类型（与原代码相同，省略重复注释）─────────────────────────────

#[derive(Debug)]
pub enum FullscreenLyricMsg {
    TimeUpdated { position: u64, duration: u64 },
    LoadTrack(Song),
    UpdatePlayback(bool),
    Close,
    PrevTrack,
    NextTrack,
    TogglePlay,
    Seek(u64),
    LyricsSeek(u64),
    SetLiked(bool),
    ToggleLike,
}

#[derive(Debug)]
pub enum FullscreenLyricOutput {
    Close,
    Seek(u64),
    PrevTrack,
    NextTrack,
    TogglePlay,
    ToggleLike(u64, bool),
}

struct GlState {
    gl: glow::Context,
    renderer: MeshGradientRenderer,
}

pub struct FullscreenLyricPage {
    song: Song,
    is_playing: bool,
    is_liked: bool,
    position: u64,
    duration: u64,
    progress_scale: gtk::Scale,
    is_seeking: Rc<std::cell::Cell<bool>>,
    lyrics_page: Controller<LyricPage>,
    gl_state: Rc<RefCell<Option<GlState>>>,
    current_margin: Rc<std::cell::Cell<f64>>,
    target_margin: Rc<std::cell::Cell<f64>>,
    animation_start_time: Rc<std::cell::Cell<Option<u64>>>,
}

#[relm4::component(pub)]
impl SimpleComponent for FullscreenLyricPage {
    type Init = ();
    type Input = FullscreenLyricMsg;
    type Output = FullscreenLyricOutput;

    view! {
        #[root]
        gtk::Overlay {
            add_css_class: "fullscreen-root",
            // GL 背景层
            #[name(gl_area)]
            gtk::GLArea {
                set_hexpand: true,
                set_vexpand: true,
                set_auto_render: true,
                set_required_version: (3, 3),
            },

            // 主内容层
            add_overlay = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_vexpand: true,

                // ── 左侧控制区（固定宽度，约 40%）────────────────────────────
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    set_spacing: 28,
                    set_margin_end: 32,
                    // 固定宽度 → 右侧歌词自然得到剩余 ~60%
                    set_size_request: (420, -1),
                    add_css_class: "left-panel-white",

                    // 1. 封面阴影包装器（负责 box-shadow，本身透明）
                    #[name(cover_shell_box)]
                    gtk::Box {
                        set_width_request: 380,
                        set_height_request: 380,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,

                        #[name(inner_animated_image)]
                        AsyncImage {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_margin_start: 20,
                            set_margin_end: 20,
                            set_margin_top: 20,
                            set_margin_bottom: 20,
                            #[watch]
                            set_url: format!("{}?param=1000y1000", model.song.cover_url.clone()),
                            set_placeholder_icon: "folder-music-symbolic",
                            set_corner_radius: 16.0,
                            set_shadow: true,
                        }
                    },

                    // 2. 歌曲信息 + 操作
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_width_request: 320,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,
                            set_hexpand: true,
                            set_halign: gtk::Align::Start,

                            gtk::Label {
                                #[watch]
                                set_label: &model.song.name,
                                // 不加独立 css class，颜色由父级 left-panel-white 统一继承
                                set_attributes: Some(&title_attrs),
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_max_width_chars: 18,
                                set_xalign: 0.0,
                            },
                            gtk::Label {
                                #[watch]
                                set_label: &model.song.artists.iter().take(2)
                                    .map(|a| a.name.clone())
                                    .collect::<Vec<_>>()
                                    .join(" / "),
                                // 稍微降低不透明度用 alpha，而不是单独 class
                                set_opacity: 0.65,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_max_width_chars: 22,
                                set_xalign: 0.0,
                            },
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_valign: gtk::Align::Center,

                            gtk::Button {
                                #[watch]
                                set_icon_name: if model.is_liked { "heart-filled" } else { "heart-outline-thick" },
                                add_css_class: "flat",
                                add_css_class: "circular",
                                connect_clicked => FullscreenLyricMsg::ToggleLike,
                            },
                            gtk::Button {
                                set_icon_name: "view-more-symbolic",
                                add_css_class: "flat",
                                add_css_class: "circular",
                            },
                        }
                    },

                    // 3. 进度条
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 6,
                        set_width_request: 320,

                        #[name(progress_scale)]
                        gtk::Scale {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_range: (0.0, 100.0),
                            set_draw_value: false,
                            set_hexpand: true,
                            #[watch]
                            set_value: model.position as f64,
                            add_css_class: "player-progress",
                        },

                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Label {
                                #[watch]
                                set_label: &format_time(model.position),
                                add_css_class: "caption",
                                set_opacity: 0.55,
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Label {
                                #[watch]
                                set_label: &format_time(model.duration),
                                add_css_class: "caption",
                                set_opacity: 0.55,
                            }
                        },
                    },

                    // 4. 播放控制胶囊（半透明背景，不用 osd）
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Center,
                        add_css_class: "player-controls-capsule",

                        gtk::Button {
                            set_icon_name: "media-playlist-shuffle-symbolic",
                            add_css_class: "flat",
                            add_css_class: "circular",
                        },
                        gtk::Button {
                            set_icon_name: "media-skip-backward-symbolic",
                            add_css_class: "flat",
                            add_css_class: "circular",
                            connect_clicked => FullscreenLyricMsg::PrevTrack,
                        },
                        // 主播放按钮略大
                        gtk::Button {
                            #[watch]
                            set_icon_name: if model.is_playing {
                                "media-playback-pause-symbolic"
                            } else {
                                "media-playback-start-symbolic"
                            },
                            add_css_class: "circular",
                            set_size_request: (48, 48),
                            connect_clicked => FullscreenLyricMsg::TogglePlay,
                        },
                        gtk::Button {
                            set_icon_name: "media-skip-forward-symbolic",
                            add_css_class: "flat",
                            add_css_class: "circular",
                            connect_clicked => FullscreenLyricMsg::NextTrack,
                        },
                        gtk::Button {
                            set_icon_name: "media-playlist-repeat-symbolic",
                            add_css_class: "flat",
                            add_css_class: "circular",
                        },
                    },
                },

                // ── 右侧歌词（hexpand 自然占满剩余空间）──────────────────────
                model.lyrics_page.widget() {
                    set_hexpand: true,
                    set_vexpand: true,
                },
            },

            // 关闭按钮（右上角 overlay）
            add_overlay = &gtk::Button {
                set_icon_name: "window-close-symbolic",
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Start,
                set_margin_top: 16,
                set_margin_end: 16,
                add_css_class: "circular",
                add_css_class: "osd",
                connect_clicked => FullscreenLyricMsg::Close,
            },
        }
    }

    fn init(
    _init: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
) -> ComponentParts<Self> {
    let is_seeking = Rc::new(std::cell::Cell::new(false));
    let gl_state: Rc<RefCell<Option<GlState>>> = Rc::new(RefCell::new(None));

    let provider = gtk::CssProvider::new();
    provider.load_from_data(FULLSCREEN_CSS);
    gtk::StyleContext::add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let title_attrs = gtk::pango::AttrList::new();
    title_attrs.insert(gtk::pango::AttrFloat::new_scale(1.45));
    title_attrs.insert(gtk::pango::AttrInt::new_weight(gtk::pango::Weight::Heavy));

    let current_margin = Rc::new(std::cell::Cell::new(20.0));
    let target_margin = Rc::new(std::cell::Cell::new(20.0));
    let animation_start_time = Rc::new(std::cell::Cell::new(None));

    let lyrics_page = LyricPage::builder()
        .launch(())
        .forward(sender.input_sender(), |msg| match msg {
            LyricsOutput::Seek(ms) => FullscreenLyricMsg::LyricsSeek(ms),
        });

    lyrics_page.emit(LyricsMsg::SetTextColor(1.0, 1.0, 1.0, 1.0));

    let mut model = Self {
        song: Song::default(),
        is_playing: false,
        is_liked: false,
        position: 0,
        duration: 0,
        progress_scale: gtk::Scale::default(),
        is_seeking: is_seeking.clone(),
        lyrics_page,
        gl_state: gl_state.clone(),
        current_margin: current_margin.clone(),
        target_margin: target_margin.clone(),
        animation_start_time: animation_start_time.clone(),
    };

    let widgets = view_output!();
    model.progress_scale.clone_from(&widgets.progress_scale);

    // ── 封面动画 + 阴影同步 tick callback ──────────────────────────────
    let inner_image = widgets.inner_animated_image.clone();
    let current_margin_clone = current_margin.clone();
    let target_margin_clone = target_margin.clone();
    let start_time_clone = animation_start_time.clone();

    widgets.cover_shell_box.add_tick_callback(move |_, clock| {
        let target = target_margin_clone.get();
        let current = current_margin_clone.get();

        if (current - target).abs() < 0.05 {
            start_time_clone.set(None);
            return gtk::glib::ControlFlow::Continue;
        }

        let frame_time = clock.frame_time() as u64 / 1000;
        if start_time_clone.get().is_none() {
            start_time_clone.set(Some(frame_time));
        }

        let start_time = start_time_clone.get().unwrap();
        let elapsed = (frame_time - start_time) as f64;
        let mut progress = (elapsed / 380.0).min(1.0).max(0.0);
        progress = 1.0 - (1.0 - progress).powi(3);

        let start_margin = if target > 12.0 { 0.0 } else { 20.0 };
        let next_margin = start_margin + (target - start_margin) * progress;
        current_margin_clone.set(next_margin);

        let m = next_margin as i32;
        inner_image.set_margin_start(m);
        inner_image.set_margin_end(m);
        inner_image.set_margin_top(m);
        inner_image.set_margin_bottom(m);

        gtk::glib::ControlFlow::Continue
    });

    // ── GLArea 生命周期 ────────────────────────────────────────────────
    let gl_area = widgets.gl_area.clone();
    let gl_state_clone = gl_state.clone();

    gl_area.connect_realize(move |area| {
        area.make_current();
        if let Some(err) = area.error() {
            log::error!("GLArea realize error: {:?}", err);
            return;
        }
        match create_glow_context() {
            Ok(gl) => {
                let mut renderer = MeshGradientRenderer::new();
                renderer.initialize(&gl);
                log::info!("GLArea background renderer initialized.");
                *gl_state_clone.borrow_mut() = Some(GlState { gl, renderer });
            }
            Err(e) => log::error!("Failed to create GL context: {}", e),
        }
    });

    let gl_state_clone = gl_state.clone();
    gl_area.connect_render(move |area, _ctx| {
        let w = area.width();
        let h = area.height();
        let scale = area.scale_factor();
        let mut state = gl_state_clone.borrow_mut();
        if let Some(ref mut gs) = *state {
            gs.renderer.draw(&gs.gl, w * scale, h * scale);
        }
        gtk::glib::Propagation::Proceed
    });

    let gl_state_clone = gl_state.clone();
    gl_area.connect_unrealize(move |_area| {
        let mut state = gl_state_clone.borrow_mut();
        if let Some(mut gs) = state.take() {
            gs.renderer.cleanup(&gs.gl);
        }
    });

    let gl_area_clone = gl_area.clone();
    gl_area.add_tick_callback(move |_, _| {
        gl_area_clone.queue_draw();
        gtk::glib::ControlFlow::Continue
    });

    // ── 进度条信号 ─────────────────────────────────────────────────────
    let is_seeking_clone = is_seeking;
    let sender_clone = sender.clone();
    widgets.progress_scale.connect_change_value(move |_, _, val| {
        if !is_seeking_clone.get() {
            sender_clone.input(FullscreenLyricMsg::Seek(val as u64));
        }
        gtk::glib::Propagation::Proceed
    });

    ComponentParts { model, widgets }
}
    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            FullscreenLyricMsg::TimeUpdated { position, duration } => {
                self.position = position;
                self.duration = duration;
                self.progress_scale.set_range(0.0, duration as f64);
                self.is_seeking.set(true);
                self.progress_scale.set_value(position as f64);
                self.is_seeking.set(false);
                self.lyrics_page.emit(LyricsMsg::GstTick(position));
            }
            FullscreenLyricMsg::LoadTrack(song) => {
                self.lyrics_page.emit(LyricsMsg::LoadById(song.id));
                let cover_url = song.cover_url.clone();
                let gl_state = self.gl_state.clone();
                let lyrics_sender = self.lyrics_page.sender().clone();
                gtk::glib::spawn_future_local(async move {
                    let url = format!("{}?param=320y320", cover_url);
                    match reqwest::get(&url).await {
                        Ok(resp) => {
                            if let Ok(bytes) = resp.bytes().await {
                                let mut state = gl_state.borrow_mut();
                                if let Some(ref mut gs) = *state {
                                    let (r, g, b) = gs.renderer.set_album(&gs.gl, &bytes, 0, 0);
                                    lyrics_sender.emit(LyricsMsg::SetBgColor(r, g, b));
                                }
                            }
                        }
                        Err(e) => log::error!("Failed to download cover: {}", e),
                    }
                });
                self.song = song;
            }
            FullscreenLyricMsg::UpdatePlayback(is_playing) => {
                self.is_playing = is_playing;
                // 播放 → margin 收到 0（图片顶满），暂停 → margin 展到 24
                self.target_margin.set(if is_playing { 0.0 } else { 20.0 });
                self.animation_start_time.set(None);
            }
            FullscreenLyricMsg::Close => {
                sender.output(FullscreenLyricOutput::Close).unwrap();
            }
            FullscreenLyricMsg::PrevTrack => {
                sender.output(FullscreenLyricOutput::PrevTrack).unwrap();
            }
            FullscreenLyricMsg::NextTrack => {
                sender.output(FullscreenLyricOutput::NextTrack).unwrap();
            }
            FullscreenLyricMsg::TogglePlay => {
                sender.output(FullscreenLyricOutput::TogglePlay).unwrap();
            }
            FullscreenLyricMsg::Seek(val) => {
                self.is_seeking.set(true);
                self.progress_scale.set_value(val as f64);
                self.is_seeking.set(false);
                self.position = val;
                sender.output(FullscreenLyricOutput::Seek(val)).unwrap();
            }
            FullscreenLyricMsg::LyricsSeek(ms) => {
                sender.output(FullscreenLyricOutput::Seek(ms)).unwrap();
            }
            FullscreenLyricMsg::SetLiked(liked) => {
                self.is_liked = liked;
            }
            FullscreenLyricMsg::ToggleLike => {
                let new_liked = !self.is_liked;
                self.is_liked = new_liked;
                sender.output(FullscreenLyricOutput::ToggleLike(self.song.id, new_liked)).unwrap();
            }
        }
    }
}

fn create_glow_context() -> Result<glow::Context, String> {
    unsafe {
        type EglGetProcAddr = unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_void;
        let egl_get_proc_addr = {
            let ptr = libc::dlsym(
                libc::RTLD_DEFAULT,
                b"eglGetProcAddress\0".as_ptr() as *const std::ffi::c_char,
            );
            if ptr.is_null() {
                return Err("eglGetProcAddress not found in process".to_string());
            }
            std::mem::transmute::<*mut std::ffi::c_void, EglGetProcAddr>(ptr)
        };
        let loader = move |name: &str| -> *const std::ffi::c_void {
            let c_name = match std::ffi::CString::new(name) {
                Ok(s) => s,
                Err(_) => return std::ptr::null(),
            };
            egl_get_proc_addr(c_name.as_ptr()) as *const std::ffi::c_void
        };
        Ok(glow::Context::from_loader_function(loader))
    }
}

fn format_time(ms: u64) -> String {
    let total_sec = ms / 1000;
    format!("{}:{:02}", total_sec / 60, total_sec % 60)
}