//! 侧边栏子组件 — Player / Lyrics / Queue

use relm4::gtk::Orientation;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, adw, gtk};
use std::sync::Arc;

use crate::api::{Artist, Playlist, Song};
use crate::lyrics::LyricsPresentation;
use crate::player::messages::{PlaybackState, PlayerCommand, PlayerEvent};
use crate::ui::lyric::{LyricPage, LyricsMsg, LyricsOutput};
use crate::ui::player::{PlayerPage, PlayerPageMsg, PlayerPageOutput};
use crate::ui::queue::{QueueMsg, QueuePage, QueuePageOutput};
use crate::ui::route::{AppRoute, SidebarPage};

pub struct Sidebar {
    stack: adw::ViewStack,
    current_page: SidebarPage,
    player_page: Controller<PlayerPage>,
    lyrics_page: Controller<LyricPage>,
    queue_page: Controller<QueuePage>,
    lyric_color: (f64, f64, f64, f64),
}

// 在 window.rs 或单独的 route.rs 里定义

#[derive(Debug)]
pub enum SidebarMsg {
    SwitchPage(SidebarPage),
    PlayerCommand(PlayerPageOutput),
    LyricsCommand(LyricsOutput),
    QueueCommand(QueuePageOutput),
    PlayerEvent(PlayerEvent),
    /// 由持久化会话提供的首屏快照，只恢复播放器视图，不影响队列页。
    RestorePlaybackSnapshot {
        song: Song,
        playlist: Playlist,
    },
    /// Re-resolve the solid sidebar foreground after a system theme change.
    SyncLyricsTheme,
    /// 点击了右上角搜索图标
    SearchClicked,
}

/// 侧边栏输出：直接分离播放器命令和 UI 操作
#[derive(Debug)]
pub enum SidebarOutput {
    /// 直接发送给 PlayerFacade 的播放器命令
    PlayerCommand(PlayerCommand),
    /// UI 专用操作（导航、对话框等）
    NavigateTo(AppRoute),
    OpenArtistDialog(Vec<Artist>),
    CollectSong(u64),
}

#[relm4::component(pub)]
impl SimpleComponent for Sidebar {
    type Init = ();
    type Input = SidebarMsg;
    type Output = SidebarOutput;

    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                set_show_start_title_buttons: true,
                set_show_end_title_buttons: true,

                pack_end = &gtk::Button {
                    set_icon_name: "system-search-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("搜索"),
                    connect_clicked => SidebarMsg::SearchClicked,
                },
            },

            #[name(stack)]
            #[wrap(Some)]
            set_content = &adw::ViewStack {},

            add_bottom_bar = &gtk::Box {
                set_orientation: Orientation::Horizontal,
                set_homogeneous: true,
                set_spacing: 4,
                set_margin_start: 7,
                set_margin_end: 7,
                set_margin_top: 6,
                set_margin_bottom: 6,

                gtk::ToggleButton {
                    add_css_class: "flat",
                    #[wrap(Some)]
                    set_child = &adw::ButtonContent {
                        set_icon_name: "music-note",
                        set_label: "Player",
                    },
                    #[watch]
                    set_active: model.current_page == SidebarPage::Player,
                    connect_clicked => SidebarMsg::SwitchPage(SidebarPage::Player),
                },

                gtk::ToggleButton {
                    add_css_class: "flat",
                    #[wrap(Some)]
                    set_child = &adw::ButtonContent {
                        set_icon_name: "chat-bubble-text",
                        set_label: "Lyrics",
                    },
                    #[watch]
                    set_active: model.current_page == SidebarPage::Lyrics,
                    connect_clicked => SidebarMsg::SwitchPage(SidebarPage::Lyrics),
                },

                gtk::ToggleButton {
                    add_css_class: "flat",
                    #[wrap(Some)]
                    set_child = &adw::ButtonContent {
                        set_icon_name: "music-queue",
                        set_label: "Collection",
                    },
                    #[watch]
                    set_active: model.current_page == SidebarPage::Queue,
                    connect_clicked => SidebarMsg::SwitchPage(SidebarPage::Queue),
                },
            },

            set_bottom_bar_style: adw::ToolbarStyle::Flat,
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let player_page = PlayerPage::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| SidebarMsg::PlayerCommand(msg));

        let lyric_page = LyricPage::builder()
            .launch(LyricsPresentation::Sidebar)
            .forward(sender.input_sender(), |msg| SidebarMsg::LyricsCommand(msg));

        let queue_page = QueuePage::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| SidebarMsg::QueueCommand(msg));

        let mut model = Self {
            stack: adw::ViewStack::default(),
            current_page: SidebarPage::Player,
            player_page: player_page,
            lyrics_page: lyric_page,
            queue_page: queue_page,
            lyric_color: (0.0, 0.0, 0.0, 0.0),
        };

        let widgets = view_output!();

        model.stack = widgets.stack.clone();

        widgets
            .stack
            .add_titled(model.player_page.widget(), Some("player"), "Player");
        widgets
            .stack
            .add_titled(model.lyrics_page.widget(), Some("lyrics"), "Lyrics");
        widgets
            .stack
            .add_titled(model.queue_page.widget(), Some("queue"), "Queue");

        widgets.stack.set_visible_child_name("player");

        // The custom snapshot widgets do not always inherit the sidebar foreground. Pass the
        // resolved solid-surface colour explicitly and refresh it when libadwaita changes theme.
        model.sync_lyric_color();
        let theme_sender = sender.input_sender().clone();
        let style_manager = adw::StyleManager::default();
        style_manager.connect_dark_notify(move |_| theme_sender.emit(SidebarMsg::SyncLyricsTheme));
        let contrast_sender = sender.input_sender().clone();
        style_manager.connect_high_contrast_notify(move |_| {
            contrast_sender.emit(SidebarMsg::SyncLyricsTheme)
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SidebarMsg::SearchClicked => {
                sender
                    .output(SidebarOutput::NavigateTo(AppRoute::Search))
                    .ok();
            }

            SidebarMsg::SwitchPage(tag) => {
                // ✅ 修复2：显式映射为小写字符串，确保和 add_titled 里的名字完全一致
                let lyrics_selected = tag == SidebarPage::Lyrics;
                let page_name = match tag {
                    SidebarPage::Player => "player",
                    SidebarPage::Lyrics => "lyrics",
                    SidebarPage::Queue => "queue",
                };

                self.stack.set_visible_child_name(page_name);
                self.current_page = tag;
                if lyrics_selected {
                    self.sync_lyric_color();
                }
            }

            SidebarMsg::SyncLyricsTheme => self.sync_lyric_color(),

            SidebarMsg::PlayerCommand(player_page_output) => {
                match player_page_output {
                    // 直接映射到 PlayerCommand
                    PlayerPageOutput::TogglePlay => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::TogglePlayPause))
                            .ok();
                    }
                    PlayerPageOutput::PrevTrack => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::Previous))
                            .ok();
                    }
                    PlayerPageOutput::NextTrack => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::Next))
                            .ok();
                    }
                    PlayerPageOutput::Seek(val) => {
                        self.lyrics_page.emit(LyricsMsg::ExternalSeek(val));
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::Seek(val)))
                            .ok();
                    }
                    PlayerPageOutput::Remove(index) => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::Remove(index)))
                            .ok();
                    }
                    PlayerPageOutput::PlayAt(index) => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::PlayAt(index)))
                            .ok();
                    }
                    PlayerPageOutput::SetMode(mode) => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::SetPlayMode(
                                mode,
                            )))
                            .ok();
                    }
                    PlayerPageOutput::SetLoop(enabled) => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::SetLoop(
                                enabled,
                            )))
                            .ok();
                    }
                    PlayerPageOutput::ToggleLike(id, liked) => {
                        sender
                            .output(SidebarOutput::PlayerCommand(PlayerCommand::LikeSong {
                                song_id: id,
                                liked,
                            }))
                            .ok();
                    }
                    // UI 专用操作
                    PlayerPageOutput::Navigate(route) => {
                        sender.output(SidebarOutput::NavigateTo(route)).ok();
                    }
                    PlayerPageOutput::OpenArtistDialog(artists) => {
                        sender.output(SidebarOutput::OpenArtistDialog(artists)).ok();
                    }
                    PlayerPageOutput::CollectSong(id) => {
                        sender.output(SidebarOutput::CollectSong(id)).ok();
                    }
                }
            }

            SidebarMsg::PlayerEvent(player_event) => match player_event {
                PlayerEvent::StateChanged(state) => {
                    self.player_page.emit(PlayerPageMsg::UpdatePlayback(
                        state == PlaybackState::Playing,
                    ));
                    self.lyrics_page
                        .emit(LyricsMsg::PlaybackChanged(state == PlaybackState::Playing));
                }
                PlayerEvent::PlaybackSettingsChanged {
                    play_mode,
                    loop_enabled,
                } => {
                    self.player_page
                        .emit(PlayerPageMsg::UpdatePlaybackSettings {
                            play_mode,
                            loop_enabled,
                        });
                }
                PlayerEvent::TimeUpdated { position, duration } => {
                    self.player_page.emit(PlayerPageMsg::UpdateProgress {
                        position: position,
                        duration: duration,
                    });

                    self.lyrics_page
                        .emit(LyricsMsg::GstTick { position, duration });
                }
                PlayerEvent::TrackChanged {
                    song,
                    current_index,
                    is_liked,
                } => {
                    self.lyrics_page.emit(LyricsMsg::LoadBySong(song.clone()));
                    self.queue_page
                        .emit(QueueMsg::SetCurrentIndex(current_index));
                    self.player_page
                        .emit(PlayerPageMsg::UpdateTrack(song.clone()));
                    self.player_page.emit(PlayerPageMsg::SetLiked(is_liked));
                }
                PlayerEvent::EndOfQueue => {}
                PlayerEvent::Error(_) => {}
                PlayerEvent::ShowToast(_) => {} // 由 Window 处理
                PlayerEvent::SetQueue {
                    tracks,
                    playlist,
                    start_index,
                } => {
                    if let Some(next_song) = tracks.get(start_index.saturating_add(1)) {
                        self.lyrics_page
                            .emit(LyricsMsg::PreloadSong(next_song.clone()));
                    }
                    self.queue_page.emit(QueueMsg::SetQueue {
                        songs: tracks.clone(),
                        playlist: playlist.clone(),
                        start_index,
                    });
                    self.player_page.emit(PlayerPageMsg::SetQueue {
                        tracks: tracks.clone(),
                        playlist: playlist.clone(),
                        start_index,
                    });
                }
            },

            SidebarMsg::RestorePlaybackSnapshot { song, playlist } => {
                self.player_page.emit(PlayerPageMsg::UpdateTrack(song));
                self.player_page.emit(PlayerPageMsg::SetQueue {
                    tracks: Arc::new(Vec::new()),
                    playlist: Arc::new(playlist),
                    start_index: 0,
                });
            }

            SidebarMsg::LyricsCommand(lyrics_output) => match lyrics_output {
                LyricsOutput::Seek(position) => {
                    sender
                        .output(SidebarOutput::PlayerCommand(PlayerCommand::Seek(position)))
                        .ok();
                }
            },

            SidebarMsg::QueueCommand(queue_output) => match queue_output {
                QueuePageOutput::Remove(index) => {
                    sender
                        .output(SidebarOutput::PlayerCommand(PlayerCommand::Remove(index)))
                        .ok();
                }
                QueuePageOutput::PlayAt(index) => {
                    sender
                        .output(SidebarOutput::PlayerCommand(PlayerCommand::PlayAt(index)))
                        .ok();
                }
            },
        }
    }
}

impl Sidebar {
    fn sync_lyric_color(&mut self) {
        let color = self.lyrics_page.widget().color();
        let resolved = (
            color.red() as f64,
            color.green() as f64,
            color.blue() as f64,
            color.alpha() as f64,
        );
        if rgba_changed(self.lyric_color, resolved) {
            self.lyric_color = resolved;
            self.lyrics_page.emit(LyricsMsg::SetTextColor(
                resolved.0, resolved.1, resolved.2, resolved.3,
            ));
        }
    }
}

fn rgba_changed(previous: (f64, f64, f64, f64), next: (f64, f64, f64, f64)) -> bool {
    const EPSILON: f64 = 1.0 / 255.0;
    (previous.0 - next.0).abs() > EPSILON
        || (previous.1 - next.1).abs() > EPSILON
        || (previous.2 - next.2).abs() > EPSILON
        || (previous.3 - next.3).abs() > EPSILON
}

#[cfg(test)]
mod tests {
    use super::rgba_changed;

    #[test]
    fn lyric_theme_sync_ignores_rounding_noise_but_detects_real_changes() {
        assert!(!rgba_changed(
            (0.2, 0.3, 0.4, 1.0),
            (0.201, 0.301, 0.401, 1.0)
        ));
        assert!(rgba_changed(
            (0.08, 0.08, 0.08, 1.0),
            (0.92, 0.92, 0.92, 1.0)
        ));
    }
}
