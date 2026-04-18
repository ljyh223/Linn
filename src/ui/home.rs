use log::trace;
use relm4::adw::prelude::NavigationPageExt;
use relm4::gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GestureExt, OrientableExt, WidgetExt}; // 注意这里加入了 AdjustmentExt
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, gtk};

use super::components::playlist_card::PlaylistCard;
use crate::api::{Playlist, get_recommend_playlist};

pub struct Home {
    playlists: Vec<Playlist>,
    cards_box: gtk::Box,
    scrolled_window: gtk::ScrolledWindow, // 新增：保存 ScrolledWindow 的引用以便控制滚动
}

#[derive(Debug)]
pub enum HomeMsg {
    LoadPlaylists,
    ScrollLeft,
    ScrollRight,
    PlaylistClicked(u64),
}

#[derive(Debug)]
pub enum HomeCmdMsg {
    PlaylistsLoaded(Vec<Playlist>),
}

#[derive(Debug)]
pub enum HomeOutput {
    OpenPlaylistDetail(u64), // 携带歌单 ID
}

#[relm4::component(pub)]
impl Component for Home {
    type Init = ();
    type Input = HomeMsg;
    type CommandOutput = HomeCmdMsg;
    type Output = HomeOutput;

    view! {
        #[root]
        gtk::Box{
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_margin_top: 16,
            set_margin_bottom: 16,
            set_margin_start: 16,
            set_margin_end: 16,

            // === 修改 1：将标题和按钮放入一个水平 Box 中 ===
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_margin_bottom: 4,

                // 标题（占据剩余空间，把按钮推到最右边）
                gtk::Label {
                    set_label: "推荐歌单",
                    add_css_class: "title-3",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },

                // 向左滚动按钮
                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    add_css_class: "circular", // 圆形按钮样式
                    add_css_class: "flat",     // 扁平化无边框样式
                    set_tooltip_text: Some("向左滚动"),
                    connect_clicked => HomeMsg::ScrollLeft,
                },

                // 向右滚动按钮
                gtk::Button {
                    set_icon_name: "go-next-symbolic",
                    add_css_class: "circular",
                    add_css_class: "flat",
                    set_tooltip_text: Some("向右滚动"),
                    connect_clicked => HomeMsg::ScrollRight,
                }
            },

            // === 修改 2：给 ScrolledWindow 命名，并修改 PolicyType ===
            #[name(scrolled_window)]
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::External,
                set_vscrollbar_policy: gtk::PolicyType::Never,
                set_min_content_height: 220,
                set_max_content_height: 220,

                #[name(cards_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 16,
                },
            },
        }

    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            playlists: Vec::new(),
            cards_box: gtk::Box::default(),
            scrolled_window: gtk::ScrolledWindow::default(),
        };
        let widgets = view_output!();

        model.cards_box = widgets.cards_box.clone();
        model.scrolled_window = widgets.scrolled_window.clone();

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

            HomeMsg::PlaylistClicked(id) => {
                // 将跳转请求抛出给外层的 Window 组件
                if let Err(e) = sender.output(HomeOutput::OpenPlaylistDetail(id)) {
                    log::error!("Failed to send OpenPlaylistDetail output: {:?}", e);
                }
            }

            // === 修改 4：实现按钮点击的滚动逻辑 ===
            HomeMsg::ScrollLeft => {
                let adj = self.scrolled_window.hadjustment();
                let scroll_amount = 250.0; // 每次滚动的像素值，你可以根据卡片宽度调整
                let new_value = (adj.value() - scroll_amount).max(adj.lower());
                adj.set_value(new_value);
            }
            HomeMsg::ScrollRight => {
                let adj = self.scrolled_window.hadjustment();
                let scroll_amount = 250.0;
                // 注意不要超过最大滚动范围 (upper - page_size)
                let max_value = adj.upper() - adj.page_size();
                let new_value = (adj.value() + scroll_amount).min(max_value);
                adj.set_value(new_value);
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let HomeCmdMsg::PlaylistsLoaded(playlists) = message {
            self.playlists = playlists;
            self.update_cards(&sender)
        }
    }
}
impl Home {
    // 接收 sender 引用
    fn update_cards(&self, sender: &ComponentSender<Self>) {
        // 清空现有卡片
        while let Some(child) = self.cards_box.first_child() {
            self.cards_box.remove(&child);
        }

        for playlist in &self.playlists {
            // let card = PlaylistCard::new(playlist);
            // let widget = card.widget();

            // ==========================================
            // 为卡片添加点击事件 (GTK4 手势系统)
            // ==========================================
            let gesture = gtk::GestureClick::new();
            
            // 假设你的 Playlist 结构体里有一个 id 字段
            // 如果你的 api 返回的 id 是数字类型直接用，如果是 String 需要 parse
            let playlist_id = playlist.id as u64; 
            
            let s = sender.clone();
            
            gesture.connect_pressed(move |gesture, n_press, _x, _y| {
                // n_press 是点击次数，1 代表单击，2 代表双击
                if n_press == 1 {
                    // 标记该事件已被处理，防止事件冒泡导致冲突
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    // 给 Home 组件自己发消息
                    s.input(HomeMsg::PlaylistClicked(playlist_id));
                }
            });

            // 将手势控制器挂载到卡片的 widget 上
            widget.add_controller(gesture);

            // 【提升 UX】：鼠标悬停在卡片上时，光标变成可点击的“小手”
            widget.set_cursor_from_name(Some("pointer"));

            self.cards_box.append(widget);
        }
    }
}