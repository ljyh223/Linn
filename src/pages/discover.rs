//! 发现音乐页面
//!
//! 展示推荐歌单的页面组件。

use gtk::prelude::*;
use gtk::glib;
use netease_cloud_music_api::MusicApi;
use relm4::{
    factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque, Position},
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::components::AsyncImage;

// 全局共享的列数配置
static GRID_COLUMNS: AtomicU32 = AtomicU32::new(4);

// 全局共享的窗口宽度
static WINDOW_WIDTH: AtomicU32 = AtomicU32::new(800);

/// 发现音乐页面组件
pub struct Discover {
    playlists: FactoryVecDeque<PlaylistItem>,
    state: DiscoverState,
    api: Arc<MusicApi>,
    playlist_data: Vec<PlaylistData>,  // 存储数据以便重建
}

/// 页面状态
#[derive(Debug, Clone)]
pub enum DiscoverState {
    Loading,
    Loaded,
    Error(String),
}

/// 歌单数据
#[derive(Debug, Clone)]
pub struct PlaylistData {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub author: String,
}

#[derive(Debug)]
pub enum DiscoverMsg {
    LoadPlaylists,
    PlaylistsLoaded(Vec<PlaylistData>),
    LoadFailed(String),
    Retry,
    PlaylistItemClicked(u64),
    RecalculateColumns,  // 重新计算列数并刷新布局
}

/// Discover 组件输出
#[derive(Debug)]
pub enum DiscoverOutput {
    PlaylistClicked(u64),
}

// Factory 组件的消息
#[derive(Debug)]
pub enum PlaylistItemMsg {
    // 未来可以添加点击事件
}

// Factory 组件的输出（可选）
#[derive(Debug)]
pub enum PlaylistItemOutput {
    Clicked(u64),
}

/// 歌单项 Factory 组件
#[derive(Debug)]
struct PlaylistItem {
    data: PlaylistData,
}

// 为 PlaylistItem 实现 Position trait
impl Position<relm4::factory::positions::GridPosition, DynamicIndex> for PlaylistItem {
    fn position(&self, index: &DynamicIndex) -> relm4::factory::positions::GridPosition {
        // 从全局共享状态读取列数
        let columns = GRID_COLUMNS.load(Ordering::Relaxed);

        let idx = index.current_index() as i32;
        let row = idx / columns as i32;
        let column = idx % columns as i32;

        relm4::factory::positions::GridPosition {
            column,
            row,
            width: 1,
            height: 1,
        }
    }
}

#[relm4::factory]
impl FactoryComponent for PlaylistItem {
    type Init = PlaylistData;
    type Input = PlaylistItemMsg;
    type Output = PlaylistItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Grid;  // 改用 Grid

    view! {
        gtk::Button {
            set_width_request: 160,
            set_height_request: 200,
            set_halign: gtk::Align::Center,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_margin_top: 8,
                set_margin_bottom: 8,

                #[name = "image"]
                AsyncImage {
                    set_width_request: 160,
                    set_height_request: 160,
                    set_border_radius: 32,
                },

                gtk::Label {
                    set_label: &self.data.name,
                    set_max_width_chars: 20,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_wrap: true,
                    set_lines: 2,
                    set_justify: gtk::Justification::Center,
                },

                gtk::Label {
                    set_label: &self.data.author,
                    set_max_width_chars: 20,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "dim-label",
                },
            }
        }
    }

    fn init_model(
        data: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        PlaylistItem { data }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        // 连接点击事件（root 现在是 Button）
        let id = self.data.id;
        root.connect_clicked(move |_| {
            let _ = sender.output(PlaylistItemOutput::Clicked(id));
        });

        // 异步加载图片
        widgets.image.set_src(&self.data.cover_url);

        widgets
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {
        // 处理消息
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Discover {
    type Init = ();
    type Input = DiscoverMsg;
    type Output = DiscoverOutput;

    view! {
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 24,
                set_margin_bottom: 24,

                gtk::Label {
                    set_label: "发现音乐",
                    set_halign: gtk::Align::Start,
                    add_css_class: "heading",
                },

                // 加载状态提示
                #[name = "status_label"]
                gtk::Label {
                    set_label: "加载中...",
                    set_halign: gtk::Align::Center,
                },

                // 错误重试按钮
                #[name = "retry_button"]
                gtk::Button {
                    set_label: "重试",
                    set_halign: gtk::Align::Center,

                    connect_clicked[sender] => move |_| {
                        sender.input(DiscoverMsg::Retry);
                    }
                },

                // 错误消息显示
                #[name = "error_label"]
                gtk::Label {
                    set_halign: gtk::Align::Center,
                    add_css_class: "error",
                },

                // 歌单列表容器
                #[name = "playlists_window"]
                gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,

                    #[name = "playlists_box"]
                    gtk::Grid {
                        set_hexpand: true,
                        set_column_spacing: 12,
                        set_row_spacing: 12,
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
        let widgets = view_output!();

        // 创建 FactoryVecDeque
        let playlists = FactoryVecDeque::builder()
            .launch(widgets.playlists_box.clone())
            .forward(sender.input_sender(), |output| {
                match output {
                    PlaylistItemOutput::Clicked(id) => DiscoverMsg::PlaylistItemClicked(id),
                }
            });

        let model = Discover {
            playlists,
            state: DiscoverState::Loading,
            api: Arc::new(MusicApi::default()),
            playlist_data: Vec::new(),
        };

        // TODO: 监听窗口大小变化，自动触发 RecalculateColumns
        // 当前需要手动调用 sender.input(DiscoverMsg::RecalculateColumns)
        // 或者在窗口调整后按快捷键刷新布局

        // 发送初始加载消息
        sender.input(DiscoverMsg::LoadPlaylists);

        ComponentParts { model, widgets }
    }

    fn pre_view() {
        // 根据 state 更新 UI 可见性
        let is_loading = matches!(model.state, DiscoverState::Loading);
        let is_error = matches!(model.state, DiscoverState::Error(_));
        let is_loaded = matches!(model.state, DiscoverState::Loaded);

        widgets.status_label.set_visible(is_loading);
        widgets.retry_button.set_visible(is_error);

        if let DiscoverState::Error(msg) = &model.state {
            widgets.error_label.set_label(msg);
            widgets.error_label.set_visible(true);
        } else {
            widgets.error_label.set_visible(false);
        }

        widgets.playlists_window.set_visible(is_loaded);
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            DiscoverMsg::LoadPlaylists | DiscoverMsg::Retry => {
                self.state = DiscoverState::Loading;

                let api = self.api.clone();

                // 使用 glib 的标准异步处理（GTK 应用推荐方式）
                glib::MainContext::default().spawn_local(async move {
                    match api.toplist().await {
                        Ok(toplists) => {
                            let playlists: Vec<PlaylistData> = toplists
                                .into_iter()
                                .map(|tl| PlaylistData {
                                    id: tl.id,
                                    name: tl.name,
                                    cover_url: tl.cover,
                                    author: "网易云音乐".to_string(),
                                })
                                .collect();

                            sender.input(DiscoverMsg::PlaylistsLoaded(playlists));
                        }
                        Err(e) => {
                            eprintln!("加载排行榜失败: {:?}", e);
                            sender.input(DiscoverMsg::LoadFailed("加载失败，请稍后重试".to_string()));
                        }
                    }
                });
            }

            DiscoverMsg::PlaylistsLoaded(playlists) => {
                self.state = DiscoverState::Loaded;

                // 保存数据以便重建
                self.playlist_data = playlists.clone();

                // 使用 FactoryVecDeque 更新列表
                let mut guard = self.playlists.guard();
                guard.clear();
                for playlist in playlists {
                    guard.push_back(playlist);
                }
            }

            DiscoverMsg::RecalculateColumns => {
                // 从全局状态获取窗口宽度
                let width = WINDOW_WIDTH.load(Ordering::Relaxed) as i32;

                let card_width = 160;
                let spacing = 12;
                let min_columns = 2;
                let max_columns = 10;

                let available_width = width - 48;
                let new_columns = (available_width / (card_width + spacing) as i32)
                    .max(min_columns as i32)
                    .min(max_columns as i32) as u32;

                let old_columns = GRID_COLUMNS.load(Ordering::Relaxed);

                // 只有列数真正改变时才重建
                if new_columns != old_columns {
                    eprintln!("列数改变: {} -> {}", old_columns, new_columns);
                    GRID_COLUMNS.store(new_columns, Ordering::Relaxed);

                    // 重建 factory 以应用新的列数
                    let mut guard = self.playlists.guard();
                    guard.clear();

                    for playlist in self.playlist_data.clone() {
                        guard.push_back(playlist);
                    }
                }
            }

            DiscoverMsg::LoadFailed(msg) => {
                self.state = DiscoverState::Error(msg);
            }

            DiscoverMsg::PlaylistItemClicked(id) => {
                eprintln!("歌单被点击: {}", id);
                sender.output(DiscoverOutput::PlaylistClicked(id));
            }
        }
    }
}
