// image_cache.rs
use std::path::PathBuf;
use std::hash::{Hash, Hasher, DefaultHasher};
use once_cell::sync::Lazy;

static CACHE: Lazy<moka::future::Cache<String, Vec<u8>>> = Lazy::new(|| {
    moka::future::Cache::builder()
        .max_capacity(128)
        .build()
});

fn disk_path(url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());
    let ext = url.rsplit('.').next()
        .filter(|e| matches!(*e, "jpg" | "jpeg" | "png" | "webp" | "gif"))
        .unwrap_or("jpg");

    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("linn/images")
        .join(format!("{}.{}", hash, ext))
}

/// 三级缓存：内存 → 磁盘 → 网络
pub async fn fetch_image_bytes(url: &str) -> Result<Vec<u8>, ImageFetchError> {
    // L1: 内存
    if let Some(bytes) = CACHE.get(url).await {
        return Ok(bytes);
    }

    // L2: 磁盘
    let path = disk_path(url);
    if let Ok(bytes) = tokio::fs::read(&path).await {
        CACHE.insert(url.to_string(), bytes.clone()).await;
        return Ok(bytes);
    }

    // L3: 网络
    let bytes = download(url).await?;

    // 回填
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(&path, &bytes).await;
    CACHE.insert(url.to_string(), bytes.clone()).await;

    Ok(bytes)
}

async fn download(url: &str) -> Result<Vec<u8>, ImageFetchError> {
    use isahc::AsyncReadResponseExt;
    let mut resp = isahc::get_async(url).await.map_err(|e| ImageFetchError::Network(e.to_string()))?;
    resp.bytes().await
        .map(|b| b.to_vec())
        .map_err(|e| ImageFetchError::Network(e.to_string()))
}

#[derive(Debug, Clone)]
pub enum ImageFetchError {
    Network(String),
    Decode(String),
}

impl std::fmt::Display for ImageFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {}", e),
            Self::Decode(e)  => write!(f, "decode error: {}", e),
        }
    }
}