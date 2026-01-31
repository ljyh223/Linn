pub mod async_image;

use iced::Subscription;
use iced::futures::{self, StreamExt};
use crate::ui::components::image::async_image::{IMAGE_CACHE, IN_FLIGHT, take_receiver};
use crate::utils::ImageSize;

#[derive(Debug, Clone)]
pub enum ImageLoaderEvent {
    ImageLoaded,
}

pub fn subscription() -> Subscription<ImageLoaderEvent> {
    // 关键修正：使用新的方式获取 Receiver
    Subscription::run(|| {
        let rx = match take_receiver() {
            Some(rx) => {
                println!("[ImageLoader] 核心引擎真正启动！");
                rx
            },
            None => {
                // 如果打印了这行，说明 subscription 被 Iced 重启了，但管道是单次有效的
                // 没关系，Iced 会维持之前的流运行
                return futures::stream::empty().boxed();
            }
        };

        futures::stream::unfold(rx, |mut rx| async move {
            let url = rx.recv().await?;
            Some((url, rx))
        })
        .map(|full_url: String| async move {
            // println!("[ImageLoader] 下载中: {}", full_url);
            match reqwest::get(&full_url).await {
                Ok(resp) => {
                    match resp.bytes().await {
                        Ok(bytes) => Some((full_url, bytes)),
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            }
        })
        .buffer_unordered(10)
        .filter_map(|res| async { res })
        .map(|(full_url, bytes)| {
            let handle = iced::widget::image::Handle::from_bytes(bytes);
            let parts: Vec<&str> = full_url.split('?').collect();
            let base_url = parts[0].to_string();
            
            let mut rank = 4;
            if parts.len() > 1 {
                if let Some(param_val) = parts[1].split('&')
                    .find(|s| s.starts_with("param="))
                    .map(|s| &s[6..]) 
                {
                    rank = ImageSize::from_param(param_val).to_rank();
                }
            }

            if let Ok(mut cache) = IMAGE_CACHE.write() {
                cache.entry(base_url.clone()).or_default().insert(rank, handle);
            }

            if let Ok(mut flying) = IN_FLIGHT.lock() {
                flying.remove(&full_url);
            }

            ImageLoaderEvent::ImageLoaded
        })
        .boxed()
    })
}