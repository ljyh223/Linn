use std::collections::HashMap;
use std::sync::{ Mutex, OnceLock};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum FetchError {
    Cancelled,
    NetworkError,
}

pub struct ImageManager {
    // 一级缓存：内存缓存 (直接保存图片的字节数据)
    // 实际项目中推荐使用 moka::future::Cache
    memory_cache: Mutex<HashMap<String, Vec<u8>>>,
}

impl ImageManager {
    /// 获取全局共享的 ImageManager 实例
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ImageManager> = OnceLock::new();
        INSTANCE.get_or_init(|| ImageManager {
            memory_cache: Mutex::new(HashMap::new()),
        })
    }

    /// 在 Tokio 线程中执行异步下载，并支持通过 CancellationToken 取消
    pub async fn fetch(&self, url: String, token: CancellationToken) -> Result<Vec<u8>, FetchError> {
        // 1. 检查内存缓存
        if let Some(bytes) = self.memory_cache.lock().unwrap().get(&url) {
            return Ok(bytes.clone());
        }

        // TODO: implement disk IO (二级缓存：查询本地磁盘)
        // let disk_result = check_disk_cache(&url).await; ...

        // 3. 网络请求 (使用 tokio::select! 监听取消信号)
        tokio::select! {
            _ = token.cancelled() => {
                // 任务被前端取消
                Err(FetchError::Cancelled)
            }
            result = self.download_from_network(&url) => {
                match result {
                    Ok(bytes) => {
                        // 写入内存缓存
                        self.memory_cache.lock().unwrap().insert(url.clone(), bytes.clone());
                        // TODO: implement disk IO (写入本地磁盘)
                        Ok(bytes)
                    }
                    Err(_) => Err(FetchError::NetworkError),
                }
            }
        }
    }

    /// 模拟网络耗时下载
    async fn download_from_network(&self, url: &str) -> Result<Vec<u8>, ()> {
        // 模拟网络延迟
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        
        if url.is_empty() {
            return Err(());
        }
        // 实际开发中这里使用 reqwest::get(url)...
        // 此处返回一个极小的透明 1x1 PNG 字节作为 mock 成功响应
        let mock_png: Vec<u8> = vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
            0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 8, 153, 99, 96, 0, 0, 0, 2, 0, 1,
            244, 113, 100, 166, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        Ok(mock_png)
    }
}