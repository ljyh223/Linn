//! Main component of the application.
use std::sync::Arc;
use std::sync::Mutex;

use flume::Sender;
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::{AdwApplicationWindowExt, AdwDialogExt};
use relm4::gtk::prelude::{BoxExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk::{self, Box, Orientation, Stack, StackTransitionType, gio, glib};
use relm4::gtk::gio::prelude::SettingsExt;
use relm4::{
    ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent, adw,
};

use relm4::Component;

use crate::api::{Artist, Playlist, UserInfo, get_user_info};
use crate::db::{Db, SessionState};
use crate::player::{PlayerFacade, PlayerEventBus};
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
use crate::ui::fullscreen_lyric::{FullscreenLyricPage, FullscreenLyricMsg, FullscreenLyricOutput};
use crate::ui::mv_player::{MvPlayerOutput, MvPlayerPage};
use crate::ui::route::{AppRoute, DetailCtrl};
use crate::ui::setting::{Settings, SettingsOutput};
use crate::ui::playlist_detail::{PlaylistDetail, PlaylistDetailOutput};
use crate::ui::sidebar::{Sidebar, SidebarMsg, SidebarOutput};
use crate::ui::search::{Search, SearchMsg, SearchOutput};
use crate::utils::animate::Fade;
use crate::APPLICATION_ID;

relm4::new_action_group!(pub WindowActionGroup, "win");
relm4::new_stateless_action!(pub CloseAction, WindowActionGroup, "close");
relm4::new_stateless_action!(pub ToggleSidebarAction, WindowActionGroup, "toggle-sidebar");

/// 全屏歌词页淡入/淡出动画时长（ms）
const FULLSCREEN_FADE_MS: u64 = 350;

#[derive(Debug)]
pub enum WindowMsg {
    NavigateTo(AppRoute),
    GoBack,

    OpenSettings,
    OpenArtistDialog(Vec<Artist>),

    PlayerEventReceived(PlayerEvent),
    /// 直接来自 Sidebar 的播放器命令
    PlayerCommandReceived(PlayerCommand),
    SettingEventReceived(SettingsOutput),

    LoadUserInfo,
    UserInfoLoaded(UserInfo),

    CollectSong(u64),

    ShowToast(String),

    /// Ctrl+K：切换侧栏显示/隐藏
    ToggleSidebar,
    /// header 全屏按钮：进入/退出全屏歌词页
    ToggleFullscreen,
    /// 全屏页淡出结束后的清理信号
    FullscreenFadedOut,
    /// 在搜索页提交了搜索词
    /// 提交搜索（回车，默认单曲搜索）
    SearchSubmit(String),
    /// 搜索输入框内容变化（实时建议）
    SearchSuggestQuery(String),
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
    search_ctrl: Controller<Search>,

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
    /// 全屏歌词页淡入/淡出动画
    fullscreen_fade: Option<Fade>,
    /// 侧栏是否可见（仅由 Ctrl+K 控制）
    sidebar_visible: bool,

    /// 缓存当前播放歌曲（用于新创建的全屏歌词页）
    current_song: Option<crate::api::Song>,
    /// 缓存当前播放状态
    current_is_playing: bool,
    /// 缓存当前播放位置
    current_position: u64,
    /// 缓存当前歌曲时长
    current_duration: u64,
    /// 进入 MV 页时暂停了音乐，离开时是否需要恢复
    should_resume_music: bool,

    /// 上次播放会话镜像，用于持久化与启动恢复
    session: SessionState,
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
                                add_named[Some("search")] = model.search_ctrl.widget() {},

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
        app.set_accelerators_for_action::<ToggleSidebarAction>(&["<Ctrl>k"]);

        let mut action_group = RelmActionGroup::<WindowActionGroup>::new();
        let close_action = RelmAction::<CloseAction>::new_stateless(glib::clone!(
            #[weak]
            root,
            move |_| root.close()
        ));
        let window_sender = sender.input_sender().clone();
        let toggle_sidebar_action =
            RelmAction::<ToggleSidebarAction>::new_stateless(move |_| {
                let _ = window_sender.send(WindowMsg::ToggleSidebar);
            });

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
        action_group.add_action(toggle_sidebar_action);
        action_group.register_for_widget(&root);

        let sidebar = Sidebar::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                SidebarOutput::PlayerCommand(cmd) => {
                    WindowMsg::PlayerCommandReceived(cmd)
                }
                SidebarOutput::NavigateTo(route) => WindowMsg::NavigateTo(route),
                SidebarOutput::OpenArtistDialog(artists) => WindowMsg::OpenArtistDialog(artists),
                SidebarOutput::CollectSong(id) => WindowMsg::CollectSong(id),
            });

        let header =
            Header::builder()
                .launch(default_user.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    HeaderOutput::GoBack => WindowMsg::GoBack,
                    HeaderOutput::NavigateTo(route) => WindowMsg::NavigateTo(route),
                    HeaderOutput::ToggleFullscreen => WindowMsg::ToggleFullscreen,
                    HeaderOutput::OpenSettings => WindowMsg::OpenSettings,
                    HeaderOutput::SearchSubmit(query) => {
                        WindowMsg::SearchSubmit(query)
                    }
                    HeaderOutput::SearchChanged(query) => {
                        WindowMsg::SearchSuggestQuery(query)
                    }
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
                    HomeOutput::OpenDailyRecommend => {
                        WindowMsg::NavigateTo(AppRoute::PlaylistDetail(PlaylistType::DailyRecommend))
                    }
                    HomeOutput::OpenPlaylistType(playlist_type) => {
                        WindowMsg::NavigateTo(AppRoute::PlaylistDetail(playlist_type))
                    }
                    HomeOutput::Playlist(id) => {
                        WindowMsg::PlayerCommandReceived(PlayerCommand::Play {
                            source: PlaySource::ById(id),
                            start_index: 0,
                        })
                    }
                    HomeOutput::NavigateToArtist(id) => {
                        WindowMsg::NavigateTo(AppRoute::Artist(id))
                    }
                    HomeOutput::PlayDirectTracks(songs) => {
                        WindowMsg::PlayerCommandReceived(PlayerCommand::Play {
                            source: PlaySource::DirectTracks(Arc::new(songs)),
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
                    WindowMsg::PlayerCommandReceived(PlayerCommand::Play {
                        source: PlaySource::ById(id),
                        start_index: 0,
                    })
                }
            },
        );

        let search_ctrl = Search::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                SearchOutput::PlaySong(song) => {
                    WindowMsg::PlayerCommandReceived(PlayerCommand::Play {
                        source: PlaySource::DirectTracks(Arc::new(vec![song])),
                        start_index: 0,
                    })
                }
                SearchOutput::Navigate(route) => WindowMsg::NavigateTo(route),
            },
        );

        // 创建 PlayerEventBus，用于广播播放器事件
        let event_bus = PlayerEventBus::new();
        let player_event_sender: relm4::Sender<PlayerEvent> = event_bus.create_sender().into();
        let player_cmd_tx = PlayerFacade::start(player_event_sender, db.clone());

        // 启动时恢复上次播放（受设置开关控制，未登录时不恢复）
        if !cookie.is_empty() {
            let settings = gio::Settings::new(APPLICATION_ID);
            let restore_on_start = settings.boolean("restore-on-start");
            let auto_play_on_restore = settings.boolean("auto-play-on-restore");
            if restore_on_start {
                if let Some(session) = db.lock().unwrap().load_session() {
                    if !session.track_ids.is_empty() {
                        let _ = player_cmd_tx.send(PlayerCommand::RestoreSession {
                            track_ids: Arc::new(session.track_ids),
                            current_index: session.current_index,
                            autoplay: auto_play_on_restore,
                            playlist: Playlist {
                                id: session.playlist_id,
                                name: session.playlist_name,
                                cover_url: session.playlist_cover_url,
                                creator_name: session.playlist_creator_name,
                                creator_id: 0,
                                description: String::new(),
                                play_count: 0,
                            },
                        });
                    }
                }
            }
        }

        // Window 订阅 PlayerEvent
        let window_event_rx = event_bus.subscribe();
        let window_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            while let Ok(event) = window_event_rx.recv() {
                let _ = window_sender.send(WindowMsg::PlayerEventReceived(event));
            }
        });

        // Sidebar 订阅 PlayerEvent
        let sidebar_event_rx = event_bus.subscribe();
        let sidebar_sender = sidebar.sender().clone();
        std::thread::spawn(move || {
            while let Ok(event) = sidebar_event_rx.recv() {
                sidebar_sender.emit(SidebarMsg::PlayerEvent(event));
            }
        });

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
            search_ctrl,
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
            fullscreen_fade: None,
            sidebar_visible: false,
            current_song: None,
            current_is_playing: false,
            current_position: 0,
            current_duration: 0,
            should_resume_music: false,
            session: SessionState::default(),
        };

                let widgets = view_output!();
        model.content_stack = widgets.content_stack.clone();
        model.detail_container = widgets.detail_container.clone();
        model.overlay_split_view = widgets.overlay_split_view.clone();
        model.toast_overlay = widgets.toast_overlay.clone();
        model.fullscreen_overlay = widgets.fullscreen_overlay.clone();

        // 初始无播放音乐：侧栏默认隐藏（原生 show_sidebar 动画侧栏在布局内滑出）
        model.overlay_split_view.set_show_sidebar(false);

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
                // 离开 MV 页 → 恢复之前暂停的音乐
                if matches!(self.current_route, AppRoute::Mv(_)) && self.should_resume_music {
                    self.should_resume_music = false;
                    if let Err(e) = self.player_cmd_tx.send(PlayerCommand::Resume) {
                        log::error!("Cannot resume music: {}", e);
                    }
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
                // 离开 MV 页 → 恢复之前暂停的音乐
                if matches!(self.current_route, AppRoute::Mv(_)) && self.should_resume_music {
                    self.should_resume_music = false;
                    if let Err(e) = self.player_cmd_tx.send(PlayerCommand::Resume) {
                        log::error!("Cannot resume music: {}", e);
                    }
                }
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
                    PlayerEvent::TrackChanged {
                        song,
                        current_index,
                        ..
                    } => {
                        // 从"无歌"进入"有歌"：侧栏动画弹出
                        let was_empty = self.current_song.is_none();
                        self.current_song = Some(song.clone());
                        if was_empty && !self.sidebar_visible {
                            self.set_sidebar_visible(true);
                        }
                        self.session.current_index = *current_index;
                        self.db.lock().unwrap().save_session(&self.session);
                    }
                    PlayerEvent::StateChanged(state) => {
                        self.current_is_playing =
                            *state == crate::player::messages::PlaybackState::Playing;
                    }
                    PlayerEvent::ShowToast(msg) => {
                        self.toast_overlay.add_toast(adw::Toast::new(msg));
                    }
                    PlayerEvent::SetQueue {
                        tracks,
                        playlist,
                        start_index,
                    } => {
                        self.session.track_ids = tracks.iter().map(|s| s.id).collect();
                        self.session.current_index = *start_index;
                        self.session.playlist_id = playlist.id;
                        self.session.playlist_name = playlist.name.clone();
                        self.session.playlist_cover_url = playlist.cover_url.clone();
                        self.session.playlist_creator_name = playlist.creator_name.clone();
                        self.db.lock().unwrap().save_session(&self.session);
                    }
                    _ => {}
                }

                // 如果全屏歌词页打开，转发给它
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
            WindowMsg::PlayerCommandReceived(player_command) => {
                if let Err(e) = self.player_cmd_tx.send(player_command) {
                    log::error!("Cannot send command to player: {}", e);
                }
            }
            WindowMsg::OpenSettings => {
                self.settings_dialog
                    .widget()
                    .present(Some(&self.main_window));
            }
            WindowMsg::SettingEventReceived(output) => match output {
                SettingsOutput::UserCookieChanged(_) => {}
                SettingsOutput::SaveCookie => {}
            }
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

            WindowMsg::SearchSubmit(query) => {
                // 只有在搜索页时才转发给搜索页
                if self.current_route == AppRoute::Search {
                    self.search_ctrl.emit(SearchMsg::Submit(query));
                }
            }

            WindowMsg::SearchSuggestQuery(query) => {
                // 只有在搜索页时才转发给搜索页
                if self.current_route == AppRoute::Search {
                    self.search_ctrl.emit(SearchMsg::Suggest(query));
                }
            }

            WindowMsg::ToggleSidebar => {
                let target = !self.sidebar_visible;
                self.set_sidebar_visible(target);
            }

            WindowMsg::ToggleFullscreen => {
                if self.fullscreen_lyric.is_some() {
                    self.close_fullscreen_lyric(&sender);
                } else if self.current_song.is_none() {
                    self.toast_overlay
                        .add_toast(adw::Toast::new("没有正在播放的歌曲"));
                } else {
                    self.open_fullscreen_lyric(&sender);
                }
            }

            WindowMsg::FullscreenFadedOut => {
                self.finish_fullscreen_cleanup();
            }

            WindowMsg::FullscreenLyricEvent(output) => {
                match output {
                    FullscreenLyricOutput::Close => {
                        self.close_fullscreen_lyric(&sender);
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
            AppRoute::Search => {
                self.content_stack.set_visible_child_name("search");
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
                            WindowMsg::PlayerCommandReceived(PlayerCommand::Play {
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
                        } => WindowMsg::PlayerCommandReceived(PlayerCommand::Play {
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
            AppRoute::Mv(id) => {
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }

                // 暂停正在播放的音乐（进入 MV 页时）
                if self.current_is_playing {
                    self.should_resume_music = true;
                    if let Err(e) = self.player_cmd_tx.send(PlayerCommand::Pause) {
                        log::error!("Cannot pause music: {}", e);
                    }
                }

                let detail = MvPlayerPage::builder().launch(*id).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        MvPlayerOutput::Navigate(route) => WindowMsg::NavigateTo(route),
                        MvPlayerOutput::ShowToast(text) => WindowMsg::ShowToast(text),
                    },
                );

                self.detail_container.append(detail.widget());
                self.content_stack.set_visible_child_name("detail");
                self.detail_ctrl = Some(DetailCtrl::Mv(detail));
            },
        }

        let can_go_back = !self.history.is_empty();

        self.header.emit(HeaderMsg::UpdateState {
            can_go_back,
            active_tab: self.current_route.clone(),
        });
    }

/// 打开全屏歌词页（淡入动画）
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
        // 底层 UI 保持原样，淡入结束后被不透明背景盖住，退出时可形成交叉淡出
        let fade = Fade::new(fl.widget(), 0.0, FULLSCREEN_FADE_MS);
        self.fullscreen_overlay.append(fl.widget());
        self.fullscreen_overlay.set_visible(true);
        fade.set_visible(true);

        self.fullscreen_fade = Some(fade);
        self.fullscreen_lyric = Some(fl);
    }

    /// 关闭全屏歌词页（淡出动画结束后清理）
    fn close_fullscreen_lyric(&mut self, sender: &ComponentSender<Self>) {
        if self.fullscreen_lyric.is_none() {
            return;
        }
        if let Some(fade) = &self.fullscreen_fade {
            let notify = sender.input_sender().clone();
            fade.set_visible_then(false, Some(std::boxed::Box::new(move || {
                let _ = notify.send(WindowMsg::FullscreenFadedOut);
            })));
        } else {
            self.finish_fullscreen_cleanup();
        }
    }

    /// 淡出结束后移除全屏页
    fn finish_fullscreen_cleanup(&mut self) {
        if self.fullscreen_lyric.is_none() {
            return;
        }
        while let Some(child) = self.fullscreen_overlay.first_child() {
            self.fullscreen_overlay.remove(&child);
        }
        self.fullscreen_overlay.set_visible(false);
        self.fullscreen_lyric = None;
        self.fullscreen_fade = None;
    }

    /// 侧栏显示/隐藏（OverlaySplitView 原生 show_sidebar 动画：侧栏与内容同一平面内滑入/滑出）
    fn set_sidebar_visible(&mut self, visible: bool) {
        self.sidebar_visible = visible;
        self.overlay_split_view.set_show_sidebar(visible);
    }
}
