//! 图片缓存工具
//!
//! 提供图片下载和文件缓存功能。

use gtk::glib::object::Cast;
use isahc::AsyncReadResponseExt;
use relm4::gtk;
use std::path::PathBuf;

/// 图片缓存目录
pub fn cache_dir() -> PathBuf {
    let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    path.push("linn");
    path.push("images");
    std::fs::create_dir_all(&path).ok();
    path
}

/// 从 URL 生成缓存文件路径
pub fn cache_path_for_url(url: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    let mut path = cache_dir();
    path.push(format!("{:x}.jpg", hash));
    path
}

/// 异步下载图片数据
async fn download_image(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut response = isahc::get_async(url).await?;
    let body: Vec<u8> = response.bytes().await?.to_vec();
    Ok(body)
}

/// 异步加载图片到本地缓存
pub async fn load_and_cache_image(url: &str) -> Result<PathBuf, String> {
    let cache_path = cache_path_for_url(url);

    // 检查缓存
    if cache_path.exists() {
        return Ok(cache_path);
    }

    // 下载图片
    match download_image(url).await {
        Ok(bytes) => {
            // 保存到缓存
            if let Ok(mut file) = tokio::fs::File::create(&cache_path).await {
                if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut file, &bytes).await {
                    eprintln!("保存图片失败: {:?}", e);
                }
                Ok(cache_path)
            } else {
                Err("无法创建缓存文件".to_string())
            }
        }
        Err(e) => {
            Err(format!("下载图片失败: {:?}", e))
        }
    }
}

/// 从文件路径加载图片为 Texture
pub fn load_image_from_file(path: &PathBuf) -> Option<gtk::gdk::Texture> {
    let file = gtk::gio::File::for_path(path);
    gtk::gdk::Texture::from_file(&file).ok()
}

/// 从 URL 加载图片为 Paintable（异步）
pub async fn load_image_paintable(url: &str) -> Option<gtk::gdk::Paintable> {
    match load_and_cache_image(url).await {
        Ok(cache_path) => load_image_from_file(&cache_path).map(|t| t.upcast::<gtk::gdk::Paintable>()),
        Err(e) => {
            eprintln!("加载图片失败: {}", e);
            None
        }
    }
}
