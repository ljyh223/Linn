use log::trace;
use relm4::gtk::{FlowBox, prelude::*};
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, factory::FactoryVecDeque, gtk}; // ✅ 引入 FactoryVecDeque

use super::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};
use crate::api::{Playlist, get_recommend_playlist};

pub struct Home {
    // ✅ 核心修改：用工厂替代手动的 Vec<Controller>
    playlist_cards: FactoryVecDeque<PlaylistCard>,
    scrolled_window: gtk::ScrolledWindow,
}

#[derive(Debug)]
pub enum HomeMsg {
    LoadPlaylists,
    ScrollLeft,
    ScrollRight,
    PlaylistClicked(u64),
    // ✅ 新增：接收工厂子组件的事件
    CardAction(PlaylistCardOutput),
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

                // ✅ 把 gtk::Box 换成 gtk::FlowBox
                #[name(cards_box)]
                gtk::FlowBox {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_row_spacing: 16,
                    // set_column_spacing: 16,
                    // ✅ 魔法在这里：设置一个极大的值，强制它永远不换行（完全等同于 Box 的行为）
                    set_max_children_per_line: 9999,
                    set_selection_mode: gtk::SelectionMode::None,
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
            playlist_cards: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |msg| match msg {
                    PlaylistCardOutput::Clicked(id) => HomeMsg::PlaylistClicked(id),
                }),
            scrolled_window: gtk::ScrolledWindow::default(),
        };

        let widgets = view_output!();

        model.scrolled_window = widgets.scrolled_window.clone();
        let factory = FactoryVecDeque::builder()
            .launch(widgets.cards_box.clone())
            .forward(sender.input_sender(), |output| HomeMsg::CardAction(output));

        model.playlist_cards = factory;

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

            // ✅ 处理卡片点击
            HomeMsg::CardAction(action) => {
                if let PlaylistCardOutput::Clicked(id) = action {
                    sender.input(HomeMsg::PlaylistClicked(id));
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let HomeCmdMsg::PlaylistsLoaded(playlists) = message {
            let mut guard = self.playlist_cards.guard();
            guard.clear();

            for playlist in playlists {
                guard.push_back(PlaylistCardInit {
                    id: playlist.id,
                    cover_url: format!("{}?param=600y600", playlist.cover_url),
                    title: playlist.name.clone(),
                });
            }
        }
    }
}
