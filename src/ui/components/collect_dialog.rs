use relm4::adw;
use relm4::adw::prelude::AdwDialogExt;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use futures::FutureExt;

use crate::api::{Playlist, get_user_playlist_created, playlist_track_add};
use crate::ui::components::image::AsyncImage;

pub struct PlaylistItem {
    playlist: Playlist,
}

pub struct PlaylistItemInit {
    pub playlist: Playlist,
}

#[relm4::factory(pub)]
impl FactoryComponent for PlaylistItem {
    type Init = PlaylistItemInit;
    type Input = ();
    type Output = u64;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 12,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_margin_start: 16,
            set_margin_end: 16,

            AsyncImage {
                set_width_request: 48,
                set_height_request: 48,
                set_corner_radius: 4.0,
                set_url: format!("{}?param=100y100", self.playlist.cover_url),
                set_placeholder_icon: "folder-music-symbolic",
                set_fallback_icon: "image-missing-symbolic",
            },

            gtk::Label {
                set_label: &self.playlist.name,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                // set_ellipsize: gtk::pango::EllipsizeMode::End,
            },

            gtk::Button {
                set_label: "添加",
                set_valign: gtk::Align::Center,
                connect_clicked[sender, id = self.playlist.id] => move |_| {
                    sender.output(id).ok();
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { playlist: init.playlist }
    }
}

pub struct CollectDialog {
    song_id: u64,
    factory: FactoryVecDeque<PlaylistItem>,
}

#[derive(Debug)]
pub enum CollectDialogMsg {
    PlaylistSelected(u64),
}

#[derive(Debug)]
pub enum CollectDialogCmdMsg {
    PlaylistsFetched(Vec<Playlist>),
    AddResult { success: bool, playlist_name: String },
}

#[relm4::component(pub)]
impl Component for CollectDialog {
    type Init = (u64, u64);
    type Input = CollectDialogMsg;
    type Output = String;
    type CommandOutput = CollectDialogCmdMsg;

    view! {
        #[root]
        adw::Dialog {
            set_title: "Collect to Playlist",
            set_content_width: 360,
            set_follows_content_size: true,

            #[wrap(Some)]
            set_child = &gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_propagate_natural_height: true,
                set_max_content_height: 400,

                #[local_ref]
                factory_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 8,
                    set_margin_bottom: 8,
                    set_spacing: 4,
                }
            }
        }
    }

    fn init(
        (song_id, user_id): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let factory = FactoryVecDeque::builder()
            .launch(gtk::Box::new(gtk::Orientation::Vertical, 0))
            .forward(sender.input_sender(), CollectDialogMsg::PlaylistSelected);

        let model = Self { song_id, factory };

        let factory_box = model.factory.widget();
        let widgets = view_output!();

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    if let Ok(playlists) = get_user_playlist_created(user_id).await {
                        let _ = out.send(CollectDialogCmdMsg::PlaylistsFetched(playlists));
                    }
                })
                .drop_on_shutdown()
                .boxed()
        });

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CollectDialogMsg::PlaylistSelected(pid) => {
                let sid = self.song_id;
                let name = {
                    let guard = self.factory.guard();
                    guard
                        .iter()
                        .find(|item| item.playlist.id == pid)
                        .map(|item| item.playlist.name.clone())
                        .unwrap_or_default()
                };

                eprintln!("收藏歌曲到歌单: pid: {}, sid: {}", pid , sid);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let success = playlist_track_add(pid, sid).await.is_ok();
                            eprintln!("收藏结果: {}", success);
                            let _ = out.send(CollectDialogCmdMsg::AddResult {
                                success,
                                playlist_name: name,
                            });
                        })
                        .drop_on_shutdown()
                        .boxed()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CollectDialogCmdMsg::PlaylistsFetched(playlists) => {
                let mut guard = self.factory.guard();
                guard.clear();
                for playlist in playlists {
                    guard.push_back(PlaylistItemInit { playlist });
                }
            }
            CollectDialogCmdMsg::AddResult {
                success,
                playlist_name,
            } => {
                let toast_msg = if success {
                    format!("已添加到「{}」", playlist_name)
                } else {
                    format!("添加到「{}」失败", playlist_name)
                };
                sender.output(toast_msg).ok();
                root.close();
            }
        }
    }
}
