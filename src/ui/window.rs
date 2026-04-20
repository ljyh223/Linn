//! Main component of the application.
use std::sync::Arc;

use flume::Sender;
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::{AdwApplicationWindowExt, AdwDialogExt};
use relm4::gtk::glib::{MainContext, clone};
use relm4::gtk::prelude::{BoxExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk::{self, Box, Orientation, Stack, StackTransitionType, glib};
use relm4::{
    ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent, adw,
};

use relm4::Component;

use crate::api::{UserInfo, get_user_info};
use crate::player::PlayerFacade;
use crate::player::messages::{PlayerCommand, PlayerEvent};
use crate::ui::collection::{Collection, CollectionMsg, CollectionOutput};
use crate::ui::explore::{Explore, ExploreOutput};
use crate::ui::header::{Header, HeaderMsg, HeaderOutput};
use crate::ui::home::{Home, HomeOutput};
use crate::ui::player::PlayerPageOutput;
use crate::ui::setting::{Settings, SettingsOutput};
use crate::ui::sidebar::{self, Sidebar, SidebarMsg, SidebarOutput}; // 假设你有独立的 Sidebar 组件

use crate::ui::playlist_detail::{PlaylistDetail, PlaylistDetailOutput};
use crate::ui::route::AppRoute;

relm4::new_action_group!(pub WindowActionGroup, "win");
relm4::new_stateless_action!(pub CloseAction, WindowActionGroup, "close");

#[derive(Debug)]
pub enum WindowMsg {
    NavigateTo(AppRoute),
    GoBack,
    ToggleSidebar,

    OpenSettings,
    PlayerEventReceived(PlayerEvent),
    SendCommandToPlayer(PlayerCommand),
    SettingEventReceived(SettingsOutput),

    LoadUserInfo,
    UserInfoLoaded(UserInfo),
}

pub struct Window {
    main_window: adw::ApplicationWindow,
    overlay_split_view: adw::OverlaySplitView,
    settings_dialog: Controller<Settings>,
    pub sidebar: Controller<Sidebar>, // 新增：独立的侧边栏
    pub header: Controller<Header>,   // 纯粹的顶部 Header
    home_ctrl: Controller<Home>,
    explore_ctrl: Controller<Explore>,       // 新增
    collection_ctrl: Controller<Collection>, // 新增

    // 动态页面控制器
    detail_ctrl: Option<Controller<PlaylistDetail>>,

    // 路由历史
    history: Vec<AppRoute>,
    current_route: AppRoute,

    // UI 句柄
    content_stack: Stack,
    detail_container: Box,

    player_cmd_tx: Sender<PlayerCommand>,
    user_info: Option<Arc<UserInfo>>,
}

#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = String;
    type Input = WindowMsg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_default_height: 700,
            set_default_width: 850,

            // 【核心修复：最外层使用 OverlaySplitView 实现左右分栏】
            #[wrap(Some)]
            #[name(overlay_split_view)]
            set_content = &adw::OverlaySplitView {
                // 设置左侧侧边栏宽度比例和极限值
                set_sidebar_width_fraction: 0.30,
                set_min_sidebar_width: 350.0,
                set_max_sidebar_width: 400.0,

                // 1. 左侧：放置侧边栏组件
                set_sidebar: Some(model.sidebar.widget()),

                // 2. 右侧主体区域
                #[wrap(Some)]
                set_content = &adw::ToolbarView {

                    // 右侧上方：常驻的 Header (那3个切换按钮)
                    add_top_bar: model.header.widget(),

                    // 右侧下方：路由切换器 Stack
                    #[name(content_stack)]
                    #[wrap(Some)]
                    set_content = &Stack {
                        set_transition_type: StackTransitionType::Crossfade, // 优雅的淡入淡出

                        // 首页常驻在 Stack 里
                        add_named[Some("home")] = model.home_ctrl.widget() {},
                        add_named[Some("explore")] = model.explore_ctrl.widget() {},
                        add_named[Some("collection")] = model.collection_ctrl.widget() {},

                        // 动态页面的占位容器
                        #[name(detail_container)]
                        add_named[Some("detail")] = &Box {
                            set_orientation: Orientation::Vertical,
                        }
                    }
                }
            }
        }
    }

    fn init(
        cookie: Self::Init,
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
        let default_user = user_arc.clone().unwrap_or_else(|| Arc::new(UserInfo {
            id: 0,
            name: "未登录".to_string(),
            avatar_url: "".to_string(),
        }));
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
                            PlayerPageOutput::Play(index) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::Play(index))
                            }
                        }
                    } // 如果以后 Sidebar 自己有页面切换要告诉 Window，可以在这里处理
                      // SidebarOutput::SwitchPage(_) => WindowMsg::NavigateTo(AppRoute::Home), // 占位
                }
            });


        
        let header = Header::builder()
            .launch(default_user.clone())
            .forward(sender.input_sender(), |msg| match msg {
                HeaderOutput::GoBack => WindowMsg::GoBack,
                HeaderOutput::NavigateTo(route) => WindowMsg::NavigateTo(route),
                HeaderOutput::ToggleSidebar => WindowMsg::ToggleSidebar,
                HeaderOutput::OpenSettings => WindowMsg::OpenSettings,
            });

        let settings_dialog =
            Settings::builder()
                .launch(())
                .forward(sender.input_sender(), |output| {
                    WindowMsg::SettingEventReceived(output)
                    // SettingsOutput::ThemeChanged(i) => WindowMsg::SettingEventReceived(SettingsOutput::ThemeChanged(i)),
                    // SettingsOutput::DynamicBackgroundChanged(b) => WindowMsg::SettingEventReceived(SettingsOutput::DynamicBackgroundChanged(b)),
                });

        let home_ctrl =
            Home::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    HomeOutput::OpenPlaylistDetail(id) => {
                        WindowMsg::NavigateTo(AppRoute::PlaylistDetail(id))
                    }
                });

        let explore_ctrl = Explore::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ExploreOutput::OpenPlaylistDetail(id) => {
                    WindowMsg::NavigateTo(AppRoute::PlaylistDetail(id))
                }
            });
        let collection_ctrl =
            Collection::builder()
                .launch(default_user.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    CollectionOutput::OpenPlaylistDetail(id) => {
                        WindowMsg::NavigateTo(AppRoute::PlaylistDetail(id))
                    }
                });

        // 把 Window 的 sender 转成 PlayerEvent
        let player_event_sender = sender.input_sender().clone().into();
        let player_cmd_tx = PlayerFacade::start(player_event_sender);

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
            settings_dialog: settings_dialog,
            user_info: None,
        };

        let widgets = view_output!();
        model.content_stack = widgets.content_stack.clone();
        model.detail_container = widgets.detail_container.clone();
        model.overlay_split_view = widgets.overlay_split_view.clone();

        if cookie.is_empty() {
            model.settings_dialog
                    .widget()
                    .present(Some(&root));
            
            model.user_info = user_arc;
            eprintln!("No cookie found. Please open settings to set your cookie.");
        }else{

            

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
                self.sidebar.emit(SidebarMsg::PlayerEvent(player_event));
            }
            WindowMsg::SendCommandToPlayer(player_command) => {
                if let Err(e) = self.player_cmd_tx.send(player_command) {
                    log::error!("Cannot send command to player: {}", e);
                }
            }
            WindowMsg::ToggleSidebar => {
                let is_shown = self.overlay_split_view.shows_sidebar();
                self.overlay_split_view.set_show_sidebar(!is_shown);
            }
            WindowMsg::OpenSettings => {
                self.settings_dialog
                    .widget()
                    .present(Some(&self.main_window));
            }
            WindowMsg::SettingEventReceived(settings_output) => {}
            WindowMsg::LoadUserInfo => {
                let sender_clone = sender.clone();
                gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(user_info) = get_user_info().await {
                        sender_clone.input(WindowMsg::UserInfoLoaded(user_info));
                    }
                });
            },
            WindowMsg::UserInfoLoaded(user_info) => {
                let new_arc = Arc::new(user_info);
                self.user_info = Some(new_arc.clone());
                self.user_info.as_ref().unwrap().save_to_disk();
                self.header.emit(HeaderMsg::UpdateUserInfo(new_arc.clone()));
                self.collection_ctrl.emit(CollectionMsg::UpdateUserInfo(new_arc.clone()));
            },
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
            AppRoute::PlaylistDetail(id) => {
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }

                let detail =
                    PlaylistDetail::builder()
                        .launch(*id)
                        .forward(sender.input_sender(), |msg| match msg {
                            PlaylistDetailOutput::PlayQueue(songs, full_ids, index) => {
                                WindowMsg::SendCommandToPlayer(PlayerCommand::PlayQueue {
                                    songs,
                                    full_ids,
                                    start_index: index,
                                })
                            }
                        });

                self.detail_container.append(detail.widget());
                self.content_stack.set_visible_child_name("detail");
                self.detail_ctrl = Some(detail);
            }
        }

        let can_go_back = !self.history.is_empty();

        self.header.emit(HeaderMsg::UpdateState {
            can_go_back,
            active_tab: self.current_route.clone(),
        });
    }
}
