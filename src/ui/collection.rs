use std::sync::Arc;

use relm4::gtk::{FlowBox, prelude::*};
use relm4::prelude::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt, gtk};

use crate::api::{Playlist, UserInfo, get_user_playlist};
use crate::ui::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};

pub struct Collection {
    user_info: Arc<UserInfo>,
    playlists: FactoryVecDeque<PlaylistCard>,
}
#[derive(Debug)]
pub enum CollectionMsg {
    LoadUserPlaylist,
    LoadUserPlaylisted(Vec<Playlist>),

    UpdateUserInfo(Arc<UserInfo>),
    PlaylistClicked(PlaylistCardOutput),
}
#[derive(Debug)]
pub enum CollectionCmdMsg {}
#[derive(Debug)]
pub enum CollectionOutput {
    OpenPlaylistDetail(u64),
}

#[relm4::component(pub)]
impl Component for Collection {
    type Init = Arc<UserInfo>;
    type Input = CollectionMsg;
    type CommandOutput = CollectionCmdMsg;
    type Output = CollectionOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_vexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 16,

                gtk::Label {
                    set_label: "我的收藏",
                    add_css_class: "title-2",
                    set_margin_bottom: 12,
                },
                #[name(flow_box)]
                gtk::FlowBox {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_row_spacing: 16,
                    set_column_spacing: 16,
                    set_homogeneous: true, // 让所有卡片保持相同大小（推荐）
                    set_selection_mode: gtk::SelectionMode::None, // 不需要选中高亮效果
                    set_min_children_per_line: 2, // 最少显示几列 (窗口极小时)
                    set_max_children_per_line: 6, // 最多显示几列 (窗口极大时)
                }
            }
        }
    }

    fn init(
        user_info: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 1. 先创建一个空的 model
        let mut model = Self {
            user_info,
            playlists: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |output| {
                    CollectionMsg::PlaylistClicked(output)
                }),
        };

        let widgets = view_output!();
        let factory = FactoryVecDeque::builder()
            .launch(widgets.flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::PlaylistClicked(output)
            });

        // let mut model = model;
        model.playlists = factory;
        sender.input(CollectionMsg::LoadUserPlaylist);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CollectionMsg::LoadUserPlaylist => {
                let sender_clone = sender.clone();
                let user_id = self.user_info.id;
                if user_id == 0 {
                    eprintln!("Invalid user ID: 0");
                    return;
                }
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(playlists) = get_user_playlist(user_id.clone()).await {
                        sender_clone.input(CollectionMsg::LoadUserPlaylisted(playlists));
                    }
                });
            }
            CollectionMsg::LoadUserPlaylisted(playlists) => {
                let mut guard = self.playlists.guard();
                guard.clear();

                for playlist in playlists {
                    guard.push_back(PlaylistCardInit {
                        id: playlist.id,
                        cover_url: format!("{}?param=600y600", playlist.cover_url),
                        title: playlist.name.clone(),
                    });
                }
            }
            CollectionMsg::UpdateUserInfo(user_info) => {
                self.user_info = user_info;
                sender.input(CollectionMsg::LoadUserPlaylist);
            }
            CollectionMsg::PlaylistClicked(playlist_card_output) => match playlist_card_output {
                PlaylistCardOutput::Clicked(id) => {
                    sender
                        .output(CollectionOutput::OpenPlaylistDetail(id))
                        .unwrap();
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}
