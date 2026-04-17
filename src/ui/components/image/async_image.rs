use relm4::gtk::{ContentFit, Widget, gdk, gio::Cancellable, glib, prelude::*};
use std::{cell::RefCell, rc::Rc};


/// 占位图 / 错误图来源
#[derive(Clone)]
pub enum ImageSource {
    /// shimmer 骨架屏（默认占位）
    Shimmer,
    /// GTK icon
    Icon { name: String, size: i32 },
    /// 编译进二进制的资源文件
    Resource(String),
    /// 纯色，用 (r, g, b, a) 表示，范围 0.0~1.0
    SolidColor(f32, f32, f32, f32),
    /// 完全自定义 widget
    Custom(Widget),
}

/// 圆角档位
#[derive(Clone, Copy, Debug, Default)]
pub enum BorderRadius {
    #[default]
    None,
    Sm,         // 4px
    Md,         // 8px
    Lg,         // 12px
    Xl,         // 16px
    Full,       // 999px → 圆形
    Px(i32),    // 任意值
}

impl BorderRadius {
    pub fn to_px(self) -> i32 {
        match self {
            Self::None    => 0,
            Self::Sm      => 4,
            Self::Md      => 8,
            Self::Lg      => 12,
            Self::Xl      => 16,
            Self::Full    => 999,
            Self::Px(v)   => v,
        }
    }
}

fn make_source_widget(source: &ImageSource) -> Widget {
    match source {
        ImageSource::Shimmer => super::make_shimmer_widget(),

        ImageSource::Icon { name, size } => {
            let img = relm4::gtk::Image::from_icon_name(name);
            img.set_pixel_size(*size);
            img.set_halign(relm4::gtk::Align::Center);
            img.set_valign(relm4::gtk::Align::Center);
            img.upcast()
        }

        ImageSource::Resource(path) => {
            let img = relm4::gtk::Image::from_resource(path);
            img.set_halign(relm4::gtk::Align::Fill);
            img.set_valign(relm4::gtk::Align::Fill);
            img.upcast()
        }

        ImageSource::SolidColor(r, g, b, a) => {
            let area = relm4::gtk::DrawingArea::new();
            area.set_halign(relm4::gtk::Align::Fill);
            area.set_valign(relm4::gtk::Align::Fill);
            area.set_hexpand(true);
            area.set_vexpand(true);
            let (r, g, b, a) = (*r as f64, *g as f64, *b as f64, *a as f64);
            area.set_draw_func(move |_, cr, w, h| {
                cr.set_source_rgba(r, g, b, a);
                cr.rectangle(0.0, 0.0, w as f64, h as f64);
                let _ = cr.fill();
            });
            area.upcast()
        }

        ImageSource::Custom(w) => w.clone(),
    }
}

#[derive(Clone)]
pub struct AsyncImage {
    pub stack:       relm4::gtk::Stack,
    picture:     relm4::gtk::Picture,
    cancellable: Rc<RefCell<Option<Cancellable>>>,
}

impl AsyncImage {
    pub fn new() -> Self {
        let stack   = relm4::gtk::Stack::new();
        let picture = relm4::gtk::Picture::new();

        stack.set_halign(relm4::gtk::Align::Fill);
        stack.set_valign(relm4::gtk::Align::Fill);
        stack.set_hexpand(false);
        stack.set_vexpand(false);
        // crossfade 过渡，不依赖 CSS
        stack.set_transition_type(relm4::gtk::StackTransitionType::Crossfade);
        stack.set_transition_duration(200);

        picture.set_content_fit(relm4::gtk::ContentFit::Cover);
        picture.set_can_shrink(true);

        // 默认子页
        let placeholder = super::make_shimmer_widget();
        let loading     = {
            let spinner = relm4::gtk::Spinner::new();
            spinner.set_spinning(true);
            spinner.set_halign(relm4::gtk::Align::Center);
            spinner.set_valign(relm4::gtk::Align::Center);
            spinner.upcast::<relm4::gtk::Widget>()
        };
        let error_view  = {
            let img = relm4::gtk::Image::from_icon_name("image-missing-symbolic");
            img.set_pixel_size(32);
            img.set_halign(relm4::gtk::Align::Center);
            img.set_valign(relm4::gtk::Align::Center);
            img.upcast::<relm4::gtk::Widget>()
        };

        stack.add_named(&placeholder, Some("placeholder"));
        stack.add_named(&loading,     Some("loading"));
        stack.add_named(&picture,     Some("image"));
        stack.add_named(&error_view,  Some("error"));
        stack.set_visible_child_name("placeholder");

        Self {
            stack,
            picture,
            cancellable: Rc::new(RefCell::new(None)),
        }
    }

    // ── 替换子页面 ─────────────────────────────────────────────────────────

    fn replace_page(&self, name: &str, source: &ImageSource) {
        if let Some(old) = self.stack.child_by_name(name) {
            self.stack.remove(&old);
        }
        let w = make_source_widget(source);
        self.stack.add_named(&w, Some(name));
    }

    pub fn set_placeholder(&self, source: ImageSource) {
        self.replace_page("placeholder", &source);
    }

    pub fn set_error_view(&self, source: ImageSource) {
        self.replace_page("error", &source);
    }

    // ── 加载 ───────────────────────────────────────────────────────────────

    pub fn set_src(&self, url: impl Into<String>) {
        let url = url.into();

        // 取消上次加载
        if let Some(c) = self.cancellable.borrow_mut().take() {
            c.cancel();
        }

        let cancellable = Cancellable::new();
        *self.cancellable.borrow_mut() = Some(cancellable.clone());
        self.stack.set_visible_child_name("loading");

        let stack = self.stack.clone();
        let picture = self.picture.clone();

        let cancel_ref = self.cancellable.clone();
        let cancel_check = cancellable.clone();

        glib::MainContext::default().spawn_local(async move {
            let result = super::fetch_image_bytes(&url).await;

            if cancel_check.is_cancelled() {
                return;
            }

            if !cancel_ref
                .borrow()
                .as_ref()
                .map_or(false, |c| !c.is_cancelled())
            {
                return;
            }

            match result.and_then(|bytes| {
                let gbytes = glib::Bytes::from_owned(bytes);
                gdk::Texture::from_bytes(&gbytes)
                    .map_err(|e| super::ImageFetchError::Decode(e.to_string()))
            }) {
                Ok(texture) => {
                    picture.set_paintable(Some(texture.upcast_ref::<gdk::Paintable>()));
                    stack.set_visible_child_name("image");
                }
                Err(_) => {
                    stack.set_visible_child_name("error");
                }
            }
        });
    }

    pub fn clear(&self) {
        if let Some(c) = self.cancellable.borrow_mut().take() {
            c.cancel();
        }
        self.picture.set_paintable(None::<&gdk::Paintable>);
        self.stack.set_visible_child_name("placeholder");
    }

    // ── 样式（无 CSS class 暴露）──────────────────────────────────────────

    pub fn set_border_radius(&self, radius: BorderRadius) {
        super::set_border_radius(&self.stack, radius.to_px());
    }

    pub fn set_size(&self, w: i32, h: i32) {
        self.stack.set_width_request(w);
        self.stack.set_height_request(h);
    }

    pub fn set_content_fit(&self, fit: ContentFit) {
        self.picture.set_content_fit(fit);
    }

    pub fn set_halign(&self, a: relm4::gtk::Align) { self.stack.set_halign(a); }
    pub fn set_valign(&self, a: relm4::gtk::Align) { self.stack.set_valign(a); }
    pub fn set_hexpand(&self, v: bool)      { self.stack.set_hexpand(v); }
    pub fn set_vexpand(&self, v: bool)      { self.stack.set_vexpand(v); }

    pub fn widget(&self) -> &relm4::gtk::Widget { self.stack.upcast_ref() }
}

impl Default for AsyncImage { fn default() -> Self { Self::new() } }
impl std::ops::Deref for AsyncImage {
    type Target = relm4::gtk::Stack;
    fn deref(&self) -> &Self::Target { &self.stack }
}