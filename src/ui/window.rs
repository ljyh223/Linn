//! Main component of the application.

use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::AdwApplicationWindowExt;
use relm4::gtk::{Box, Orientation, Stack, StackTransitionType, glib};
use relm4::gtk::prelude::{BoxExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{
    ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent, adw
};

use relm4::Component;

use crate::ui::sidebar::Sidebar; // 假设你有独立的 Sidebar 组件
use crate::ui::header::Header;
use crate::ui::home::{Home, HomeOutput};
use crate::ui::playlist_detail::PlaylistDetail;
use crate::ui::route::AppRoute;

relm4::new_action_group!(pub WindowActionGroup, "win");
relm4::new_stateless_action!(pub CloseAction, WindowActionGroup, "close");

#[derive(Debug)]
pub enum WindowMsg {
    NavigateTo(AppRoute),
    GoBack,
}

pub struct Window {
    // UI 控制器全家桶
    pub sidebar: Controller<Sidebar>, // 新增：独立的侧边栏
    pub header: Controller<Header>,   // 纯粹的顶部 Header
    home_ctrl: Controller<Home>,
    
    // 动态页面控制器
    detail_ctrl: Option<Controller<PlaylistDetail>>,
    
    // 路由历史
    history: Vec<AppRoute>,
    current_route: AppRoute,
    
    // UI 句柄
    content_stack: Stack,
    detail_container: Box,
}

#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = ();
    type Input = WindowMsg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_default_height: 700,
            set_default_width: 850,

            // 【核心修复：最外层使用 OverlaySplitView 实现左右分栏】
            #[wrap(Some)]
            set_content = &adw::OverlaySplitView {
                // 设置左侧侧边栏宽度比例和极限值
                set_sidebar_width_fraction: 0.35,
                set_min_sidebar_width: 200.0,
                set_max_sidebar_width: 300.0,

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
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let app = relm4::main_adw_application();
        app.set_accelerators_for_action::<CloseAction>(&["<Ctrl>W"]);

        let mut action_group = RelmActionGroup::<WindowActionGroup>::new();
        let close_action = RelmAction::<CloseAction>::new_stateless(glib::clone!(
            #[weak] root, move |_| root.close()
        ));
        action_group.add_action(close_action);
        action_group.register_for_widget(&root);

        // 初始化所有静态组件
        let sidebar = Sidebar::builder().launch(()).detach();
        let header = Header::builder().launch(()).detach();

        let home_ctrl = Home::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                HomeOutput::OpenPlaylistDetail(id) => WindowMsg::NavigateTo(AppRoute::PlaylistDetail(id)),
            });

        let mut model = Self {
            sidebar,
            header,
            home_ctrl,
            detail_ctrl: None,
            history: Vec::new(),
            current_route: AppRoute::Home,
            content_stack: Stack::default(),
            detail_container: Box::default(),
        };

        let widgets = view_output!();
        model.content_stack = widgets.content_stack.clone();
        model.detail_container = widgets.detail_container.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            WindowMsg::NavigateTo(route) => {
                if self.current_route == route { return; }
                self.history.push(self.current_route.clone());
                self.current_route = route;
                self.render_current_route(&sender);
            }
            WindowMsg::GoBack => {
                if let Some(prev_route) = self.history.pop() {
                    self.current_route = prev_route;
                    self.render_current_route(&sender);
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
            AppRoute::PlaylistDetail(id) => {
                while let Some(child) = self.detail_container.first_child() {
                    self.detail_container.remove(&child);
                }

                let detail = PlaylistDetail::builder()
                    .launch(*id)
                    // .forward(sender.input_sender(), |msg| match msg { ... }) // 如果有返回按钮
                    .detach(); 

                self.detail_container.append(detail.widget());
                self.content_stack.set_visible_child_name("detail");
                self.detail_ctrl = Some(detail); 
            }
        }
    }
}