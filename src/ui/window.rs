//! Main component of the application.
use std::sync::Arc;
use std::sync::Mutex;

use flume::Sender;
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::{AdwApplicationWindowExt, AdwDialogExt};
use relm4::gtk::prelude::{BoxExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk::{self, Box, Orientation, Stack, StackTransitionType, glib};
use relm4::{
    ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent, adw,
};

use relm4::Component;

use crate::api::{Artist, UserInfo, get_user_info};
use crate::db::Db;
use crate::player::PlayerFacade;
use crate::player::messages::{PlayerCommand, PlayerEvent};
use crate::ui::artist::{ArtistPage, ArtistPageOutput};
use crate::ui::collection::{Collection, CollectionMsg, CollectionOutput};
use crate::ui::comments::CommentsPage;
use crate::ui::components::artist_dialog::ArtistDialog;
use crate::ui::components::collect_dialog::CollectDialog;
use crate::ui::explore::{Explore, ExploreOutput};
use crate::ui::header::{Header, HeaderMsg, HeaderOutput};
use crate::ui::home::{Home, HomeOutput};
use crate::ui::model::{PlaySource, PlaylistType};
use crate::ui::player::PlayerPageOutput;
use crate::ui::fullscreen_lyric::{FullscreenLyricPage, FullscreenLyricMsg, FullscreenLyricOutput};
use crate::ui::route::{AppRoute, DetailCtrl, SidebarState};
use crate::ui::setting::{Settings, SettingsOutput};
use crate::ui::playlist_detail::{PlaylistDetail, PlaylistDetailOutput};
use crate::ui::sidebar::{Sidebar, SidebarMsg, SidebarOutput};

relm4::new_action_group!(pub WindowActionGroup, "win");
relm4::new_stateless_action!(pub CloseAction, WindowActionGroup, "close");

#[derive(Debug)]
pub enum WindowMsg {
    NavigateTo(AppRoute),
    GoBack,

    OpenSettings,
    OpenArtistDialog(Vec<Artist>),

    PlayerEventReceived(PlayerEvent),
    SendCommandToPlayer(PlayerCommand),
    SettingEventReceived(SettingsOutput),

    LoadUserInfo,
    UserInfoLoaded(UserInfo),

    CollectSong(u64),

    ShowToast(String),

    /// 循环侧栏状态（半展开→全覆盖→全收起）
    CycleSidebarState,
    /// 全屏歌词页输出
    FullscreenLyricEvent(FullscreenLyricOutput),
}

pub struct Window {
    main_window: adw::ApplicationWindow,
    overlay_split_view: adw::OverlaySplitView,
    toast_overlay: adw::ToastOverlay,

    settings_dialog: Controller<Settings>,
    artist_dialog: Option<relm4::Controller<ArtistDialog>>,
    collect_dialog: Option<Controller<CollectDialog>>,

    pub sidebar: Controller<Sidebar>,
    pub header: Controller<Header>,
    home_ctrl: Controller<Home>,
    explore_ctrl: Controller<Explore>,
    collection_ctrl: Controller<Collection>,

    detail_ctrl: Option<DetailCtrl>,

    history: Vec<AppRoute>,
    current_route: AppRoute,

    content_stack: Stack,
    detail_container: Box,

    player_cmd_tx: Sender<PlayerCommand>,
    user_info: Option<Arc<UserInfo>>,
    db: Arc<Mutex<Db>>,

    /// 全屏歌词页控制器
    fullscreen_lyric: Option<Controller<FullscreenLyricPage>>,
    /// 全屏歌词页的 overlay 容器
    fullscreen_overlay: gtk::Box,
    /// 侧栏状态
    sidebar_state: SidebarState,

    /// 缓存当前播放歌曲（用于新创建的全屏歌词页）
    current_song: Option<crate::api::Song>,
    /// 缓存当前播放状态
    current_is_playing: bool,
    /// 缓存当前播放位置
    current_position: u64,
    /// 缓存当前歌曲时长
    current_duration: u64,
}

#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = (String, Arc<Mutex<Db>>);
    type Input = WindowMsg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_default_height: 700,
            set_default_width: 850,

            // 全屏 overlay 覆盖整个窗口（包括 header）
            #[wrap(Some)]
            #[name(window_overlay)]
            set_content = &gtk::Overlay {
                #[wrap(Some)]
                #[name(toast_overlay)]
                set_child = &adw::ToastOverlay {
                    #[name(overlay_split_view)]
                    #[wrap(Some)]
                    set_child = &adw::OverlaySplitView {
                        set_sidebar_width_fraction: 0.30,
                        set_min_sidebar_width: 350.0,
                        set_max_sidebar_width: 400.0,

                        set_sidebar: Some(model.sidebar.widget()),

                        #[wrap(Some)]
                        set_content = &adw::ToolbarView {

                            add_top_bar: model.header.widget(),

                            #[name(content_stack)]
                            #[wrap(Some)]
                            set_content = &Stack {
                                set_transition_type: StackTransitionType::Crossfade,

                                add_named[Some("home")] = model.home_ctrl.widget() {},
                                add_named[Some("explore")] = model.explore_ctrl.widget() {},
                                add_named[Some("collection")] = model.collection_ctrl.widget() {},

                                #[name(detail_container)]
                                add_named[Some("detail")] = &Box {
                                    set_orientation: Orientation::Vertical,
                                }
                            }
                        }
                    },
                },

                // 全屏歌词页 overlay 层（默认隐藏，覆盖整个窗口包括 header）
                #[name(fullscreen_overlay)]
                add_overlay = &gtk::Box {
                    set_visible: false,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                },
            }
        }
    }

    fn init(
        (cookie, db): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let app = relm4::main_adw_application();
        app.set_accelerators_for_action::<CloseAction>(&["<Ctrl>W"]);

        let mut action_group = RelmActionGroup::<WindowActionGroup>::new();
        let close_action = RelmAction::<CloseAction>::new_stateless(glib::clone!(
            #[weak]
            root,
            move |_| root.close()
        ));

        let loaded_user = UserInfo::load_from_disk();
        let user_arc = loaded_user.map(Arc::new);
        let default_user = user_arc.clone().unwrap_or_else(|| {
            Arc::new(UserInfo {
                id: 0,
                name: "未登录".to_string(),
                avatar_url: "".to_string(),
            })
        });
        action_group.add_action(close_action);
        action_group.register_for_widget(&root);

        let sidebar = Sidebar::builder()
            .launch(())
            // 【修改】添加 forward 处理 Sidebar 的输出
            .forward(sender.input_sender(), |msg| {
                eprintln!("Sidebar output: {:?}", msg);
                match msg {
                    SidebarOutput::PlayerCommand(cmd) => {
                        eprintln!("Sidebar output: {:?}", cmd);
                        // 把 UI 指令翻译成后端指令
                        match cmd {
                            PlayerPageOutput::TogglePlay => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::TogglePlayPause)
                            }
                            PlayerPageOutput::NextTrack => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::Next)
                            }
                            PlayerPageOutput::PrevTrack => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::Previous)
                            }
                            PlayerPageOutput::Seek(val) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::Seek(val))
                            }
                            PlayerPageOutput::Remove(index) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::Remove(index))
                            }
                            PlayerPageOutput::PlayAt(index) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::PlayAt(index))
                            }
                            PlayerPageOutput::Navigate(app_route) => {
                                WindowMsg::NavigateTo(app_route)
                            }
                            PlayerPageOutput::OpenArtistDialog(artists) => {
                                WindowMsg::OpenArtistDialog(artists)
                            }
                            PlayerPageOutput::ToggleLike(id, liked) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::LikeSong { song_id: id, liked })
                            }
                            PlayerPageOutput::SetMode(mode) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::SetPlayMode(mode))
                            }
                            PlayerPageOutput::SetLoop(enabled) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::SetLoop(enabled))
                            }
                            PlayerPageOutput::CollectSong(id) => {
                                WindowMsg::CollectSong(id)
                            }
                        }
                    }
                }
            });

        let header =
            Header::builder()
                .launch(default_user.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    HeaderOutput::GoBack => WindowMsg::GoBack,
                    HeaderOutput::NavigateTo(route) => WindowMsg::NavigateTo(route),
                    HeaderOutput::CycleSidebarState => WindowMsg::CycleSidebarState,
                    HeaderOutput::OpenSettings => WindowMsg::OpenSettings,
                });

        let settings_dialog =
            Settings::builder()
                .launch(())
                .forward(sender.input_sender(), |output| {
                    WindowMsg::SettingEventReceived(output)
                });

        let home_ctrl =
            Home::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    HomeOutput::OpenPlaylistDetail(id) => {
                        WindowMsg::NavigateTo(AppRoute::PlaylistDetail(PlaylistType::Playlist(id)))
                    }
                    HomeOutput::Playlist(id) => {
                        WindowMsg::SendCommandToPlayer(PlayerCommand::Play {
                            source: PlaySource::ById(id),
                            start_index: 0,
                        })
                    }
                });

        let explore_ctrl = Explore::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ExploreOutput::OpenPlaylistDetail(id) => {
                    WindowMsg::NavigateTo(AppRoute::PlaylistDetail(PlaylistType::Playlist(id)))
                }
            });
        let collection_ctrl = Collection::builder().launch((default_user.clone(), db.clone())).forward(
            sender.input_sender(),
            |msg| match msg {
                CollectionOutput::OpenPlaylistDetail(playlist_type) => {
                    WindowMsg::NavigateTo(AppRoute::PlaylistDetail(playlist_type))
                }
                CollectionOutput::Playlist(id) => {
                    WindowMsg::SendCommandToPlayer(PlayerCommand::Play {
                        source: PlaySource::ById(id),
                        start_index: 0,
                    })
                }
            },
        );

        // 把 Window 的 sender 转成 PlayerEvent
        let player_event_sender = sender.input_sender().clone().into();
        let player_cmd_tx = PlayerFacade::start(player_event_sender, db.clone());

        let mut model = Self {
            main_window: root.clone(),
            sidebar,
            header,
            home_ctrl,
            detail_ctrl: None,
            history: Vec::new(),
            current_route: AppRoute::Home,
            content_stack: Stack::default(),
            detail_container: Box::default(),
            explore_ctrl,
            collection_ctrl,
            player_cmd_tx,
            overlay_split_view: adw::OverlaySplitView::default(),
            toast_overlay: adw::ToastOverlay::default(),
            settings_dialog,
            artist_dialog: None,
            collect_dialog: None,
            user_info: None,
            db,
            fullscreen_lyric: None,
            fullscreen_overlay: gtk::Box::default(),
            sidebar_state: SidebarState::HalfExpanded,
            current_song: None,
            current_is_playing: false,
            current_position: 0,
            current_duration: 0,
        };

        let widgets = view_output!();
        model.content_stack = widgets.content_stack.clone();
        model.detail_container = widgets.detail_container.clone();
        model.overlay_split_view = widgets.overlay_split_view.clone();
        model.toast_overlay = widgets.toast_overlay.clone();
        model.fullscreen_overlay = widgets.fullscreen_overlay.clone();

        if cookie.is_empty() {
            model.settings_dialog.widget().present(Some(&root));

            model.user_info = user_arc;
            eprintln!("No cookie found. Please open settings to set your cookie.");
        } else {
            sender.input(WindowMsg::LoadUserInfo);
            UserInfo::load_from_disk().map(|user_info| {
                model.user_info = Some(Arc::new(user_info));
            });
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            WindowMsg::NavigateTo(route) => {
                if self.current_route == route {
                    return;
                }
                match route {
                    AppRoute::Home | AppRoute::Explore | AppRoute::Collection => {
                        self.history.clear();
                    }
                    _ => {
                        self.history.push(self.current_route.clone());
                    }
                }

                self.current_route = route;
                self.render_current_route(&sender);
            }
            WindowMsg::GoBack => {
                if let Some(prev_route) = self.history.pop() {
                    self.current_route = prev_route;
                    self.render_current_route(&sender);
                }
            }
            WindowMsg::PlayerEventReceived(player_event) => {
                // 缓存当前播放状态
                match &player_event {
                    PlayerEvent::TimeUpdated { position, duration } => {
                        self.current_position = *position;
                        self.current_duration = *duration;
                    }
                    PlayerEvent::TrackChanged { song, .. } => {
                        self.current_song = Some(song.clone());
                    }
                    PlayerEvent::StateChanged(state) => {
                        self.current_is_playing =
                            *state == crate::player::messages::PlaybackState::Playing;
                    }
                    _ => {}
                }

                // 转发给侧栏
                self.sidebar.emit(SidebarMsg::PlayerEvent(player_event.clone()));
                // 如果全屏歌词页打开，也转发给它
                if let Some(ref fl) = self.fullscreen_lyric {
                    match &player_event {
                        PlayerEvent::TimeUpdated { position, duration } => {
                            fl.emit(FullscreenLyricMsg::TimeUpdated {
                                position: *position,
                                duration: *duration,
                            });
                        }
                        PlayerEvent::TrackChanged { song, .. } => {
                            fl.emit(FullscreenLyricMsg::LoadTrack(song.clone()));
                        }
                        PlayerEvent::StateChanged(state) => {
                            fl.emit(FullscreenLyricMsg::UpdatePlayback(
                                *state == crate::player::messages::PlaybackState::Playing,
                            ));
                        }
                        _ => {}
                    }
                }
            }
            WindowMsg::SendCommandToPlayer(player_command) => {
                if let Err(e) = self.player_cmd_tx.send(player_command) {
                    log::error!("Cannot send command to player: {}", e);
                }
            }
            WindowMsg::OpenSettings => {
                self.settings_dialog
                    .widget()
                    .present(Some(&self.main_window));
            }
            WindowMsg::SettingEventReceived(_settings_output) => {}
            WindowMsg::LoadUserInfo => {
                let sender_clone = sender.clone();
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(user_info) = get_user_info().await {
                        sender_clone.input(WindowMsg::UserInfoLoaded(user_info));
                    }
                });
            }
            WindowMsg::UserInfoLoaded(user_info) => {
                let new_arc = Arc::new(user_info);
                self.user_info = Some(new_arc.clone());
                self.user_info.as_ref().unwrap().save_to_disk();
                self.header.emit(HeaderMsg::UpdateUserInfo(new_arc.clone()));
                self.collection_ctrl
                    .emit(CollectionMsg::UpdateUserInfo(new_arc.clone()));
            }
            // update 里改成这样
            WindowMsg::OpenArtistDialog(artists) => {
                let artist_dialog = ArtistDialog::builder()
                    .launch(artists)
                    .forward(sender.input_sender(), |id| {
                        WindowMsg::NavigateTo(AppRoute::Artist(id))
                    });
                artist_dialog.widget().present(Some(&self.main_window));
                self.artist_dialog = Some(artist_dialog); // 存起来，防止被 drop
            }
            WindowMsg::CollectSong(id) => {
                self.collect_dialog = None;
                let uid = self.user_info.as_ref().map(|u| u.id).unwrap_or(0);
                let dialog = CollectDialog::builder()
                    .launch((id, uid))
                    .forward(sender.input_sender(), WindowMsg::ShowToast);
                dialog.widget().present(Some(&self.main_window));
                self.collect_dialog = Some(dialog);
            }
            WindowMsg::ShowToast(msg) => {
                self.toast_overlay.add_toast(adw::Toast::new(&msg));
            }

            WindowMsg::CycleSidebarState => {
                self.sidebar_state = self.sidebar_state.next();
                self.apply_sidebar_state(&sender);
            }

            WindowMsg::FullscreenLyricEvent(output) => {
                match output {
                    FullscreenLyricOutput::Close => {
                        self.close_fullscreen_lyric();
                    }
                    FullscreenLyricOutput::Seek(ms) => {
                        if let Err(e) = self.player_cmd_tx.send(PlayerCommand::Seek(ms)) {
                            log::error!("Cannot send seek command: {}", e);
                        }
                    }
                    FullscreenLyricOutput::PrevTrack => {
                        if let Err(e) = self.player_cmd_tx.send(PlayerCommand::Previous) {
                            log::error!("Cannot send prev command: {}", e);
                        }
                    }
                    FullscreenLyricOutput::NextTrack => {
                        if let Err(e) = self.player_cmd_tx.send(PlayerCommand::Next) {
                            log::error!("Cannot send next command: {}", e);
                        }
                    }
                    FullscreenLyricOutput::TogglePlay => {
                        if let Err(e) = self.player_cmd_tx.send(PlayerCommand::TogglePlayPause) {
                            log::error!("Cannot send toggle play command: {}", e);
                        }
                    }
                    FullscreenLyricOutput::ToggleLike(id, liked) => {
                        if let Err(e) = self.player_cmd_tx.send(PlayerCommand::LikeSong { song_id: id, liked }) {
                            log::error!("Cannot send like command: {}", e);
                        }
                    }
                }
            }
        }
    }
}

impl Window {
    fn render_current_route(&mut self, sender: &ComponentSender<Self>) {
        match &self.current_route {
            AppRoute::Home => {
                self.content_stack.set_visible_child_name("home");
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }
                self.detail_ctrl = None;
            }
            AppRoute::Explore => {
                self.content_stack.set_visible_child_name("explore");
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }
                self.detail_ctrl = None;
            }
            AppRoute::Collection => {
                self.content_stack.set_visible_child_name("collection");
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }
                self.detail_ctrl = None;
            }
            AppRoute::PlaylistDetail(playlist) => {
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }

                let db = self.db.clone();
                let user_id = self.user_info.as_ref().map(|u| u.id).unwrap_or(0);
                let detail = PlaylistDetail::builder().launch((playlist.clone(), db, user_id)).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        PlaylistDetailOutput::PlayQueue{tracks, track_ids, start_index, playlist} => {
                            WindowMsg::SendCommandToPlayer(PlayerCommand::Play {
                                source: PlaySource::LazyQueue {
                                    tracks,
                                    track_ids,
                                    playlist,
                                },
                                start_index,
                            })
                        }
                        PlaylistDetailOutput::ShowToast(text) => {
                            WindowMsg::ShowToast(text)
                        }
                    },
                );

                self.detail_container.append(detail.widget());
                self.content_stack.set_visible_child_name("detail");
                self.detail_ctrl = Some(DetailCtrl::Playlist(detail));
            }
            AppRoute::Artist(id) => {
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }


                let detail = ArtistPage::builder().launch(*id).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        ArtistPageOutput::PlayQueue {
                            artist_id,
                            artist_name,
                            songs,
                            start_index,
                        } => WindowMsg::SendCommandToPlayer(PlayerCommand::Play {
                            source: PlaySource::ArtistQueue {
                                songs: songs,
                                artist_name: artist_name,
                                artist_id: artist_id,
                            },
                            start_index: start_index,
                        }),
                        ArtistPageOutput::Navigate(app_route) => WindowMsg::NavigateTo(app_route),
                    },
                );

                self.detail_container.append(detail.widget());
                self.content_stack.set_visible_child_name("detail");
                self.detail_ctrl = Some(DetailCtrl::Artist(detail));
            }
            AppRoute::Comments(song_id) => {
                eprintln!("Comments: {}", song_id);
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }

                let detail = CommentsPage::builder().launch(*song_id).forward(
                    sender.input_sender(),
                    |_msg| WindowMsg::ShowToast(String::new()),
                );

                self.detail_container.append(detail.widget());
                self.content_stack.set_visible_child_name("detail");
                self.detail_ctrl = Some(DetailCtrl::Comments(detail));
            },
        }

        let can_go_back = !self.history.is_empty();

        self.header.emit(HeaderMsg::UpdateState {
            can_go_back,
            active_tab: self.current_route.clone(),
        });
    }

/// 打开全屏歌词页
    fn open_fullscreen_lyric(&mut self, sender: &ComponentSender<Self>) {
        if self.fullscreen_lyric.is_some() {
            return;
        }

        let fl = FullscreenLyricPage::builder()
            .launch(())
            .forward(sender.input_sender(), WindowMsg::FullscreenLyricEvent);

        if let Some(ref song) = self.current_song {
            fl.emit(FullscreenLyricMsg::LoadTrack(song.clone()));
        }
        fl.emit(FullscreenLyricMsg::UpdatePlayback(self.current_is_playing));
        fl.emit(FullscreenLyricMsg::TimeUpdated {
            position: self.current_position,
            duration: self.current_duration,
        });

        while let Some(child) = self.fullscreen_overlay.first_child() {
            self.fullscreen_overlay.remove(&child);
        }
        self.fullscreen_overlay.append(fl.widget());
        self.fullscreen_overlay.set_visible(true);

        // 隐藏整个正常UI内容
        self.header.widget().set_visible(false);
        self.overlay_split_view.set_show_sidebar(false);
        self.content_stack.set_visible(false);

        // 全屏
        // self.main_window.fullscreen();

        self.fullscreen_lyric = Some(fl);
    }

    /// 关闭全屏歌词页
    fn close_fullscreen_lyric(&mut self) {
        if self.fullscreen_lyric.is_some() {
            while let Some(child) = self.fullscreen_overlay.first_child() {
                self.fullscreen_overlay.remove(&child);
            }
            self.fullscreen_overlay.set_visible(false);
            self.fullscreen_lyric = None;

            // 恢复 UI
            self.header.widget().set_visible(true);
            self.overlay_split_view.set_show_sidebar(true);
            self.content_stack.set_visible(true);

            // 取消全屏
            self.main_window.unfullscreen();

            self.sidebar_state = SidebarState::HalfExpanded;
        }
    }

    /// 应用侧栏状态
    fn apply_sidebar_state(&mut self, sender: &ComponentSender<Self>) {
        match self.sidebar_state {
            SidebarState::HalfExpanded => {
                // 正常显示侧栏，显示播放器页
                self.overlay_split_view.set_show_sidebar(true);
                self.sidebar.emit(SidebarMsg::SwitchPage(crate::ui::route::SidebarPage::Player));
            }
            SidebarState::FullCover => {
                // 全覆盖：显示全屏歌词页 overlay（覆盖整个窗口包括 header）
                self.open_fullscreen_lyric(sender);
            }
            SidebarState::FullCollapsed => {
                // 全收起：隐藏侧栏
                self.overlay_split_view.set_show_sidebar(false);
            }
        }
    }
}
