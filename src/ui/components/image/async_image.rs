use iced::futures::StreamExt;
use iced::widget::{container, image, stack, text, center, column};
use iced::{Color, Element, Fill, Length, Subscription, Task, Theme, border, futures};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};

use crate::utils::ImageSize;



pub static IMAGE_CACHE: Lazy<RwLock<HashMap<String, image::Handle>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static LOAD_QUEUE: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

#[derive(Debug, Clone)]
pub enum InternalEvent {
    Loaded(String, image::Handle),
}

// --- THE ASYNC IMAGE COMPONENT ---

pub struct AsyncImage {
    url: String,
    size: Option<ImageSize>,
    placeholder_text: String,
    radius: f32,
    width: Length,
    height: Length,
}

impl AsyncImage {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            size: None,
            placeholder_text: "Loading...".into(),
            radius: 0.0,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder_text = text.into();
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    fn resolved_url(&self) -> String {
        match self.size {
            Some(size) => size.apply_to_url(&self.url),
            None => self.url.clone(), // Original
        }
    }

     pub fn view<Message: 'static>(self) -> Element<'static, Message> {
        let resolved_url = self.resolved_url(); // The URL with size params
        let cache = IMAGE_CACHE.read().unwrap();

        // FIX: Look up the RESOLVED url in the cache, not the raw one
        let content: Element<'static, Message> = if let Some(handle) = cache.get(&resolved_url) {
            image(handle.clone()).width(Fill).height(Fill).into()
        } else {
            // Register for loading using the resolved url
            let mut queue = LOAD_QUEUE.lock().unwrap();
            queue.insert(resolved_url.clone());
            
            center(text(self.placeholder_text.clone())).into()
        };

        container(content)
            .width(self.width)
            .height(self.height)
            .clip(true)
            .style(move |_theme: &Theme| {
                container::Style {
                    border: border::Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: self.radius.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}