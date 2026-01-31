pub mod async_image;

use iced::Subscription;
use iced::futures::StreamExt;


use crate::ui::components::image::async_image::{IMAGE_CACHE, LOAD_QUEUE};

#[derive(Debug, Clone)]
pub enum ImageLoaderEvent {
    ImageLoaded, // We just need to signal "something changed"
}

pub fn subscription() -> Subscription<ImageLoaderEvent> {
    Subscription::run(|| {
        iced::futures::stream::unfold((), |_| async move {
            // Check queue every 100ms
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            
            let mut urls = Vec::new();
            {
                let mut queue = LOAD_QUEUE.lock().unwrap();
                for url in queue.drain() {
                    urls.push(url);
                }
            }

            if urls.is_empty() { return Some((Vec::new(), ())); }

            let mut loaded_any = false;
            for url in urls {
                // Check if already in cache to avoid double-loading
                if IMAGE_CACHE.read().unwrap().contains_key(&url) { continue; }

                if let Ok(resp) = reqwest::get(&url).await {
                    if let Ok(bytes) = resp.bytes().await {
                        let handle = iced::widget::image::Handle::from_bytes(bytes);
                        IMAGE_CACHE.write().unwrap().insert(url, handle);
                        loaded_any = true;
                    }
                }
            }

            // Important: We only send the message if at least one image loaded.
            // This prevents an infinite loop of empty messages.
            let events = if loaded_any {
                vec![ImageLoaderEvent::ImageLoaded]
            } else {
                vec![]
            };

            Some((events, ()))
        })
        .flat_map(iced::futures::stream::iter)
    })
}