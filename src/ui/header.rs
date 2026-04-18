//! Header component — 纯粹的顶部导航栏

use relm4::adw::ButtonContent;
use relm4::gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, gtk, Component}; // 注意引入 Component
use crate::ui::route::{self, AppRoute};

pub struct Header {
    // 根据当前路由状态，控制“返回”按钮是否可点击
    can_go_back: bool,
    current_tab: AppRoute,
}

#[derive(Debug)]
pub enum HeaderMsg {
    GoBackClicked,
    TabClicked(AppRoute),
    // 供外部 (Window) 调用，更新 Header 的状态（比如点进详情页后，使返回按钮可用）
    UpdateState { can_go_back: bool, active_tab: AppRoute }, 
}

// 向上层抛出的路由事件
#[derive(Debug)]
pub enum HeaderOutput {
    GoBack,
    NavigateTo(AppRoute),
}
#[relm4::component(pub)]
impl Component for Header {
    type Init = ();
    type Input = HeaderMsg;
    type Output = HeaderOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 16,
            set_margin_top: 8,
            set_margin_bottom: 8,
            set_margin_start: 16,
            set_margin_end: 16,
            
            // ===================================
            // 1. 左侧：返回按钮
            // ===================================
            gtk::Button {
                set_icon_name: "go-previous-symbolic",
                add_css_class: "circular",
                add_css_class: "flat",
                // 如果不能后退，就把它置灰禁用
                #[watch]
                set_sensitive: model.can_go_back,
                connect_clicked => HeaderMsg::GoBackClicked,
                
            },

            // 占位，把中间的按钮推到中间
            gtk::Box { set_hexpand: true },

            // ===================================
            // 2. 中间：导航按钮组 (使用 ToggleButton 达到选中效果)
            // ===================================
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                add_css_class: "linked", // 让按钮连在一起，更像选项卡

                gtk::ToggleButton {
                    set_label: "Home",
                    #[watch]
                    set_active: model.current_tab == AppRoute::Home,
                    connect_clicked => HeaderMsg::TabClicked(AppRoute::Home),
   
                },
                gtk::ToggleButton {
                    set_label: "Explore",
                    #[watch]
                    set_active: model.current_tab == AppRoute::Explore,
                    connect_clicked => HeaderMsg::TabClicked(AppRoute::Explore),
           
                },
                gtk::ToggleButton {
                    set_label: "Collection",
                    #[watch]
                    set_active: model.current_tab == AppRoute::Collection,
                    connect_clicked => HeaderMsg::TabClicked(AppRoute::Collection),
                },
            },

            // 占位，保持按钮绝对居中
            gtk::Box { set_hexpand: true },
            
            // （这里未来可以放搜索框或用户头像）
            gtk::Box { set_size_request: (32, -1) } // 补齐右侧宽度，对称
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            can_go_back: false,
            current_tab: AppRoute::Home,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            HeaderMsg::GoBackClicked => {
                sender.output(HeaderOutput::GoBack).unwrap();
            }
            HeaderMsg::TabClicked(tab) => {
                // 防止重复点击发送多次请求
                // if self.current_tab == tab { return; }
                self.current_tab = tab.clone();
                sender.output(HeaderOutput::NavigateTo(tab)).unwrap();
            }
            HeaderMsg::UpdateState { can_go_back, active_tab } => {
                // 接收到 Window 的通知，更新 UI 状态
                self.can_go_back = can_go_back;
                self.current_tab = active_tab;
            }
        }
    }
}