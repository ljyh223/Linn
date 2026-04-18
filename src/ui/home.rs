use log::trace;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, gtk};

// 引入你的 Card 组件及其初始化的结构和输出枚举
use super::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};
use crate::api::{Playlist, get_recommend_playlist};

pub struct Home {
    // 核心修改 1：不再存储单纯的 Playlist 数据，而是存储带有 UI 和状态的 Controller！
    playlist_cards: Vec<Controller<PlaylistCard>>, 
    
    cards_box: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
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
    OpenPlaylistDetail(u64),
}

#[relm4::component(pub)]
impl Component for Home {
    type Init = ();
    type Input = HomeMsg;
    type CommandOutput = HomeCmdMsg;
    type Output = HomeOutput;

    view! {
        #[root]
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

                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    add_css_class: "circular",
                    add_css_class: "flat",
                    set_tooltip_text: Some("向左滚动"),
                    connect_clicked => HomeMsg::ScrollLeft,
                },

                gtk::Button {
                    set_icon_name: "go-next-symbolic",
                    add_css_class: "circular",
                    add_css_class: "flat",
                    set_tooltip_text: Some("向右滚动"),
                    connect_clicked => HomeMsg::ScrollRight,
                }
            },

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
            playlist_cards: Vec::new(),
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
                if let Err(e) = sender.output(HomeOutput::OpenPlaylistDetail(id)) {
                    log::error!("Failed to send OpenPlaylistDetail output: {:?}", e);
                }
            }

            HomeMsg::ScrollLeft => {
                let adj = self.scrolled_window.hadjustment();
                let scroll_amount = 250.0;
                let new_value = (adj.value() - scroll_amount).max(adj.lower());
                adj.set_value(new_value);
            }
            HomeMsg::ScrollRight => {
                let adj = self.scrolled_window.hadjustment();
                let scroll_amount = 250.0;
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
            // ==========================================
            // 核心修改 2：组件化地渲染列表项
            // ==========================================

            // 1. 清理旧的 UI 子节点
            while let Some(child) = self.cards_box.first_child() {
                self.cards_box.remove(&child);
            }
            // 2. 清理旧的 Controller（这会自动 Drop 组件，回收内存）
            self.playlist_cards.clear();

            // 3. 循环创建新的 Card 组件
            for playlist in playlists {
                // 根据你的 Playlist API 模型字段，传入初始化数据
                // (注意：这里字段名 cover_url / name 等请根据你实际的 API 修改)
                let init_data = PlaylistCardInit {
                    id: playlist.id, 
                    cover_url: format!("{}?param=600y600",playlist.cover_url), 
                    title: playlist.name.clone(), 
                };

                // 创建子组件，并将子组件发出的 Output 转发为当前组件的 Input
                let controller = PlaylistCard::builder()
                    .launch(init_data)
                    .forward(sender.input_sender(), |output| match output {
                        // 接收到卡片发出的 Clicked(id)，转换为 Home 的 PlaylistClicked(id)
                        PlaylistCardOutput::Clicked(id) => HomeMsg::PlaylistClicked(id),
                    });

                // 将子组件的根 widget 添加到视图中
                self.cards_box.append(controller.widget());

                // 必须将 controller 保存起来！如果不存入 Vec，循环结束时组件会被销毁
                self.playlist_cards.push(controller);
            }
        }
    }
}