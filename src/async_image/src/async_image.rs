use relm4::{gtk::prelude::*, gtk};
use once_cell::sync::Lazy;

// 全局图片加载器单例
pub struct ImageLoader {
    cache: moka::future::Cache<String, gtk::gdk::Texture>,
}

impl ImageLoader {
    pub fn global() -> &'static Self {
        use once_cell::sync::Lazy;
        static INSTANCE: Lazy<ImageLoader> = Lazy::new(|| {
            init_global_css();
            ImageLoader {
                cache: moka::future::Cache::builder().max_capacity(100).build(),
            }
        });
        &INSTANCE
    }

    pub async fn load(&self, url: &str) -> Result<gtk::gdk::Texture, Box<dyn std::error::Error>> {
        if let Some(texture) = self.cache.get(url).await {
            return Ok(texture);
        }

        let cache_path = cache_path_for_url(url);
        if cache_path.exists() {
            if let Ok(file) = gtk::gio::File::for_path(&cache_path).load_contents_future().await {
                let bytes = gtk::glib::Bytes::from(&file.0);
                if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                    self.cache.insert(url.to_string(), texture.clone()).await;
                    return Ok(texture);
                }
            }
        }

        let mut response = isahc::get_async(url).await?;
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        use isahc::AsyncReadResponseExt;
        let buffer = response.bytes().await?.to_vec();
        std::fs::write(&cache_path, &buffer)?;

        let bytes = gtk::glib::Bytes::from(&buffer[..]);
        let texture = gtk::gdk::Texture::from_bytes(&bytes)?;
        self.cache.insert(url.to_string(), texture.clone()).await;

        Ok(texture)
    }
}

fn cache_path_for_url(url: &str) -> std::path::PathBuf {
    let mut path = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    path.push("linn/images");
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());
    let ext = url.split('.').last()
        .filter(|e| matches!(*e, "jpg" | "jpeg" | "png" | "webp"))
        .unwrap_or("jpg");
    path.push(format!("{}.{}", hash, ext));
    path
}

fn init_global_css() {
    static CSS_INITIALIZED: Lazy<std::sync::Mutex<bool>> = Lazy::new(|| std::sync::Mutex::new(false));
    let mut initialized = CSS_INITIALIZED.lock().unwrap();
    if *initialized {
        return;
    }

    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_data(
        "
        .rounded-0px { border-radius: 0px; }
        .rounded-4px { border-radius: 4px; }
        .rounded-8px { border-radius: 8px; }
        .rounded-12px { border-radius: 12px; }
        .rounded-16px { border-radius: 16px; }
        .rounded-20px { border-radius: 20px; }
        .rounded-24px { border-radius: 24px; }
        .rounded-32px { border-radius: 32px; }
        .rounded-48px { border-radius: 48px; }
        .rounded-64px { border-radius: 64px; }

        .async-image-placeholder {
            background: linear-gradient(90deg, @shade_color 0%, @theme_bg_color 50%, @shade_color 100%);
            background-size: 200% 100%;
            animation: async-image-shimmer 1.5s infinite;
        }
        @keyframes async-image-shimmer {
            0% { background-position: 200% 0; }
            100% { background-position: -200% 0; }
        }
        .async-image-error { opacity: 0.5; }
    ",
    );

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    *initialized = true;
}

fn ensure_custom_radius_class(radius: i32) {
    use std::sync::Mutex;
    static CUSTOM_RADII: Lazy<Mutex<std::collections::HashSet<i32>>> =
        Lazy::new(|| Mutex::new(std::collections::HashSet::new()));

    let mut custom = CUSTOM_RADII.lock().unwrap();
    if custom.contains(&radius) {
        return;
    }

    custom.insert(radius);
    let class_name = format!("rounded-custom-{}px", radius);
    let css = format!(".{} {{ border-radius: {}px; }}", class_name, radius);

    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_data(&css);

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

// ============================================================================
// 🎨 简化版：可以直接在 view! 中使用的组件
// ============================================================================

/// 异步图片组件 - 可以直接在 view! 宏中使用
///
/// 基本用法：
/// ```rust
/// view! {
///     gtk::Window {
///         AsyncImage {
///             set_src: "https://example.com/image.jpg",
///             set_width_request: 200,
///             set_height_request: 200,
///             set_border_radius: 16,
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AsyncImage {
    stack: gtk::Stack,
    picture: gtk::Picture,
    cancellable: std::rc::Rc<std::cell::RefCell<Option<gtk::gio::Cancellable>>>,
}

// 实现 GTK4 的子类化，让它可以像原生 widget 一样使用
impl AsyncImage {
    /// 创建默认实例（用于 view! 宏）
    pub fn new() -> Self {
        let _ = ImageLoader::global();

        let stack = gtk::Stack::new();
        let picture = gtk::Picture::new();
        let spinner = gtk::Spinner::new();
        let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let error_icon = gtk::Image::from_icon_name("image-missing-symbolic");

        stack.set_halign(gtk::Align::Fill);
        stack.set_valign(gtk::Align::Fill);
        stack.set_hexpand(false);
        stack.set_vexpand(false);

        placeholder.add_css_class("async-image-placeholder");
        placeholder.set_halign(gtk::Align::Fill);
        placeholder.set_valign(gtk::Align::Fill);

        error_icon.set_pixel_size(48);
        error_icon.add_css_class("async-image-error");
        spinner.start();

        stack.add_child(&placeholder).set_name("placeholder");
        stack.add_child(&spinner).set_name("loading");
        stack.add_child(&picture).set_name("image");
        stack.add_child(&error_icon).set_name("error");
        stack.set_visible_child_name("placeholder");

        Self {
            stack,
            picture,
            cancellable: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    /// 设置图片 URL
    pub fn set_src(&self, url: &str) {
        if let Some(c) = self.cancellable.borrow_mut().take() {
            c.cancel();
        }

        let cancellable = gtk::gio::Cancellable::new();
        self.cancellable.replace(Some(cancellable.clone()));
        self.stack.set_visible_child_name("loading");

        let stack = self.stack.clone();
        let picture = self.picture.clone();
        let cancellable_ref = self.cancellable.clone();
        let url = url.to_string();

        gtk::glib::spawn_future_local(async move {
            match ImageLoader::global().load(&url).await {
                Ok(texture) => {
                    if cancellable_ref.borrow().as_ref().map_or(false, |c| c.is_cancelled()) {
                        return;
                    }
                    picture.set_paintable(Some(&texture.upcast::<gtk::gdk::Paintable>()));
                    stack.set_visible_child_name("image");
                }
                Err(_) => {
                    stack.set_visible_child_name("error");
                }
            }
        });
    }

    /// 设置圆角
    pub fn set_border_radius(&self, radius: i32) {
        for r in [0, 4, 8, 12, 16, 20, 24, 32, 48, 64] {
            self.stack.remove_css_class(&format!("rounded-{}px", r));
            self.picture.remove_css_class(&format!("rounded-{}px", r));
        }

        let class_name = format!("rounded-custom-{}px", radius);
        ensure_custom_radius_class(radius);

        self.stack.add_css_class(&class_name);
        self.picture.add_css_class(&class_name);
        self.stack.set_overflow(gtk::Overflow::Hidden);
        self.picture.set_overflow(gtk::Overflow::Hidden);
    }

    // 重写所有需要的方法，让它可以像 gtk::Widget 一样使用
    pub fn width_request(&self, width: i32) {
        self.stack.set_width_request(width);
    }

    pub fn height_request(&self, height: i32) {
        self.stack.set_height_request(height);
    }

    pub fn halign(&self, align: gtk::Align) {
        self.stack.set_halign(align);
    }

    pub fn valign(&self, align: gtk::Align) {
        self.stack.set_valign(align);
    }

    pub fn hexpand(&self, expand: bool) {
        self.stack.set_hexpand(expand);
    }

    pub fn vexpand(&self, expand: bool) {
        self.stack.set_vexpand(expand);
    }

    // 导出内部的 Stack，用于 append 操作
    pub fn widget(&self) -> &gtk::Widget {
        self.stack.upcast_ref()
    }
}

impl Default for AsyncImage {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 🚀 高级 API - 链式配置
// ============================================================================

/// 链式配置构建器
pub struct AsyncImageBuilder {
    inner: AsyncImage,
    src: Option<String>,
    border_radius: Option<i32>,
}

impl AsyncImageBuilder {
    pub fn new() -> Self {
        Self {
            inner: AsyncImage::new(),
            src: None,
            border_radius: None,
        }
    }

    pub fn src(mut self, url: &str) -> Self {
        self.src = Some(url.to_string());
        self
    }

    pub fn border_radius(mut self, radius: i32) -> Self {
        self.border_radius = Some(radius);
        self
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.inner.width_request(width);
        self.inner.height_request(height);
        self
    }

    pub fn halign(mut self, align: gtk::Align) -> Self {
        self.inner.halign(align);
        self
    }

    pub fn valign(mut self, align: gtk::Align) -> Self {
        self.inner.valign(align);
        self
    }

    pub fn hexpand(mut self, expand: bool) -> Self {
        self.inner.hexpand(expand);
        self
    }

    pub fn vexpand(mut self, expand: bool) -> Self {
        self.inner.vexpand(expand);
        self
    }

    pub fn build(self) -> AsyncImage {
        if let Some(url) = self.src {
            self.inner.set_src(&url);
        }
        if let Some(radius) = self.border_radius {
            self.inner.set_border_radius(radius);
        }
        self.inner
    }
}

impl Default for AsyncImageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 🔧 宏辅助 - 简化 view! 中的使用
// ============================================================================

/// 在 view! 宏中快速创建 AsyncImage
///
/// 用法：
/// ```rust
/// use async_image::async_image;
///
/// view! {
///     gtk::Box {
///         async_image!("https://example.com/image.jpg", size: (200, 200), radius: 16)
///     }
/// }
/// ```
#[macro_export]
macro_rules! async_image {
    ($url:expr) => {{
        let img = $crate::AsyncImage::new();
        img.set_src($url);
        img
    }};

    ($url:expr, size: ($w:expr, $h:expr)) => {{
        let img = $crate::AsyncImage::new();
        img.set_src($url);
        img.width_request($w);
        img.height_request($h);
        img
    }};

    ($url:expr, size: ($w:expr, $h:expr), radius: $r:expr) => {{
        let img = $crate::AsyncImage::new();
        img.set_src($url);
        img.width_request($w);
        img.height_request($h);
        img.set_border_radius($r);
        img
    }};

    ($url:expr, radius: $r:expr) => {{
        let img = $crate::AsyncImage::new();
        img.set_src($url);
        img.set_border_radius($r);
        img
    }};
}

// Deref 和 AsRef 实现，用于在 view! 中使用
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
