//! 发现音乐页面
//!
//! 展示推荐歌单的页面组件。

use gtk::prelude::*;
use gtk::glib;
use netease_cloud_music_api::MusicApi;
use relm4::{
    factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque},
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};
use std::sync::Arc;

use crate::utils::image_cache;

/// 发现音乐页面组件
pub struct Discover {
    playlists: FactoryVecDeque<PlaylistItem>,
    state: DiscoverState,
    api: Arc<MusicApi>,
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

#[relm4::factory]
impl FactoryComponent for PlaylistItem {
    type Init = PlaylistData;
    type Input = PlaylistItemMsg;
    type Output = PlaylistItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Button {
            set_halign: gtk::Align::Start,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_margin_top: 8,
                set_margin_bottom: 8,

                #[name = "image"]
                gtk::Image {
                    set_width_request: 144,
                    set_height_request: 144,
                    set_icon_name: Some("music-note-symbolic"),
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

        // 连接点击事件
        let id = self.data.id;
        root.connect_clicked(move |_| {
            sender.output(PlaylistItemOutput::Clicked(id));
        });

        // 异步加载图片
        let cover_url = self.data.cover_url.clone();
        let image_weak = widgets.image.downgrade();
        glib::MainContext::default().spawn_local(async move {
            if let Some(paintable) = image_cache::load_image_paintable(&cover_url).await {
                if let Some(img) = image_weak.upgrade() {
                    img.set_paintable(Some(&paintable));
                }
            }
        });

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
    type Output = ();

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
                    gtk::FlowBox {
                        set_hexpand: true,
                        set_min_children_per_line: 3,
                        set_max_children_per_line: 6,
                        set_column_spacing: 12,
                        set_row_spacing: 12,
                        set_selection_mode: gtk::SelectionMode::None,
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
        };

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

                // 使用 FactoryVecDeque 更新列表
                let mut guard = self.playlists.guard();
                guard.clear();
                for playlist in playlists {
                    guard.push_back(playlist);
                }
            }

            DiscoverMsg::LoadFailed(msg) => {
                self.state = DiscoverState::Error(msg);
            }

            DiscoverMsg::PlaylistItemClicked(id) => {
                eprintln!("歌单被点击: {}", id);
                // TODO: 未来可以导航到歌单详情页
            }
        }
    }
}
