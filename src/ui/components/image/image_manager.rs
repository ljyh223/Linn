use moka::future::Cache;
use reqwest::Client;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::APP_NAME;

#[derive(Debug, Clone)]
pub enum FetchError {
    Cancelled,
    NetworkError(String),
    DiskError(String),
}

pub struct ImageManager {
    // 一级缓存：Moka 高性能内存缓存，自带 LRU 淘汰，最大容量 100 张图片
    memory_cache: Cache<String, Vec<u8>>,
    // 全局复用的 HTTP 客户端 (带连接池)
    http_client: Client,
    // 本地磁盘缓存目录
    cache_dir: PathBuf,
}

impl ImageManager {
    /// 获取全局单例
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ImageManager> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            // 初始化本地缓存目录 ~/.cache/linn/images/
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join(APP_NAME)
                .join("images");

            // 同步创建目录（仅在进程启动时执行一次，可接受阻塞）
            if !cache_dir.exists() {
                std::fs::create_dir_all(&cache_dir).expect("Failed to create image cache directory");
            }

            ImageManager {
                memory_cache: Cache::builder()
                    .max_capacity(100) // 内存中最多保留 100 张图
                    .time_to_idle(Duration::from_secs(10 * 60)) // 10 分钟不使用则释放
                    .build(),
                http_client: Client::builder()
                    .timeout(Duration::from_secs(15))
                    .build()
                    .expect("Failed to build reqwest client"),
                cache_dir,
            }
        })
    }

    /// 获取图片字节（三级缓存：Memory -> Disk -> Network）
    pub async fn fetch(&self, url: String, token: CancellationToken) -> Result<Vec<u8>, FetchError> {
        if url.is_empty() {
            return Err(FetchError::NetworkError("Empty URL".to_string()));
        }

        // 1. 查一级缓存 (Memory)
        if let Some(bytes) = self.memory_cache.get(&url).await {
            return Ok(bytes);
        }

        // 使用 tokio::select! 监听取消信号
        tokio::select! {
            _ = token.cancelled() => {
                Err(FetchError::Cancelled)
            }
            result = self.fetch_disk_or_network(&url) => {
                match result {
                    Ok(bytes) => {
                        // 写入一级缓存 (Memory)
                        self.memory_cache.insert(url.clone(), bytes.clone()).await;
                        Ok(bytes)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    /// 二级/三级缓存的具体实现
    async fn fetch_disk_or_network(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let file_path = self.get_disk_cache_path(url);

        // 2. 查二级缓存 (Disk)
        if let Ok(bytes) = fs::read(&file_path).await {
            return Ok(bytes);
        }

        // 3. 查三级缓存 (Network Request)
        let response = self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(FetchError::NetworkError(format!("HTTP {}", response.status())));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| FetchError::NetworkError(e.to_string()))?
            .to_vec();

        // 异步写入磁盘，不阻塞当前返回流 (Fire and forget)
        // 这样 UI 可以立刻拿到图片，后台慢慢存盘
        let file_path_clone = file_path.clone();
        let bytes_clone = bytes.clone();
        tokio::spawn(async move {
            if let Ok(mut file) = fs::File::create(&file_path_clone).await {
                let _ = file.write_all(&bytes_clone).await;
            }
        });

        Ok(bytes)
    }

    /// 工具方法：将 URL 转换为本地安全的文件名
    fn get_disk_cache_path(&self, url: &str) -> PathBuf {
        // 使用标准库简易哈希 URL 作为文件名
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let filename = format!("{:x}.img", hasher.finish());
        
        self.cache_dir.join(filename)
    }
}