//! 全屏歌词页组件
//!
//! 布局：背景层(GLArea) + 水平分割(左封面控制/右歌词) + 右上角关闭按钮

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
.fullscreen-overlay-bg { background-color: #000; }
.fullscreen-content-area { background: transparent; }
";

#[derive(Debug)]
pub enum FullscreenLyricMsg {
    /// 播放时间更新
    TimeUpdated { position: u64, duration: u64 },
    /// 加载歌曲（封面 + 歌词）
    LoadTrack(Song),
    /// 播放状态变化
    UpdatePlayback(bool),
    /// 关闭全屏歌词页
    Close,
    /// 上一首
    PrevTrack,
    /// 下一首
    NextTrack,
    /// 播放/暂停
    TogglePlay,
    /// Seek
    Seek(u64),
    /// 歌词组件 Seek 输出
    LyricsSeek(u64),
    /// 设置喜欢
    SetLiked(bool),
    /// 切换喜欢
    ToggleLike,
}

#[derive(Debug)]
pub enum FullscreenLyricOutput {
    /// 关闭全屏歌词页
    Close,
    /// Seek 请求
    Seek(u64),
    /// 上一首
    PrevTrack,
    /// 下一首
    NextTrack,
    /// 播放/暂停
    TogglePlay,
    /// 切换喜欢
    ToggleLike(u64, bool),
}

/// GL 资源，存储 glow 上下文和渲染器
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
}

#[relm4::component(pub)]
impl SimpleComponent for FullscreenLyricPage {
    type Init = ();
    type Input = FullscreenLyricMsg;
    type Output = FullscreenLyricOutput;

    view! {
        #[root]
        gtk::Overlay {
            // 背景层：GLArea
            #[name(gl_area)]
            gtk::GLArea {
                set_hexpand: true,
                set_vexpand: true,
                set_auto_render: true,
                set_required_version: (3, 3),
            },

            // 主内容层：水平分割（必须作为 overlay 才能在 GLArea 上半透明叠加）
            add_overlay = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Fill,
                add_css_class: "fullscreen-content-box",

                // 左侧：封面 + 歌曲信息 + 进度条 + 控制器
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    set_spacing: 16,
                    set_margin_all: 32,
                    set_opacity: 0.9,

                    // 封面
                    AsyncImage {
                        set_width_request: 320,
                        set_height_request: 320,
                        set_corner_radius: 16.0,
                        #[watch]
                        set_url: format!("{}?param=1000y1000", model.song.cover_url.clone()),
                        set_placeholder_icon: "folder-music-symbolic",
                        add_css_class: "card",
                    },

                    // 歌名 + 歌手
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 4,
                        set_halign: gtk::Align::Center,

                        gtk::Label {
                            #[watch]
                            set_label: &model.song.name,
                            add_css_class: "title-1",
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_max_width_chars: 25,
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &model.song.artists.iter().take(2).map(|a| a.name.clone()).collect::<Vec<_>>().join(" / "),
                            add_css_class: "dim-label",
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_max_width_chars: 25,
                        },
                    },

                    // 进度条
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 4,
                        set_width_request: 300,
                        set_halign: gtk::Align::Center,

                        #[name(progress_scale)]
                        gtk::Scale {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_range: (0.0, 100.0),
                            set_draw_value: false,
                            #[watch]
                            set_value: model.position as f64,
                            set_height_request: 20,
                            set_hexpand: true,
                            add_css_class: "player-progress",
                        },

                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Label {
                                #[watch]
                                set_label: &format_time(model.position),
                                add_css_class: "caption",
                                add_css_class: "dim-label",
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Label {
                                #[watch]
                                set_label: &format_time(model.duration),
                                add_css_class: "caption",
                                add_css_class: "dim-label",
                            }
                        },
                    },

                    // 控制器
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,
                        set_halign: gtk::Align::Center,

                        // 上一首
                        gtk::Button {
                            set_icon_name: "media-skip-backward-symbolic",
                            add_css_class: "flat",
                            set_size_request: (40, 40),
                            connect_clicked => FullscreenLyricMsg::PrevTrack,
                        },

                        // 播放/暂停
                        gtk::Button {
                            #[watch]
                            set_icon_name: if model.is_playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" },
                            add_css_class: "suggested-action",
                            set_size_request: (60, 40),
                            connect_clicked => FullscreenLyricMsg::TogglePlay,
                        },

                        // 下一首
                        gtk::Button {
                            set_icon_name: "media-skip-forward-symbolic",
                            add_css_class: "flat",
                            set_size_request: (40, 40),
                            connect_clicked => FullscreenLyricMsg::NextTrack,
                        },

                        // 喜欢
                        gtk::Button {
                            #[watch]
                            set_icon_name: if model.is_liked { "heart-filled" } else { "heart-outline-thick" },
                            add_css_class: "flat",
                            connect_clicked => FullscreenLyricMsg::ToggleLike,
                        },
                    },
                },

                // 右侧：歌词
                model.lyrics_page.widget() {
                    set_hexpand: true,
                    set_vexpand: true,
                },
            },

            // 右上角关闭按钮
            add_overlay = &gtk::Button {
                set_icon_name: "window-close-symbolic",
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Start,
                set_margin_top: 12,
                set_margin_end: 12,
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

        let lyrics_page = LyricPage::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                LyricsOutput::Seek(ms) => FullscreenLyricMsg::LyricsSeek(ms),
            });

        let gl_state: Rc<RefCell<Option<GlState>>> = Rc::new(RefCell::new(None));

        let provider = gtk::CssProvider::new();
        provider.load_from_data(FULLSCREEN_CSS);
        gtk::StyleContext::add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

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
        };

        let widgets = view_output!();
        model.progress_scale.clone_from(&widgets.progress_scale);

        let gl_area = widgets.gl_area.clone();

        // GLArea realize: 初始化 GL 上下文和渲染器
        let gl_state_clone = gl_state.clone();
        gl_area.connect_realize(move |area| {
            area.make_current();
            if let Some(err) = area.error() {
                eprintln!("GLArea realize error: {:?}", err);
                log::error!("GLArea realize error: {:?}", err);
                return;
            }
            eprintln!("GLArea realized, creating glow context...");
            match create_glow_context() {
                Ok(gl) => {
                    eprintln!("Glow context created, initializing renderer...");
                    let mut renderer = MeshGradientRenderer::new();
                    renderer.initialize(&gl);
                    log::info!("GLArea background renderer initialized, mesh ready: {}", renderer.is_initialized());
                    *gl_state_clone.borrow_mut() = Some(GlState { gl, renderer });
                }
                Err(e) => {
                    eprintln!("Failed to create GL context: {}", e);
                    log::error!("Failed to create GL context: {}", e);
                }
            }
        });

        // GLArea render: 绘制背景
        let gl_state_clone = gl_state.clone();
        gl_area.connect_render(move |area, _ctx| {
            let w = area.width();
            let h = area.height();
            let scale = area.scale_factor();
            // eprintln!("GLArea render: w={} h={} scale={}", w, h, scale);
            let mut state = gl_state_clone.borrow_mut();
            if let Some(ref mut gs) = *state {
                gs.renderer.draw(&gs.gl, w * scale, h * scale);
            }
            gtk::glib::Propagation::Proceed
        });

        // GLArea unrealize: 清理 GL 资源
        let gl_state_clone = gl_state.clone();
        gl_area.connect_unrealize(move |_area| {
            let mut state = gl_state_clone.borrow_mut();
            if let Some(mut gs) = state.take() {
                gs.renderer.cleanup(&gs.gl);
            }
        });

        // Tick callback: 驱动动画循环
        let gl_area_clone = gl_area.clone();
        gl_area.add_tick_callback(move |_, _| {
            // eprint!("GLArea tick callback: ");
            gl_area_clone.queue_draw();
            gtk::glib::ControlFlow::Continue
        });

        // Progress scale signal
        let is_seeking_clone = is_seeking;
        let sender_clone = sender.clone();
        widgets
            .progress_scale
            .connect_change_value(move |_, _, val| {
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
                // 下载封面并更新 GL 背景
                let cover_url = song.cover_url.clone();
                let gl_state = self.gl_state.clone();
                gtk::glib::spawn_future_local(async move {
                    let url = format!("{}?param=320y320", cover_url);
                    match reqwest::get(&url).await {
                        Ok(resp) => {
                            if let Ok(bytes) = resp.bytes().await {
                                // 直接传原始字节，不要在外面解码
                                let mut state = gl_state.borrow_mut();
                                if let Some(ref mut gs) = *state {
                                    gs.renderer.set_album(&gs.gl, &bytes, 0, 0);
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
                let song_id = self.song.id;
                sender
                    .output(FullscreenLyricOutput::ToggleLike(song_id, new_liked))
                    .unwrap();
            }
        }
    }
}

/// 从当前进程已加载的库中查找 eglGetProcAddress
///
/// GTK4 在初始化时已经加载了 libEGL 和 libepoxy，所以不需要手动 dlopen。
/// 使用 RTLD_DEFAULT 在整个进程空间查找符号，避免重复加载库的问题。
fn create_glow_context() -> Result<glow::Context, String> {
    unsafe {
        type EglGetProcAddr = unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_void;

        let egl_get_proc_addr = {
            // 使用 libc::dlsym(RTLD_DEFAULT, ...) 从进程中已加载的所有库中查找符号
            // GTK4 已经链接了 epoxy，而 epoxy 加载了 EGL/GLX，所以 eglGetProcAddress 一定在进程空间中
            let ptr = libc::dlsym(libc::RTLD_DEFAULT, b"eglGetProcAddress\0".as_ptr() as *const std::ffi::c_char);
            if ptr.is_null() {
                return Err("eglGetProcAddress not found in process (is GTK4 using EGL?)".to_string());
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

/// 格式化毫秒为 mm:ss
fn format_time(ms: u64) -> String {
    let total_sec = ms / 1000;
    let mins = total_sec / 60;
    let secs = total_sec % 60;
    format!("{}:{:02}", mins, secs)
}
