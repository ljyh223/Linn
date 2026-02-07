use relm4::{gtk::prelude::*, gtk};
use std::sync::Arc;
use gtk::glib;

// 全局图片加载器单例
pub struct ImageLoader {
    cache: moka::future::Cache<String, gtk::gdk::Texture>,
}

impl ImageLoader {
    pub fn global() -> &'static Self {
        use once_cell::sync::Lazy;
        static INSTANCE: Lazy<ImageLoader> = Lazy::new(|| ImageLoader {
            // 缓存最多 100 张图片
            cache: moka::future::Cache::builder()
                .max_capacity(100)
                .build(),
        });
        &INSTANCE
    }

    pub async fn load(&self, url: &str) -> Result<gtk::gdk::Texture, Box<dyn std::error::Error>> {
        // 尝试从缓存获取
        if let Some(texture) = self.cache.get(url).await {
            return Ok(texture);
        }

        // 从缓存文件加载
        let cache_path = cache_path_for_url(url);
        if cache_path.exists() {
            if let Ok(file) = gtk::gio::File::for_path(&cache_path).load_contents_future().await {
                if let Ok(bytes) = gtk::glib::Bytes::from(&file.0).try_into() {
                    if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                        self.cache.insert(url.to_string(), texture.clone()).await;
                        return Ok(texture);
                    }
                }
            }
        }

        // 从网络加载
        let mut response = isahc::get_async(url).await?;

        // 确保缓存目录存在
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // 下载数据
        use isahc::AsyncReadResponseExt;
        let buffer = response.bytes().await?.to_vec();

        // 保存到缓存
        std::fs::write(&cache_path, &buffer)?;

        // 创建 texture
        let bytes = gtk::glib::Bytes::from(&buffer[..]);
        let texture = gtk::gdk::Texture::from_bytes(&bytes)?;

        // 加入缓存
        self.cache.insert(url.to_string(), texture.clone()).await;

        Ok(texture)
    }
}

fn cache_path_for_url(url: &str) -> std::path::PathBuf {
    let mut path = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    path.push("linn");
    path.push("images");

    // 使用 URL 的 hash 作为文件名
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());

    // 尝试从 URL 提取扩展名
    let ext = url
        .split('.')
        .last()
        .filter(|e| matches!(*e, "jpg" | "jpeg" | "png" | "webp"))
        .unwrap_or("jpg");

    path.push(format!("{}.{}", hash, ext));
    path
}

/// 异步图片加载组件
///
/// 简单的包装组件，不需要自定义 GtkWidget 子类
#[derive(Debug, Clone)]
pub struct AsyncImage {
    stack: gtk::Stack,
    picture: gtk::Picture,
    spinner: gtk::Spinner,
    placeholder: gtk::Box,
    error_icon: gtk::Image,
    cancellable: std::rc::Rc<std::cell::RefCell<Option<gtk::gio::Cancellable>>>,
}

impl AsyncImage {
    pub fn new() -> Self {
        let stack = gtk::Stack::new();
        let picture = gtk::Picture::new();
        let spinner = gtk::Spinner::new();
        let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let error_icon = gtk::Image::from_icon_name("image-missing-symbolic");

        // 设置 Stack
        stack.set_halign(gtk::Align::Fill);
        stack.set_valign(gtk::Align::Fill);
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        // 占位符（Shimmer 效果）
        placeholder.add_css_class("placeholder");
        placeholder.set_halign(gtk::Align::Fill);
        placeholder.set_valign(gtk::Align::Fill);
        placeholder.set_width_request(100);
        placeholder.set_height_request(100);

        // 错误图标
        error_icon.set_pixel_size(48);
        error_icon.add_css_class("error-image");

        // Spinner
        spinner.start();

        // 添加所有页面到 Stack
        stack.add_child(&placeholder).set_name("placeholder");
        stack.add_child(&spinner).set_name("loading");
        stack.add_child(&picture).set_name("image");
        stack.add_child(&error_icon).set_name("error");

        // 默认显示占位符
        stack.set_visible_child_name("placeholder");

        Self {
            stack,
            picture,
            spinner,
            placeholder,
            error_icon,
            cancellable: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    pub fn set_src(&self, value: Option<&str>) {
        // 取消之前的任务
        if let Some(c) = self.cancellable.borrow_mut().take() {
            c.cancel();
        }

        // 开始加载
        if let Some(url) = value {
            self.start_loading(url.to_string());
        } else {
            self.stack.set_visible_child_name("placeholder");
        }
    }

    pub fn set_width_request(&self, width: i32) {
        self.stack.set_width_request(width);
        self.placeholder.set_width_request(width);
    }

    pub fn set_height_request(&self, height: i32) {
        self.stack.set_height_request(height);
        self.placeholder.set_height_request(height);
    }

    fn start_loading(&self, url: String) {
        let cancellable = gtk::gio::Cancellable::new();
        self.cancellable.replace(Some(cancellable.clone()));

        // 显示 loading
        self.stack.set_visible_child_name("loading");

        // 异步加载
        let stack = self.stack.clone();
        let picture = self.picture.clone();
        let cancellable_ref = self.cancellable.clone();

        gtk::glib::spawn_future_local(async move {
            let loader = ImageLoader::global();

            match loader.load(&url).await {
                Ok(texture) => {
                    // 检查是否被取消
                    if cancellable_ref.borrow().as_ref().map_or(false, |c| c.is_cancelled()) {
                        return;
                    }

                    picture.set_paintable(Some(&texture.upcast::<gtk::gdk::Paintable>()));
                    stack.set_visible_child_name("image");
                }
                Err(e) => {
                    eprintln!("图片加载失败 {}: {}", url, e);
                    stack.set_visible_child_name("error");
                }
            }
        });
    }

    /// 获取内部的 Stack widget（用于添加到父容器）
    pub fn widget(&self) -> &gtk::Stack {
        &self.stack
    }
}

impl Default for AsyncImage {
    fn default() -> Self {
        Self::new()
    }
}

// 实现 Deref 以便可以直接调用 Stack 的方法
impl std::ops::Deref for AsyncImage {
    type Target = gtk::Stack;

    fn deref(&self) -> &Self::Target {
        &self.stack
    }
}

// 实现 AsRef<gtk::Widget> 以便在 Relm4 view! 宏中使用
impl AsRef<gtk::Widget> for AsyncImage {
    fn as_ref(&self) -> &gtk::Widget {
        self.stack.upcast_ref()
    }
}
