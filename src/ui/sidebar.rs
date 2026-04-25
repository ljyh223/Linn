//! 侧边栏子组件 — Player / Lyrics / Queue

use relm4::gtk::Orientation;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, adw, gtk};

use crate::player::messages::{PlaybackState, PlayerEvent};
use crate::ui::lyric::{LyricPage, LyricsMsg, LyricsOutput};
use crate::ui::player::{PlayerPage, PlayerPageMsg, PlayerPageOutput};
use crate::ui::queue::{QueueMsg, QueuePage, QueuePageOutput};
use crate::ui::route::SidebarPage;

pub struct Sidebar {
    stack: adw::ViewStack,
    current_page: SidebarPage,
    player_page: Controller<PlayerPage>,
    lyrics_page: Controller<LyricPage>,
    queue_page: Controller<QueuePage>,
}

// 在 window.rs 或单独的 route.rs 里定义

#[derive(Debug)]
pub enum SidebarMsg {
    SwitchPage(SidebarPage),
    PlayerCommand(PlayerPageOutput),
    LyricsCommand(LyricsOutput),
    QueueCommand(QueuePageOutput),
    PlayerEvent(PlayerEvent),
}

#[derive(Debug)]
pub enum SidebarOutput {
    PlayerCommand(PlayerPageOutput),
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
            .launch(())
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

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SidebarMsg::SwitchPage(tag) => {
                // ✅ 修复2：显式映射为小写字符串，确保和 add_titled 里的名字完全一致
                let page_name = match tag {
                    SidebarPage::Player => "player",
                    SidebarPage::Lyrics => "lyrics",
                    SidebarPage::Queue => "queue",
                };

                self.stack.set_visible_child_name(page_name);
                self.current_page = tag;
            }

            SidebarMsg::PlayerCommand(player_page_output) => {
                sender
                    .output(SidebarOutput::PlayerCommand(player_page_output))
                    .ok();
            }

            SidebarMsg::PlayerEvent(player_event) => match player_event {
                PlayerEvent::StateChanged(state) => {
                    self.player_page.emit(PlayerPageMsg::UpdatePlayback(
                        state == PlaybackState::Playing,
                    ));
                }
                PlayerEvent::TimeUpdated { position, duration } => {
                    self.player_page.emit(PlayerPageMsg::UpdateProgress {
                        position: position,
                        duration: duration,
                    });

                    self.lyrics_page.emit(LyricsMsg::GstTick(position));
                }
                PlayerEvent::TrackChanged {
                    song,
                    current_index,
                    is_liked,
                } => {
                    self.lyrics_page.emit(LyricsMsg::LoadById(song.id));
                    self.queue_page
                        .emit(QueueMsg::SetCurrentIndex(current_index));
                    self.player_page
                        .emit(PlayerPageMsg::UpdateTrack(song.clone()));
                    self.player_page
                        .emit(PlayerPageMsg::SetLiked(is_liked));
                }
                PlayerEvent::EndOfQueue => {},
                PlayerEvent::Error(_) => {},
                PlayerEvent::SetQueue {
                    tracks,
                    playlist,
                    start_index,
                } => {
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

            SidebarMsg::LyricsCommand(lyrics_output) => match lyrics_output {
                LyricsOutput::Seek(position) => {
                    sender
                        .output(SidebarOutput::PlayerCommand(PlayerPageOutput::Seek(
                            position,
                        )))
                        .ok();
                }
            },

            SidebarMsg::QueueCommand(queue_output) => match queue_output {
                QueuePageOutput::Remove(index) => {
                    sender
                        .output(SidebarOutput::PlayerCommand(PlayerPageOutput::Remove(
                            index,
                        )))
                        .ok();
                }
                QueuePageOutput::PlayAt(index) => {
                    sender
                        .output(SidebarOutput::PlayerCommand(PlayerPageOutput::PlayAt(index)))
                        .ok();
                }
            },
        }
    }
}
