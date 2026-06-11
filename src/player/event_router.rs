use std::sync::{Arc, Mutex};
use flume::Sender;

use super::messages::PlayerEvent;

/// PlayerEvent 的多播路由器
///
/// 解决问题：PlayerFacade 发送事件后，需要广播给多个消费者（Window、Sidebar、FullscreenLyricPage）
/// 原来由 Window 手动转发，现在由 EventBus 自动广播
pub struct PlayerEventBus {
    subscribers: Arc<Mutex<Vec<flume::Sender<PlayerEvent>>>>,
}

impl PlayerEventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 创建一个 Sender，PlayerFacade 可以用它发送事件
    pub fn create_sender(&self) -> Sender<PlayerEvent> {
        let (tx, rx) = flume::unbounded::<PlayerEvent>();
        let subscribers = self.subscribers.clone();

        // 启动一个转发线程，把收到的事件广播给所有订阅者
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                let subs = subscribers.lock().unwrap();
                for sub in subs.iter() {
                    let _ = sub.send(event.clone());
                }
            }
        });

        tx
    }

    /// 订阅事件，返回一个接收者
    pub fn subscribe(&self) -> flume::Receiver<PlayerEvent> {
        let (tx, rx) = flume::unbounded::<PlayerEvent>();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    /// 创建一个适配器，把 PlayerEvent 转换成其他消息
    pub fn subscribe_with_map<F, T>(&self, mapper: F) -> flume::Receiver<T>
    where
        F: Fn(PlayerEvent) -> T + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = flume::unbounded::<T>();
        let event_rx = self.subscribe();

        std::thread::spawn(move || {
            while let Ok(event) = event_rx.recv() {
                let msg = mapper(event);
                let _ = tx.send(msg);
            }
        });

        rx
    }
}

impl Default for PlayerEventBus {
    fn default() -> Self {
        Self::new()
    }
}
