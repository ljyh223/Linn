use std::sync::Arc;

use relm4::gtk::{FlowBox, prelude::*};
use relm4::prelude::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt, gtk};

use crate::api::{
    Album, Playlist, UserDetails, UserInfo, get_user_detail, get_user_playlist, get_user_sub_album,
};
use crate::ui::components::image::AsyncImage;
use crate::ui::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};

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
    LoadUserPlaylisted(Vec<Playlist>),
    LoadUserDetailled(UserDetails),
    LoadUserSubAlbumed(Vec<Album>),

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

            // 最外层的大容器
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 24,
                set_margin_all: 32, // 给整个页面留出优雅的留白

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
                            // 假设 UserInfo 里面有 nickname 字段，请根据实际情况修改
                            #[watch]
                            set_label: &model.user_info.name.clone(), // 稍后可替换为 &model.user_info.nickname
                            set_xalign: 0.0,
                            add_css_class: "title-1", // 使用大字体 CSS
                        },
                        gtk::Label {
                            set_label: "记录你的音乐足迹",
                            set_xalign: 0.0,
                            add_css_class: "dim-label", // 次级灰色文字
                        },
                        gtk::Box{
                            set_orientation: gtk::Orientation::Horizontal,

                            gtk::Label {
                                #[watch]
                                set_label: &model.user_details.as_ref()
                                    .map(|d| format!("关注: {}", d.follows))
                                    .unwrap_or_else(|| "关注: --".to_string()),
                                set_xalign: 0.0,
                                add_css_class: "dim-label", // 次级灰色文字
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &model.user_details.as_ref()
                                    .map(|d| format!("粉丝: {}", d.followeds))
                                    .unwrap_or_else(|| "粉丝: --".to_string()),
                                set_xalign: 0.0,
                                add_css_class: "dim-label", // 次级灰色文字
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
                        set_halign: gtk::Align::Start, // 让 Tab 按钮靠左对齐，更现代
                    },

                    // 2. 视图栈 (根据 Tab 切换内容)
                    #[name(view_stack)]
                    gtk::Stack {
                        set_vexpand: true,
                        set_transition_type: gtk::StackTransitionType::Crossfade, // 切换时淡入淡出

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
                                    set_homogeneous: true,
                                    set_selection_mode: gtk::SelectionMode::None,
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
                                    set_homogeneous: true,
                                    set_selection_mode: gtk::SelectionMode::None,
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
                                set_homogeneous: true,
                                set_selection_mode: gtk::SelectionMode::None,
                                set_min_children_per_line: 2,
                                set_max_children_per_line: 7,
                                // set_hexpand: true, 
                                // set_vexpand: true, 
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
                .forward(sender.input_sender(), |msg| match msg {
                    PlaylistCardOutput::Clicked(id) => {
                        CollectionMsg::PlaylistClicked(PlaylistCardOutput::Clicked(id))
                    }
                }),
            collected_playlists: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| match msg {
                    PlaylistCardOutput::Clicked(id) => {
                        CollectionMsg::PlaylistClicked(PlaylistCardOutput::Clicked(id))
                    }
                }),
            albums: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| match msg {
                    PlaylistCardOutput::Clicked(id) => {
                        CollectionMsg::PlaylistClicked(PlaylistCardOutput::Clicked(id))
                    }
                }),
            user_details: None,
        };

        let widgets = view_output!();
        model.created_playlists = FactoryVecDeque::builder()
            .launch(widgets.created_flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::PlaylistClicked(output)
            });

        model.collected_playlists = FactoryVecDeque::builder()
            .launch(widgets.collected_flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::PlaylistClicked(output)
            });

        model.albums = FactoryVecDeque::builder()
            .launch(widgets.album_flow_box.clone())
            .forward(sender.input_sender(), |output| {
                CollectionMsg::PlaylistClicked(output)
            });

        sender.input(CollectionMsg::LoadUserPlaylist);
        sender.input(CollectionMsg::LoadUserDetail);
        sender.input(CollectionMsg::LoadUserSubAlbums);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CollectionMsg::LoadUserPlaylist => {
                let sender_clone = sender.clone();
                let user_id = self.user_info.id;
                if user_id == 0 {
                    return;
                }
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(playlists) = get_user_playlist(user_id).await {
                        sender_clone.input(CollectionMsg::LoadUserPlaylisted(playlists));
                    }
                });
            }
            CollectionMsg::LoadUserPlaylisted(playlists) => {
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
            CollectionMsg::LoadUserDetail => {
                let sender_clone = sender.clone();
                let user_id = self.user_info.id;
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(user_details) = get_user_detail(user_id).await {
                        sender_clone.input(CollectionMsg::LoadUserDetailled(user_details));
                    }
                });
            }
            CollectionMsg::LoadUserDetailled(user_details) => {
                eprintln!("用户详情加载完成: {:?}", user_details);
                self.user_details = Some(user_details);
            }
            CollectionMsg::LoadUserSubAlbums => {
                let sender_clone = sender.clone();
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(user_sub_albums) = get_user_sub_album().await {
                        sender_clone.input(CollectionMsg::LoadUserSubAlbumed(user_sub_albums));
                    }
                });
            }
            CollectionMsg::LoadUserSubAlbumed(sub_album) => {
                eprintln!("用户专辑加载完成: {:?}", sub_album);
                let mut guard = self.albums.guard();
                guard.clear();
                for album in sub_album {
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

    fn update_cmd(
        &mut self,
        _message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}
