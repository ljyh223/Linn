// async_image.rs

use std::cell::RefCell;
use std::rc::Rc;

use relm4::gtk::{
    self, gdk,
    gio::{Cancellable, prelude::CancellableExt},
    glib::{
        self,
        object::{Cast, IsA, ObjectExt},
    },
    prelude::WidgetExt,
};

use super::image_cache::{ImageFetchError, fetch_image_bytes};

// ── 占位图配置 ────────────────────────────────────────────────────────────────

/// 占位图来源（占位和错误状态复用此类型）
#[derive(Clone)]
pub enum ImageSource {
    /// GTK icon name，如 "image-missing-symbolic"
    Icon { name: String, size: i32 },
    /// 资源路径（编译进二进制的资源）
    Resource(String),
    /// 纯色填充
    Color,
    /// shimmer 骨架屏动画（默认）
    Shimmer,
}

impl Default for ImageSource {
    fn default() -> Self {
        Self::Shimmer
    }
}

// ── 圆角工具 ──────────────────────────────────────────────────────────────────

/// 预设圆角档位
#[derive(Clone, Copy, Debug)]
pub enum BorderRadius {
    None,
    Sm,   // 4px
    Md,   // 8px
    Lg,   // 12px
    Xl,   // 16px
    Full, // 9999px → 圆形
    Custom(i32),
}

impl BorderRadius {
    fn px(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Sm => 4,
            Self::Md => 8,
            Self::Lg => 12,
            Self::Xl => 16,
            Self::Full => 9999,
            Self::Custom(v) => v,
        }
    }
}

// ── CSS 初始化 (一次) ─────────────────────────────────────────────────────────

fn ensure_css() {
    use std::sync::OnceLock;

    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let css = gtk::CssProvider::new();
        css.load_from_data(
            r#"
            .async-img-shimmer {
                background: linear-gradient(
                    90deg,
                    alpha(@theme_bg_color, 0.8) 0%,
                    alpha(@theme_fg_color, 0.06) 50%,
                    alpha(@theme_bg_color, 0.8) 100%
                );
                background-size: 200% 100%;
                animation: shimmer 1.6s ease-in-out infinite;
            }
            @keyframes shimmer {
                0%   { background-position: 200% 0; }
                100% { background-position: -200% 0; }
            }
            .async-img-error {
                opacity: 0.45;
            }
            .async-img-picture {
                /* picture 本身不裁剪，由外层 stack overflow: hidden 处理 */
            }
            "#,
        );
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // 动态圆角 CSS 通过 inline style 注入，无需预生成类名
    });
}

// ── 核心组件 ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AsyncImage {
    pub stack: gtk::Stack,
    pub picture: gtk::Picture,
    pub cancellable: Rc<RefCell<Option<Cancellable>>>,
}

impl AsyncImage {
    pub fn new() -> Self {
        ensure_css();

        let stack = gtk::Stack::new();
        let picture = gtk::Picture::new();

        stack.set_halign(gtk::Align::Fill);
        stack.set_valign(gtk::Align::Fill);
        stack.set_hexpand(false);
        stack.set_vexpand(false);
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.set_transition_duration(180);

        // picture.set_content_fit(gtk::ContentFit::Cover);
        picture.add_css_class("async-img-picture");

        // 默认子页面（可被 set_placeholder / set_error_view 替换）
        let placeholder = Self::make_shimmer();
        let loading = Self::make_loading_spinner();
        let error_view = Self::make_default_error();

        stack.add_named(&placeholder, Some("placeholder"));
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&picture, Some("image"));
        stack.add_named(&error_view, Some("error"));
        stack.set_visible_child_name("placeholder");

        Self {
            stack,
            picture,
            cancellable: Rc::new(RefCell::new(None)),
        }
    }

    // ── 子页面工厂 ─────────────────────────────────────────────────────────

    fn make_shimmer() -> gtk::Widget {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
        b.set_halign(gtk::Align::Fill);
        b.set_valign(gtk::Align::Fill);
        b.add_css_class("async-img-shimmer");
        b.upcast()
    }

    fn make_loading_spinner() -> gtk::Widget {
        let spinner = gtk::Spinner::new();
        spinner.set_spinning(true);
        spinner.set_halign(gtk::Align::Center);
        spinner.set_valign(gtk::Align::Center);
        spinner.upcast()
    }

    fn make_default_error() -> gtk::Widget {
        let img = gtk::Image::from_icon_name("image-missing-symbolic");
        img.set_pixel_size(32);
        img.add_css_class("async-img-error");
        img.set_halign(gtk::Align::Center);
        img.set_valign(gtk::Align::Center);
        img.upcast()
    }

    fn make_widget_from_source(source: &ImageSource) -> gtk::Widget {
        match source {
            ImageSource::Shimmer => Self::make_shimmer(),
            ImageSource::Color => {
                let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
                b.set_halign(gtk::Align::Fill);
                b.set_valign(gtk::Align::Fill);
                b.upcast()
            }
            ImageSource::Icon { name, size } => {
                let img = gtk::Image::from_icon_name(name);
                img.set_pixel_size(*size);
                img.set_halign(gtk::Align::Center);
                img.set_valign(gtk::Align::Center);
                img.upcast()
            }
            ImageSource::Resource(path) => {
                let img = gtk::Image::from_resource(path);
                img.set_halign(gtk::Align::Fill);
                img.set_valign(gtk::Align::Fill);
                img.upcast()
            }
        }
    }

    // ── 公开配置 API ───────────────────────────────────────────────────────

    /// 替换占位图（在 set_src 之前调用）
    pub fn set_placeholder(&self, source: ImageSource) {
        let w = Self::make_widget_from_source(&source);
        // 移除旧的，换上新的
        if let Some(old) = self.stack.child_by_name("placeholder") {
            self.stack.remove(&old);
        }
        self.stack.add_named(&w, Some("placeholder"));
    }

    /// 替换错误视图
    pub fn set_error_view(&self, source: ImageSource) {
        let w = Self::make_widget_from_source(&source);
        if let Some(old) = self.stack.child_by_name("error") {
            self.stack.remove(&old);
        }
        self.stack.add_named(&w, Some("error"));
    }

    /// 设置图片 URL，触发加载（可重复调用，会取消上一次）
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
            let result = fetch_image_bytes(&url).await;

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
                    .map_err(|e| ImageFetchError::Decode(e.to_string()))
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

    /// 清除图片，回到占位图状态
    pub fn clear(&self) {
        if let Some(c) = self.cancellable.borrow_mut().take() {
            c.cancel();
        }
        self.picture.set_paintable(None::<&gdk::Paintable>);
        self.stack.set_visible_child_name("placeholder");
    }

    // ── 样式 API ───────────────────────────────────────────────────────────

    pub fn set_border_radius(&self, radius: BorderRadius) {
        let px = radius.px();
        let css = format!("border-radius: {}px; overflow: hidden;", px);
        self.stack.set_css_classes(&[]);
        // 通过 inline style provider per-widget 注入（不污染全局）
        apply_inline_style(&self.stack, &css);
    }

    pub fn set_size(&self, width: i32, height: i32) {
        self.stack.set_width_request(width);
        self.stack.set_height_request(height);
    }

    // pub fn set_content_fit(&self, fit: gtk::ContentFit) {
    //     self.picture.set_content_fit(fit);
    // }

    pub fn set_halign(&self, align: gtk::Align) {
        self.stack.set_halign(align);
    }
    pub fn set_valign(&self, align: gtk::Align) {
        self.stack.set_valign(align);
    }
    pub fn set_hexpand(&self, v: bool) {
        self.stack.set_hexpand(v);
    }
    pub fn set_vexpand(&self, v: bool) {
        self.stack.set_vexpand(v);
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.stack.upcast_ref()
    }
}

/// 为单个 widget 注入 inline CSS（使用 per-widget CssProvider）
fn apply_inline_style(widget: &impl IsA<gtk::Widget>, css: &str) {
    let provider = gtk::CssProvider::new();
    // 使用 widget 自身的 name 作为选择器会污染其他同名控件
    // 更稳妥的做法是给 widget 加一个唯一 CSS class
    // 这里用一个简单稳定的方案：直接给 stack 设置 overflow 和 border-radius
    // 因为 GTK4 支持在代码里设置这些属性
    widget.as_ref().set_overflow(gtk::Overflow::Hidden);
    provider.load_from_data(&format!(
        "* {{ border-radius: {}; }}",
        css.split(':')
            .nth(1)
            .unwrap_or("0px")
            .trim()
            .trim_end_matches(';')
    ));
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    // 将 provider 绑定到 widget 生命周期
    unsafe {
        widget.as_ref().set_data("css-provider", provider);
    }
}

impl Default for AsyncImage {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for AsyncImage {
    type Target = gtk::Stack;
    fn deref(&self) -> &Self::Target {
        &self.stack
    }
}

impl AsRef<gtk::Widget> for AsyncImage {
    fn as_ref(&self) -> &gtk::Widget {
        self.stack.upcast_ref()
    }
}
