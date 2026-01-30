use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// Import for image encoding
use image::ImageEncoder;

/// 图片缓存管理器
#[derive(Clone)]
pub struct ImageCache {
    cache_dir: PathBuf,
    http_client: reqwest::Client,
}

impl ImageCache {
    /// 创建新的图片缓存管理器
    pub fn new() -> anyhow::Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("linn")
            .join("images");

        // 确保缓存目录存在
        std::fs::create_dir_all(&cache_dir)?;

        // 创建 HTTP 客户端，设置 User-Agent
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            cache_dir,
            http_client,
        })
    }

    /// 从 URL 获取图片的缓存路径
    pub fn get_cache_path(&self, url: &str) -> PathBuf {
        let hash = Self::hash_url(url);
        self.cache_dir.join(hash)
    }

    /// 检查图片是否已缓存
    pub fn is_cached(&self, url: &str) -> bool {
        self.get_cache_path(url).exists()
    }

    /// 从缓存加载图片（如果存在）
    pub async fn load_from_cache(&self, url: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let path = self.get_cache_path(url);

        if path.exists() {
            let data = fs::read(&path).await?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// 下载并缓存图片
    pub async fn download_and_cache(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        // 如果已经缓存，直接返回
        if let Some(cached) = self.load_from_cache(url).await? {
            return Ok(cached);
        }

        // 下载图片（使用重用的 HTTP 客户端）
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch image: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download image: HTTP {}",
                status
            ));
        }

        let data = response.bytes().await?.to_vec();

        // 保存到缓存
        let path = self.get_cache_path(url);
        let mut file = fs::File::create(&path).await?;
        file.write_all(&data).await?;

        Ok(data)
    }

    /// 生成 URL 的哈希值作为文件名
    fn hash_url(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = hasher.finalize();
        format!("{:x}.jpg", hash)
    }

    /// 处理图片数据，添加圆角
    pub fn apply_rounded_corners(
        data: Vec<u8>,
        corner_radius: u32,
    ) -> anyhow::Result<Vec<u8>> {
        // 解码图片
        let img = image::load_from_memory(&data)?;
        let mut rgba = img.to_rgba8();

        let width = rgba.width();
        let height = rgba.height();
        let radius = corner_radius.min(width / 2).min(height / 2);

        // 创建圆角遮罩
        for y in 0..height {
            for x in 0..width {
                // 检查是否在四个角的圆形区域内
                let in_corner = match (x < radius, y < radius, x >= width - radius, y >= height - radius) {
                    (true, true, _, _) => { // 左上角
                        (x as f32 - radius as f32).powi(2) + (y as f32 - radius as f32).powi(2) > (radius as f32).powi(2)
                    }
                    (_, true, true, _) => { // 右上角
                        ((x as f32 - (width - radius) as f32)).powi(2) + (y as f32 - radius as f32).powi(2) > (radius as f32).powi(2)
                    }
                    (true, _, _, true) => { // 左下角
                        (x as f32 - radius as f32).powi(2) + ((y as f32 - (height - radius) as f32)).powi(2) > (radius as f32).powi(2)
                    }
                    (_, _, true, true) => { // 右下角
                        ((x as f32 - (width - radius) as f32)).powi(2) + ((y as f32 - (height - radius) as f32)).powi(2) > (radius as f32).powi(2)
                    }
                    _ => false,
                };

                if in_corner {
                    // 设置为透明
                    let pixel_index = ((y * width + x) * 4) as usize;
                    rgba.as_mut()[pixel_index + 3] = 0; // Alpha = 0
                }
            }
        }

        // 编码回 PNG
        let mut output = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut output);
        encoder.write_image(
            rgba.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )?;

        Ok(output)
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new().expect("Failed to create image cache")
    }
}
