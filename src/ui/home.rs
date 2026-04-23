use log::trace;
use relm4::gtk::{FlowBox, prelude::*};
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, factory::FactoryVecDeque, gtk}; // ✅ 引入 FactoryVecDeque

use super::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};
use crate::api::{Playlist, get_recommend_playlist};
use crate::ui::model::PlaylistType;

pub struct Home {
    playlist_cards: FactoryVecDeque<PlaylistCard>,
}

#[derive(Debug)]
pub enum HomeMsg {
    LoadPlaylists,
    CardAction(PlaylistCardOutput),
}

#[derive(Debug)]
pub enum HomeCmdMsg {
    PlaylistsLoaded(Vec<Playlist>),
}

#[derive(Debug)]
pub enum HomeOutput {
    OpenPlaylistDetail(u64),
    Playlist(PlaylistType),
}

#[relm4::component(pub)]
impl Component for Home {
    type Init = ();
    type Input = HomeMsg;
    type CommandOutput = HomeCmdMsg;
    type Output = HomeOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_vexpand: true,
            gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_margin_top: 16,
            set_margin_bottom: 16,
            set_margin_start: 16,
            set_margin_end: 16,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_margin_bottom: 4,

                gtk::Label {
                    set_label: "推荐歌单",
                    add_css_class: "title-3",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },
            },

            #[name(cards_box)]
            gtk::FlowBox {
                set_orientation: gtk::Orientation::Horizontal,
                set_row_spacing: 16,
                set_column_spacing: 16,
                set_min_children_per_line: 2,
                set_max_children_per_line: 7,
            }

        }
        }
        
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            playlist_cards: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| {
                    HomeMsg::CardAction(msg)
                }),
        };
        let widgets = view_output!();

        let factory = FactoryVecDeque::builder()
            .launch(widgets.cards_box.clone())
            .forward(sender.input_sender(), |output| HomeMsg::CardAction(output));

        model.playlist_cards = factory;

        sender.input(HomeMsg::LoadPlaylists);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        trace!("Home: {message:?}");
        match message {
            HomeMsg::LoadPlaylists => {
                sender.command(|out, _shutdown| async move {
                    match get_recommend_playlist().await {
                        Ok(playlists) => {
                            out.send(HomeCmdMsg::PlaylistsLoaded(playlists)).unwrap();
                        }
                        Err(e) => {
                            eprintln!("加载推荐歌单失败: {}", e);
                        }
                    }
                });
            }

            HomeMsg::CardAction(action) => match action {
                PlaylistCardOutput::Clicked(id) => {
                    let _ = sender.output(HomeOutput::OpenPlaylistDetail(id));
                }
                PlaylistCardOutput::ClickedPlaylist(playlist_id) => {
                    trace!("点击了歌单play: {}", playlist_id);
                    let _ = sender.output(HomeOutput::Playlist(PlaylistType::Playlist(playlist_id)));
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            HomeCmdMsg::PlaylistsLoaded(playlists) => {
                let mut guard = self.playlist_cards.guard();
                guard.clear();

                for playlist in playlists {
                    guard.push_back(PlaylistCardInit {
                        id: playlist.id,
                        cover_url: format!("{}?param=600y600", playlist.cover_url),
                        title: playlist.name.clone(),
                        show_play_button: true,
                    });
                }
            }
        }
    }
}
