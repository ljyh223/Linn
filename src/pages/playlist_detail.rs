//! 歌单详情页面
//!
//! 显示歌单的详细信息，包括封面、标题、简介、歌曲列表等。

use gtk::prelude::*;
use gtk::glib;
use netease_cloud_music_api::MusicApi;
use relm4::{
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};
use std::sync::Arc;

use crate::components::{AsyncImage};
use crate::components::list_detail::{DetailData, DetailTab};

/// 歌单详情页面
pub struct PlaylistDetail {
    /// 详情数据
    detail_data: Option<Arc<DetailData>>,
    /// API 客户端
    api: Arc<MusicApi>,
    /// 歌单 ID
    playlist_id: u64,
    /// 当前标签页
    current_tab: DetailTab,
    /// 搜索值
    search_value: String,
}

/// 页面消息
#[derive(Debug)]
pub enum PlaylistDetailMsg {
    /// 加载歌单详情
    LoadDetail(u64),
    /// 详情数据加载完成
    DetailLoaded(DetailData),
    /// 播放全部
    PlayAll,
    /// 标签页切换
    TabChanged(DetailTab),
    /// 搜索歌曲
    SearchChanged(String),
}

/// 歌单详情页面组件
#[relm4::component(pub)]
impl SimpleComponent for PlaylistDetail {
    type Init = u64;
    type Input = PlaylistDetailMsg;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,

            // 歌单详情头部
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: false,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 20,
                    set_margin_start: 24,
                    set_margin_end: 24,
                    set_margin_top: 24,
                    set_margin_bottom: 24,

                    // 封面区域
                    gtk::Overlay {
                        set_width_request: 240,
                        set_height_request: 240,

                        #[name = "cover_image"]
                        AsyncImage {
                            set_width_request: 240,
                            set_height_request: 240,
                        },

                        // 遮罩层
                        gtk::Box {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Start,
                            set_height_request: 80,
                            add_css_class: "cover-mask",
                        },

                        // 播放量
                        #[name = "play_count"]
                        gtk::Label {
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Start,
                            set_margin_top: 10,
                            set_margin_end: 12,
                            add_css_class: "play-count",
                        },
                    },

                    // 数据区域
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_spacing: 8,

                        // 标题
                        #[name = "title_label"]
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "加载中...",
                            add_css_class: "detail-title",
                        },

                        // 简介容器
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,

                            #[name = "description_label"]
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_visible: false,
                                add_css_class: "detail-description",
                            },

                            // 元数据
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 4,
                                set_margin_top: 8,

                                // 创建者
                                #[name = "creator_label"]
                                gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_visible: false,
                                    add_css_class: "detail-meta",
                                },

                                // 歌曲数量
                                #[name = "count_label"]
                                gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_visible: false,
                                    add_css_class: "detail-meta",
                                },

                                // 时间
                                #[name = "time_label"]
                                gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_visible: false,
                                    add_css_class: "detail-meta",
                                },

                                // 标签
                                #[name = "tags_box"]
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 8,
                                    set_visible: false,
                                }
                            }
                        },

                        // 底部操作栏
                        gtk::Box {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::End,
                            set_vexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            set_margin_top: 12,

                            // 左侧按钮
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 8,

                                // 播放全部按钮
                                gtk::Button {
                                    set_label: "播放全部",
                                    add_css_class: "play-button",

                                    connect_clicked[sender] => move |_| {
                                        sender.input(PlaylistDetailMsg::PlayAll);
                                    }
                                },
                            },

                            // 右侧搜索和标签页
                            gtk::Box {
                                set_halign: gtk::Align::End,
                                set_hexpand: true,
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,

                                // 搜索框
                                gtk::Entry {
                                    set_placeholder_text: Some("模糊搜索"),
                                    set_width_request: 130,

                                    connect_changed[sender] => move |entry| {
                                        sender.input(PlaylistDetailMsg::SearchChanged(entry.text().to_string()));
                                    }
                                },

                                // 标签页切换
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 0,
                                    add_css_class: "tabs",

                                    gtk::ToggleButton {
                                        set_label: "歌曲",
                                        set_active: true,
                                        add_css_class: "tab-button",

                                        connect_clicked[sender] => move |_| {
                                            sender.input(PlaylistDetailMsg::TabChanged(DetailTab::Songs));
                                        }
                                    },

                                    gtk::ToggleButton {
                                        set_label: "评论",
                                        add_css_class: "tab-button",

                                        connect_clicked[sender] => move |_| {
                                            sender.input(PlaylistDetailMsg::TabChanged(DetailTab::Comments));
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            },

            // 分隔线
            gtk::Separator {
                set_orientation: gtk::Orientation::Horizontal,
            },

            // 内容区域（歌曲列表/评论）
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_start: 24,
                    set_margin_end: 24,
                    set_margin_top: 12,
                    set_margin_bottom: 12,

                    #[name = "content_label"]
                    gtk::Label {
                        set_label: "歌曲列表",
                        set_halign: gtk::Align::Start,
                    }
                }
            }
        }
    }

    fn init(
        playlist_id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PlaylistDetail {
            detail_data: None,
            api: Arc::new(MusicApi::default()),
            playlist_id,
            current_tab: DetailTab::Songs,
            search_value: String::new(),
        };

        let widgets = view_output!();

        // 发送初始加载消息
        sender.input(PlaylistDetailMsg::LoadDetail(playlist_id));

        ComponentParts { model, widgets }
    }

    fn pre_view() {
        // 更新数据
        if let Some(data) = &model.detail_data {
            // 设置封面
            if !data.cover_url.is_empty() {
                widgets.cover_image.set_src(Some(&data.cover_url));
            }

            // 设置标题
            widgets.title_label.set_label(&data.name);

            // 设置描述
            if let Some(desc) = &data.description {
                widgets.description_label.set_label(desc);
                widgets.description_label.set_visible(true);
            } else {
                widgets.description_label.set_visible(false);
            }

            // 设置播放量
            if let Some(count) = data.play_count {
                widgets.play_count.set_label(&format!("▶ {}", format_number(count)));
            } else {
                widgets.play_count.set_visible(false);
            }

            // 设置创建者
            if let Some(creator) = &data.creator_name {
                widgets.creator_label.set_label(&format!("👤 {}", creator));
                widgets.creator_label.set_visible(true);
            } else {
                widgets.creator_label.set_visible(false);
            }

            // 设置歌曲数量
            if let Some(count) = data.song_count {
                widgets.count_label.set_label(&format!("🎵 {} 首歌曲", count));
                widgets.count_label.set_visible(true);
            } else {
                widgets.count_label.set_visible(false);
            }

            // 设置时间
            if let Some(update_time) = data.update_time {
                widgets.time_label.set_label(&format!("🕒 {}", format_timestamp(update_time)));
                widgets.time_label.set_visible(true);
            } else if let Some(create_time) = data.create_time {
                widgets.time_label.set_label(&format!("🕒 {}", format_timestamp(create_time)));
                widgets.time_label.set_visible(true);
            } else {
                widgets.time_label.set_visible(false);
            }

            // 设置标签
            if let Some(tags) = &data.tags {
                // 清除旧标签
                while let Some(child) = widgets.tags_box.first_child() {
                    widgets.tags_box.remove(&child);
                }

                // 添加新标签
                for tag in tags {
                    let tag_btn = gtk::Button::new();
                    tag_btn.set_label(tag);
                    tag_btn.add_css_class("tag-button");
                    widgets.tags_box.append(&tag_btn);
                }
                widgets.tags_box.set_visible(true);
            } else {
                widgets.tags_box.set_visible(false);
            }
        }

        // 更新内容区域标签
        let tab_name = match model.current_tab {
            DetailTab::Songs => "歌曲列表",
            DetailTab::Comments => "评论",
        };
        widgets.content_label.set_label(tab_name);
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlaylistDetailMsg::LoadDetail(id) => {
                self.playlist_id = id;

                let api = self.api.clone();

                // 使用 glib 的异步处理
                glib::MainContext::default().spawn_local(async move {
                    // TODO: 调用真实的 API 获取歌单详情
                    // 目前先使用模拟数据
                    let detail_data = DetailData {
                        id,
                        name: "我喜欢的音乐".to_string(),
                        cover_url: "https://p2.music.126.net/UeTuwE7pvjBpypWLudqukA==/109951165629056368.jpg".to_string(),
                        description: Some("这是我的私人歌单，包含了我喜欢的所有音乐。".to_string()),
                        play_count: Some(1234567),
                        creator_name: Some("网易云音乐用户".to_string()),
                        artist_name: None,
                        song_count: Some(100),
                        update_time: Some(1704067200),
                        create_time: Some(1609459200),
                        tags: Some(vec!["流行".to_string(), "华语".to_string(), "轻松".to_string()]),
                        privacy: None,
                    };

                    sender.input(PlaylistDetailMsg::DetailLoaded(detail_data));
                });
            }

            PlaylistDetailMsg::DetailLoaded(data) => {
                self.detail_data = Some(Arc::new(data));
            }

            PlaylistDetailMsg::PlayAll => {
                eprintln!("播放全部: 歌单 ID {}", self.playlist_id);
                // TODO: 实现播放全部功能
            }

            PlaylistDetailMsg::TabChanged(tab) => {
                self.current_tab = tab;
                eprintln!("标签页切换: {:?}", tab);
                // TODO: 更新内容区域显示
            }

            PlaylistDetailMsg::SearchChanged(text) => {
                eprintln!("搜索: {}", text);
                self.search_value = text;
                // TODO: 实现搜索过滤功能
            }
        }
    }
}

/// 格式化数字（播放量等）
fn format_number(num: u64) -> String {
    if num >= 100_000_000 {
        format!("{:.1}亿", num as f64 / 100_000_000.0)
    } else if num >= 10_000 {
        format!("{:.1}万", num as f64 / 10_000.0)
    } else {
        num.to_string()
    }
}

/// 格式化时间戳
fn format_timestamp(timestamp: i64) -> String {
    use gtk::glib::DateTime;
    use gtk::glib::TimeZone;

    if let Ok(dt) = DateTime::from_unix_utc(timestamp) {
        let tz = TimeZone::local();
        if let Ok(local) = dt.to_timezone(&tz) {
            return local.format("%Y-%m-%d").unwrap_or_default().to_string();
        }
    }
    String::new()
}
