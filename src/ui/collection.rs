use std::sync::Arc;

use relm4::gtk::{FlowBox, prelude::*};
use relm4::prelude::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt, gtk};

use crate::api::{
    Album, Playlist, Song, UserDetails, UserInfo, get_user_detail, get_user_playlist,
    get_user_sub_album,
};
use crate::ui::components::image::AsyncImage;
use crate::ui::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};
use crate::ui::model::PlaylistType;

pub struct Collection {
    user_info: Arc<UserInfo>,
    user_details: Option<UserDetails>,
    // 歌单页面中的两个独立列表
    created_playlists: FactoryVecDeque<PlaylistCard>,
    collected_playlists: FactoryVecDeque<PlaylistCard>,
    albums: FactoryVecDeque<PlaylistCard>,
}

#[derive(Debug)]
pub enum CollectionMsg {
    LoadUserPlaylist,
    LoadUserDetail,
    LoadUserSubAlbums,
    UpdateUserInfo(Arc<UserInfo>),

    CardAction(PlaylistCardOutput, PlaylistType),
}

#[derive(Debug)]
pub enum CollectionCmdMsg {
    LoadUserPlaylisted(Vec<Playlist>),
    LoadUserDetailled(UserDetails),
    LoadUserSubAlbumed(Vec<Album>),
}

#[derive(Debug)]
pub enum CollectionOutput {
    OpenPlaylistDetail(PlaylistType),
    Playlist(PlaylistType),
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

            // 最外层的大容器
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 24,
                set_margin_all: 32,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 16,


                    AsyncImage {
                        set_width_request: 160,
                        set_height_request: 160,
                        #[watch]
                        set_url: format!("{}?param=160y160", model.user_info.avatar_url.clone()),
                        set_placeholder_icon: "avatar-default-symbolic", // 这里替换为加载用户
                        set_corner_radius: 80.0,
                    },

                    // 昵称与简介
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,
                        set_spacing: 4,

                        gtk::Label {
                            #[watch]
                            set_label: &model.user_info.name.clone(),
                            set_xalign: 0.0,
                            add_css_class: "title-1",
                        },
                        gtk::Label {
                            set_label: "记录你的音乐足迹",
                            set_xalign: 0.0,
                            add_css_class: "dim-label",
                        },
                        gtk::Box{
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk::Label {
                                #[watch]
                                set_label: &model.user_details.as_ref()
                                    .map(|d| format!("关注: {}", d.follows))
                                    .unwrap_or_else(|| "关注: --".to_string()),
                                set_xalign: 0.0,
                                add_css_class: "dim-label",
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &model.user_details.as_ref()
                                    .map(|d| format!("粉丝: {}", d.followeds))
                                    .unwrap_or_else(|| "粉丝: --".to_string()),
                                set_xalign: 0.0,
                                add_css_class: "dim-label",
                            }
                        }

                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,

                    // 1. Tab 切换器
                    gtk::StackSwitcher {
                        set_stack: Some(&view_stack),
                        set_halign: gtk::Align::Start,
                    },

                    // 2. 视图栈 (根据 Tab 切换内容)
                    #[name(view_stack)]
                    gtk::Stack {
                        set_vexpand: true,
                        set_transition_type: gtk::StackTransitionType::Crossfade,

                        // ----- Tab 1: 我的歌单 -----
                        add_titled[Some("playlists"), "我的歌单"] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 32,
                            set_margin_top: 12,

                            // [我创建的] 区域
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,

                                gtk::Label {
                                    set_label: "我创建的",
                                    set_xalign: 0.0,
                                    add_css_class: "title-2",
                                },
                                #[name(created_flow_box)]
                                gtk::FlowBox {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_row_spacing: 16,
                                    set_column_spacing: 16,
                                    set_min_children_per_line: 2,
                                    set_max_children_per_line: 7,
                                }
                            },

                            // [我收藏的] 区域
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,

                                gtk::Label {
                                    set_label: "我收藏的",
                                    set_xalign: 0.0,
                                    add_css_class: "title-2",
                                },
                                #[name(collected_flow_box)]
                                gtk::FlowBox {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_row_spacing: 16,
                                    set_column_spacing: 16,
                                    set_min_children_per_line: 2,
                                    set_max_children_per_line: 7,
                                }
                            }
                        },

                        // ----- Tab 2: 我的专辑 -----
                        add_titled[Some("albums"), "专辑"] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            // set_valign: gtk::Align::Center,

                            // gtk::Label { set_label: "你还没有收藏任何专辑哦", add_css_class: "dim-label" },
                            #[name(album_flow_box)]
                            gtk::FlowBox {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_row_spacing: 16,
                                set_column_spacing: 16,
                                set_min_children_per_line: 2,
                                set_max_children_per_line: 7,
                            }

                        },

                        // ----- Tab 3: 我的 MV -----
                        add_titled[Some("mvs"), "MV"] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Center,
                            gtk::Label { set_label: "你还没有收藏任何 MV 哦", add_css_class: "dim-label" }
                            // 未来在这里添加 MV 的 FlowBox
                        }
                    }
                }
            }
        }
    }

    fn init(
        user_info: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create model first with placeholder factories
        let mut model = Self {
            user_info: user_info,
            created_playlists: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| {
                    CollectionMsg::CardAction(msg, PlaylistType::Playlist(0))
                }),
            collected_playlists: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| {
                    CollectionMsg::CardAction(msg, PlaylistType::Playlist(0))
                }),
            albums: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| {
                    CollectionMsg::CardAction(msg, PlaylistType::Album(0))
                }),
            user_details: None,
        };

        let widgets = view_output!();
        model.created_playlists = FactoryVecDeque::builder()
            .launch(widgets.created_flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::CardAction(output, PlaylistType::Playlist(0))
            });

        model.collected_playlists = FactoryVecDeque::builder()
            .launch(widgets.collected_flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::CardAction(output, PlaylistType::Playlist(0))
            });

        model.albums = FactoryVecDeque::builder()
            .launch(widgets.album_flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::CardAction(output, PlaylistType::Album(0))
            });

        sender.input(CollectionMsg::LoadUserPlaylist);
        sender.input(CollectionMsg::LoadUserDetail);
        sender.input(CollectionMsg::LoadUserSubAlbums);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CollectionMsg::LoadUserPlaylist => {
                let user_id = self.user_info.id;
                if user_id == 0 {
                    return;
                }
                sender.command(move |out, _shutdown| async move {
                    if let Ok(playlists) = get_user_playlist(user_id).await {
                        let _ = out.send(CollectionCmdMsg::LoadUserPlaylisted(playlists));
                    }
                });
            }
            CollectionMsg::UpdateUserInfo(user_info) => {
                self.user_info = user_info;
                sender.input(CollectionMsg::LoadUserPlaylist);
            }
            CollectionMsg::CardAction(playlist_card_output, playlist_type) => {
                match playlist_card_output {
                    PlaylistCardOutput::Clicked(id) => {
                        sender
                            .output(CollectionOutput::OpenPlaylistDetail(match playlist_type {
                                PlaylistType::Playlist(_) => PlaylistType::Playlist(id),
                                PlaylistType::Album(_) => PlaylistType::Album(id),
                                PlaylistType::DailyRecommend => PlaylistType::DailyRecommend,
                            }))
                            .unwrap();
                    }
                    PlaylistCardOutput::ClickedPlaylist(id) => {
                        sender
                            .output(CollectionOutput::Playlist(match playlist_type {
                                PlaylistType::Playlist(_) => PlaylistType::Playlist(id),
                                PlaylistType::Album(_) => PlaylistType::Album(id),
                                PlaylistType::DailyRecommend => PlaylistType::DailyRecommend,
                            }))
                            .unwrap();
                    }
                }
            }
            CollectionMsg::LoadUserDetail => {
                let user_id = self.user_info.id;
                sender.command(move |out, _shutdown| async move {
                    if let Ok(user_details) = get_user_detail(user_id).await {
                        let _ = out.send(CollectionCmdMsg::LoadUserDetailled(user_details));
                    }
                });
            }
            CollectionMsg::LoadUserSubAlbums => {
                sender.command(|out, _shutdown| async move {
                    if let Ok(user_sub_albums) = get_user_sub_album().await {
                        let _ = out.send(CollectionCmdMsg::LoadUserSubAlbumed(user_sub_albums));
                    }
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CollectionCmdMsg::LoadUserPlaylisted(playlists) => {
                let mut created_guard = self.created_playlists.guard();
                let mut collected_guard = self.collected_playlists.guard();

                created_guard.clear();
                collected_guard.clear();

                let my_user_id = self.user_info.id;

                for playlist in playlists {
                    let card = PlaylistCardInit {
                        id: playlist.id,
                        cover_url: format!("{}?param=600y600", playlist.cover_url),
                        title: playlist.name.clone(),
                        show_play_button: true,
                    };

                    if playlist.creator_id == my_user_id {
                        created_guard.push_back(card);
                    } else {
                        collected_guard.push_back(card);
                    }
                }
            }
            CollectionCmdMsg::LoadUserDetailled(user_details) => {
                eprintln!("用户详情加载完成: {:?}", user_details);
                self.user_details = Some(user_details);
            }
            CollectionCmdMsg::LoadUserSubAlbumed(albums) => {
                eprintln!("用户专辑加载完成: {:?}", albums);
                let mut guard = self.albums.guard();
                guard.clear();
                for album in albums {
                    let card = PlaylistCardInit {
                        id: album.id,
                        cover_url: format!("{}?param=600y600", album.cover_url),
                        title: album.name.clone(),
                        show_play_button: true,
                    };
                    guard.push_back(card);
                }
            }
        }
    }
}
