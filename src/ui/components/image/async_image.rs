use iced::widget::{container, image, center, text};
use iced::{border, Color, Element, Fill, Length, Theme};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, RwLock};
use tokio::sync::mpsc;
use crate::utils::ImageSize;

pub type MultiResCache = HashMap<String, BTreeMap<u32, image::Handle>>;
pub static IMAGE_CACHE: Lazy<RwLock<MultiResCache>> = Lazy::new(|| RwLock::new(HashMap::new()));

// 防止同一 URL 同时发起多个 HTTP 请求
pub static IN_FLIGHT: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub static LOAD_TX: Lazy<mpsc::UnboundedSender<String>> = Lazy::new(|| IMAGE_CHANNEL.tx.clone());

static IMAGE_CHANNEL: Lazy<ImageChannel> = Lazy::new(|| {
    let (tx, rx) = mpsc::unbounded_channel();
    ImageChannel {
        tx,
        rx: Mutex::new(Some(rx)),
    }
});


pub static LOAD_RX: Lazy<Mutex<Option<mpsc::UnboundedReceiver<String>>>> = 
    Lazy::new(|| Mutex::new(None));

pub fn take_receiver() -> Option<mpsc::UnboundedReceiver<String>> {
    IMAGE_CHANNEL.rx.lock().unwrap().take()
}

struct ImageChannel {
    tx: mpsc::UnboundedSender<String>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<String>>>,
}



pub struct AsyncImage {
    url: String,
    size: ImageSize,
    radius: f32,
    width: Length,
    height: Length,
}

impl AsyncImage {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            size: ImageSize::Medium,
            radius: 0.0,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    pub fn size(mut self, size: ImageSize) -> Self { self.size = size; self }
    pub fn border_radius(mut self, r: f32) -> Self { self.radius = r; self }
    pub fn width(mut self, w: Length) -> Self { self.width = w; self }
    pub fn height(mut self, h: Length) -> Self { self.height = h; self }

    pub fn view<Message: 'static>(self) -> Element<'static, Message> {
        let cache = IMAGE_CACHE.read().unwrap();
        let target_rank = self.size.to_rank(); 

        let best_available = cache.get(&self.url).and_then(|variants| {
            variants.range(target_rank..).next().map(|(_, h)| h)
                .or_else(|| variants.values().last())
        });

        let (content, needs_load): (Element<'static, Message>, bool) = match best_available {
            Some(handle) => {
                let has_exact = cache.get(&self.url)
                    .map(|v| v.contains_key(&target_rank))
                    .unwrap_or(false);
                
                // 如果已经有完美的图，就显示它；如果图质量不够，显示它并标记需要加载更清晰的
                (Element::from(iced::widget::image(handle.clone()).width(Fill).height(Fill)), !has_exact)
            }
            None => {
                // 完全没缓存，显示占位
                (Element::from(center(text("..."))), true)
            }
        };

        if needs_load {
            let full_url = self.size.apply_to_url(&self.url);
            
            let mut flying = IN_FLIGHT.lock().unwrap();
            if !flying.contains(&full_url) {
                flying.insert(full_url.clone());
                println!("[AsyncImage] 请求下载: {}", full_url);
                let _ = LOAD_TX.send(full_url);
            }
        }

        container(content)
            .width(self.width)
            .height(self.height)
            .clip(true)
            .style(move |_| container::Style {
                border: border::Border {
                    radius: self.radius.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}