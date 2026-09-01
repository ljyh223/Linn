use ksni::{Tray, TrayMethods};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::player::messages::{PlaybackState, PlayerEvent};

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    ToggleWindow,
    PreviousTrack,
    TogglePlayPause,
    NextTrack,
    Quit,
    HostAvailabilityChanged(bool),
}

/// StatusNotifierItem 托盘服务。
///
/// 服务运行在独立 async-io 线程：它只把用户操作转发给 UI，播放器状态则通过
/// `PlayerEventBus` 的订阅更新标题和菜单，避免 GTK 主线程承担 D-Bus 事件循环。
pub fn start(action_tx: flume::Sender<TrayAction>, player_events: flume::Receiver<PlayerEvent>) {
    std::thread::spawn(move || {
        async_std::task::block_on(async move {
            // ksni 在启动时已有宿主的情况下不会调用 `watcher_online`。
            // 用这个标记区分“初始注册成功”和“允许稍后重连的离线状态”。
            let watcher_went_offline = Arc::new(AtomicBool::new(false));
            let tray = LinnTray::new(action_tx.clone(), watcher_went_offline.clone());
            let handle = match tray.assume_sni_available(true).spawn().await {
                Ok(handle) => handle,
                Err(error) => {
                    log::warn!("无法启动系统托盘: {error}");
                    return;
                }
            };

            // 注册成功且期间没有收到离线回调，说明启动时已有可用托盘宿主。
            if !watcher_went_offline.load(Ordering::Acquire) {
                let _ = action_tx.send(TrayAction::HostAvailabilityChanged(true));
            }

            while let Ok(event) = player_events.recv_async().await {
                handle
                    .update(move |tray| tray.apply_player_event(&event))
                    .await;
            }
        });
    });
}

struct LinnTray {
    action_tx: flume::Sender<TrayAction>,
    watcher_went_offline: Arc<AtomicBool>,
    track_title: Option<String>,
    is_playing: bool,
}

impl LinnTray {
    fn new(action_tx: flume::Sender<TrayAction>, watcher_went_offline: Arc<AtomicBool>) -> Self {
        Self {
            action_tx,
            watcher_went_offline,
            track_title: None,
            is_playing: false,
        }
    }

    fn send_action(&self, action: TrayAction) {
        if let Err(error) = self.action_tx.send(action) {
            log::debug!("托盘动作接收端已关闭: {error}");
        }
    }

    fn apply_player_event(&mut self, event: &PlayerEvent) {
        match event {
            PlayerEvent::TrackChanged { song, .. } => self.track_title = Some(song.name.clone()),
            PlayerEvent::StateChanged(state) => {
                self.is_playing = *state == PlaybackState::Playing;
            }
            _ => {}
        }
    }
}

impl Tray for LinnTray {
    fn id(&self) -> String {
        "io.github.ljyh223.Linn".into()
    }

    fn icon_name(&self) -> String {
        "io.github.ljyh223.Linn".into()
    }

    fn title(&self) -> String {
        match (&self.track_title, self.is_playing) {
            (Some(title), true) => format!("Linn · 正在播放 {title}"),
            (Some(title), false) => format!("Linn · 已暂停 {title}"),
            (None, _) => "Linn".into(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send_action(TrayAction::ToggleWindow);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;

        let previous_tx = self.action_tx.clone();
        let play_pause_tx = self.action_tx.clone();
        let next_tx = self.action_tx.clone();
        let show_tx = self.action_tx.clone();
        let quit_tx = self.action_tx.clone();

        vec![
            StandardItem {
                label: "上一首".into(),
                icon_name: "media-skip-backward-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = previous_tx.send(TrayAction::PreviousTrack);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if self.is_playing { "暂停" } else { "播放" }.into(),
                icon_name: if self.is_playing {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                }
                .into(),
                activate: Box::new(move |_| {
                    let _ = play_pause_tx.send(TrayAction::TogglePlayPause);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "下一首".into(),
                icon_name: "media-skip-forward-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = next_tx.send(TrayAction::NextTrack);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "显示/隐藏窗口".into(),
                icon_name: "view-reveal-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = show_tx.send(TrayAction::ToggleWindow);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "退出".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = quit_tx.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_online(&self) {
        self.watcher_went_offline.store(false, Ordering::Release);
        self.send_action(TrayAction::HostAvailabilityChanged(true));
    }

    fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
        self.watcher_went_offline.store(true, Ordering::Release);
        self.send_action(TrayAction::HostAvailabilityChanged(false));
        // 保持服务存活，供 GNOME 扩展、面板或桌面会话稍后接管托盘。
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Song;

    #[test]
    fn title_reflects_the_current_track_and_playback_state() {
        let (action_tx, _action_rx) = flume::unbounded();
        let mut tray = LinnTray::new(action_tx, Arc::new(AtomicBool::new(false)));

        tray.apply_player_event(&PlayerEvent::TrackChanged {
            song: Song {
                name: "夜曲".into(),
                ..Default::default()
            },
            current_index: 0,
            is_liked: false,
        });
        tray.apply_player_event(&PlayerEvent::StateChanged(PlaybackState::Playing));
        assert_eq!(tray.title(), "Linn · 正在播放 夜曲");

        tray.apply_player_event(&PlayerEvent::StateChanged(PlaybackState::Paused));
        assert_eq!(tray.title(), "Linn · 已暂停 夜曲");
    }

    #[test]
    fn activation_requests_a_window_toggle() {
        let (action_tx, action_rx) = flume::unbounded();
        let mut tray = LinnTray::new(action_tx, Arc::new(AtomicBool::new(false)));

        tray.activate(0, 0);

        assert!(matches!(action_rx.try_recv(), Ok(TrayAction::ToggleWindow)));
    }
}
