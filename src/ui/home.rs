use futures::stream::{self, StreamExt};
use log::trace;
use relm4::factory::FactoryVecDeque;
use relm4::gtk::{prelude::*, Adjustment};
use relm4::{gtk, prelude::*, ComponentParts, ComponentSender};

// 引入我们刚刚生成的用于普通 Box 的卡片组件
use super::components::playlist_card::{
    BoxPlaylistCard, PlaylistCard, PlaylistCardInit, PlaylistCardOutput,
};
use crate::api::{get_playlist_detail, get_recommend_playlist, Playlist, PlaylistDetail};
use crate::ui::model::PlaylistType;

const RADAR_PLAYLIST_IDS: &[u64] = &[
    3136952023, 8402996200, 5320167908, 5327906368, 5362359247, 5300458264, 5341776086,
];
const CONCURRENCY_LIMIT: usize = 3;

pub struct Home {
    playlist_cards: FactoryVecDeque<PlaylistCard>,
    radar_cards: FactoryVecDeque<BoxPlaylistCard>, // 改用 BoxPlaylistCard
    radar_adjustment: Adjustment,                  // 新增：用于控制滚动
}

#[derive(Debug)]
pub enum HomeMsg {
    LoadPlaylists,
    LoadRadarPlaylists,
    CardAction(PlaylistCardOutput),
    RadarCardAction(PlaylistCardOutput),
    ScrollLeft,  // 新增
    ScrollRight, // 新增
}

#[derive(Debug)]
pub enum HomeCmdMsg {
    PlaylistsLoaded(Vec<Playlist>),
    RadarPlaylistsLoaded(Vec<PlaylistDetail>),
}

#[derive(Debug)]
pub enum HomeOutput {
    OpenPlaylistDetail(u64),
    Playlist(PlaylistType),
}

#[relm4::component(pub)]
impl Component for Home {
    type Init = ();
    type Input = HomeMsg;
    type CommandOutput = HomeCmdMsg;
    type Output = HomeOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_vexpand: true,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_top: 16,
                set_margin_bottom: 16,
                set_margin_start: 16,
                set_margin_end: 16,

                // ── 雷达歌单（横向滚动列表） ──
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Box{
                        set_orientation: gtk::Orientation::Horizontal,

                        gtk::Label {
                            set_label: "雷达歌单",
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

                    

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

           

                        #[name(radar_scrolled_window)]
                        gtk::ScrolledWindow {
                            set_hscrollbar_policy: gtk::PolicyType::External, // 隐藏原生滚动条可用 CSS: scrollbar { opacity: 0; }
                            set_vscrollbar_policy: gtk::PolicyType::Never,
                            set_min_content_height: 220,
                            set_max_content_height: 220,
                            set_hexpand: true, // 让它占据按钮之外的所有空间

                            // 这里是雷达卡片的实际父容器
                            #[name(radar_hbox)]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 16,
                                set_margin_start: 4,
                                set_margin_end: 4,
                            }
                        },
                    },
                },


                // ── 推荐歌单（FlowBox 网格） ──
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Label {
                        set_label: "推荐歌单",
                        add_css_class: "title-3",
                        set_halign: gtk::Align::Start,
                    },

                    #[name(cards_box)]
                    gtk::FlowBox {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_row_spacing: 16,
                        set_column_spacing: 16,
                        set_min_children_per_line: 2,
                        set_max_children_per_line: 7,
                        set_selection_mode: gtk::SelectionMode::None,
                    },
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            playlist_cards: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), HomeMsg::CardAction),
            radar_cards: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .forward(sender.input_sender(), HomeMsg::RadarCardAction),
            radar_adjustment: Adjustment::default(), // 临时占位
        };

        let widgets = view_output!();

        // 绑定真实的 Widget 到工厂
        model.playlist_cards = FactoryVecDeque::builder()
            .launch(widgets.cards_box.clone())
            .forward(sender.input_sender(), HomeMsg::CardAction);

        model.radar_cards = FactoryVecDeque::builder()
            .launch(widgets.radar_hbox.clone())
            .forward(sender.input_sender(), HomeMsg::RadarCardAction);

        // 提取 ScrolledWindow 的水平调节器保存到 Model 中
        model.radar_adjustment = widgets.radar_scrolled_window.hadjustment();

        sender.input(HomeMsg::LoadRadarPlaylists);
        sender.input(HomeMsg::LoadPlaylists);

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        trace!("Home: {message:?}");
        match message {
            HomeMsg::LoadPlaylists => {
                sender.command(|out, _shutdown| async move {
                    match get_recommend_playlist().await {
                        Ok(playlists) => {
                            let _ = out.send(HomeCmdMsg::PlaylistsLoaded(playlists));
                        }
                        Err(e) => log::error!("加载推荐歌单失败: {e}"),
                    }
                });
            }

            HomeMsg::LoadRadarPlaylists => {
                sender.command(|out, _shutdown| async move {
                    let ids = RADAR_PLAYLIST_IDS.to_vec();
                    let results: Vec<_> = stream::iter(
                        ids.into_iter()
                            .enumerate()
                            .map(|(i, id)| async move {
                                let result = get_playlist_detail(id).await;
                                (i, result)
                            }),
                    )
                    .buffer_unordered(CONCURRENCY_LIMIT)
                    .collect()
                    .await;

                    let mut results = results;
                    results.sort_by_key(|(i, _)| *i);
                    let playlists: Vec<PlaylistDetail> =
                        results.into_iter().filter_map(|(_, r)| r.ok()).collect();

                    let _ = out.send(HomeCmdMsg::RadarPlaylistsLoaded(playlists));
                });
            }

            // 处理滚动逻辑
            HomeMsg::ScrollLeft => {
                let adj = &self.radar_adjustment;
                let scroll_amount = 250.0;
                let new_value = (adj.value() - scroll_amount).max(adj.lower());
                adj.set_value(new_value);
            }

            HomeMsg::ScrollRight => {
                let adj = &self.radar_adjustment;
                let scroll_amount = 250.0;
                let max_value = adj.upper() - adj.page_size();
                let new_value = (adj.value() + scroll_amount).min(max_value);
                adj.set_value(new_value);
            }

            // 处理卡片点击
            HomeMsg::CardAction(action) | HomeMsg::RadarCardAction(action) => match action {
                PlaylistCardOutput::Clicked(id) => {
                    let _ = sender.output(HomeOutput::OpenPlaylistDetail(id));
                }
                PlaylistCardOutput::ClickedPlaylist(playlist_id) => {
                    trace!("点击了歌单play: {playlist_id}");
                    let _ = sender.output(HomeOutput::Playlist(PlaylistType::Playlist(
                        playlist_id,
                    )));
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            HomeCmdMsg::PlaylistsLoaded(playlists) => {
                let mut guard = self.playlist_cards.guard();
                guard.clear();
                for playlist in playlists {
                    guard.push_back(PlaylistCardInit {
                        id: playlist.id,
                        cover_url: format!("{}?param=300y300", playlist.cover_url),
                        title: playlist.name.clone(),
                        show_play_button: true,
                    });
                }
            }

            HomeCmdMsg::RadarPlaylistsLoaded(playlists) => {
                let mut guard = self.radar_cards.guard();
                guard.clear();
                for detail in playlists {
                    // ⚠️ 请根据实际 PlaylistDetail 结构体调整字段名 (如 detail.playlist.xxx)
                    guard.push_back(PlaylistCardInit {
                        id: detail.id,
                        cover_url: format!("{}?param=300y300", detail.cover_url),
                        title: detail.name.clone(),
                        show_play_button: true,
                    });
                }
            }
        }
    }
}
