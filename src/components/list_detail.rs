//! 通用列表详情组件
//!
//! 用于显示歌单、专辑、艺术家等的详情信息，支持可配置的显示选项。

use gtk::prelude::*;
use relm4::{gtk, ComponentParts, ComponentSender, SimpleComponent};
use std::sync::Arc;

use crate::components::AsyncImage;

/// 详情数据类型（通用）
#[derive(Debug, Clone)]
pub struct DetailData {
    /// ID
    pub id: u64,
    /// 名称
    pub name: String,
    /// 封面 URL
    pub cover_url: String,
    /// 简介/描述
    pub description: Option<String>,
    /// 播放量
    pub play_count: Option<u64>,
    /// 创建者名称
    pub creator_name: Option<String>,
    /// 艺术家名称（专辑用）
    pub artist_name: Option<String>,
    /// 歌曲数量
    pub song_count: Option<u64>,
    /// 更新时间戳
    pub update_time: Option<i64>,
    /// 创建时间戳
    pub create_time: Option<i64>,
    /// 标签
    pub tags: Option<Vec<String>>,
    /// 隐私状态（10 表示隐私歌单）
    pub privacy: Option<u32>,
}

impl Default for DetailData {
    fn default() -> Self {
        Self {
            id: 0,
            name: "未知".to_string(),
            cover_url: String::new(),
            description: None,
            play_count: None,
            creator_name: None,
            artist_name: None,
            song_count: None,
            update_time: None,
            create_time: None,
            tags: None,
            privacy: None,
        }
    }
}

/// 详情类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailType {
    /// 歌单
    Playlist,
    /// 专辑
    Album,
    /// 艺术家
    Artist,
}

/// 详情组件配置
#[derive(Debug, Clone)]
pub struct DetailConfig {
    /// 标题类型（省略号或正常）
    pub title_ellipsis: bool,
    /// 显示封面遮罩
    pub show_cover_mask: bool,
    /// 显示播放量
    pub show_play_count: bool,
    /// 显示创建者
    pub show_creator: bool,
    /// 显示艺术家
    pub show_artist: bool,
    /// 显示歌曲数量
    pub show_count: bool,
    /// 显示搜索框
    pub show_search: bool,
    /// 显示标签页（歌曲/评论）
    pub show_tabs: bool,
}

impl Default for DetailConfig {
    fn default() -> Self {
        Self {
            title_ellipsis: false,
            show_cover_mask: true,
            show_play_count: true,
            show_creator: true,
            show_artist: false,
            show_count: true,
            show_search: true,
            show_tabs: true,
        }
    }
}

impl DetailConfig {
    /// 创建歌单配置
    pub fn playlist() -> Self {
        Self {
            show_creator: true,
            show_artist: false,
            ..Default::default()
        }
    }

    /// 创建专辑配置
    pub fn album() -> Self {
        Self {
            show_creator: false,
            show_artist: true,
            show_play_count: false,
            ..Default::default()
        }
    }

    /// 创建艺术家配置
    pub fn artist() -> Self {
        Self {
            show_creator: false,
            show_artist: false,
            show_play_count: false,
            show_tabs: false,
            ..Default::default()
        }
    }
}

/// 标签页类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    /// 歌曲
    Songs,
    /// 评论
    Comments,
}

/// 组件消息
#[derive(Debug)]
pub enum ListDetailMsg {
    /// 更新详情数据
    UpdateData(Arc<DetailData>),
    /// 更新配置
    UpdateConfig(DetailConfig),
    /// 搜索值改变
    SearchChanged(String),
    /// 播放全部按钮点击
    PlayAllClicked,
    /// 标签页切换
    TabChanged(DetailTab),
    /// 标签点击
    TagClicked(String),
    /// 描述点击
    DescriptionClicked,
    /// 艺术家点击
    ArtistClicked(String),
    /// 滚动状态改变（用于收起详情）
    ScrollingChanged(bool),
}

/// 组件输出
#[derive(Debug)]
pub enum ListDetailOutput {
    /// 播放全部
    PlayAll,
    /// 标签页切换
    TabChanged(DetailTab),
    /// 搜索值改变
    SearchChanged(String),
    /// 标签点击
    TagClicked(String),
    /// 描述点击
    DescriptionClicked(String),
    /// 艺术家点击
    ArtistClicked(u64),
}

/// 列表详情组件
pub struct ListDetail {
    /// 详情数据
    data: Option<Arc<DetailData>>,
    /// 配置
    config: DetailConfig,
    /// 详情类型
    detail_type: DetailType,
    /// 当前标签页
    current_tab: DetailTab,
    /// 搜索值
    search_value: String,
    /// 是否正在滚动（用于收起详情）
    is_scrolling: bool,
    /// 加载状态
    loading: bool,
}

impl ListDetail {
    /// 创建新的列表详情组件
    pub fn new(detail_type: DetailType) -> Self {
        Self {
            data: None,
            config: match detail_type {
                DetailType::Playlist => DetailConfig::playlist(),
                DetailType::Album => DetailConfig::album(),
                DetailType::Artist => DetailConfig::artist(),
            },
            detail_type,
            current_tab: DetailTab::Songs,
            search_value: String::new(),
            is_scrolling: false,
            loading: false,
        }
    }

    /// 创建歌单详情组件
    pub fn playlist() -> Self {
        Self::new(DetailType::Playlist)
    }

    /// 创建专辑详情组件
    pub fn album() -> Self {
        Self::new(DetailType::Album)
    }

    /// 创建艺术家详情组件
    pub fn artist() -> Self {
        Self::new(DetailType::Artist)
    }

    /// 格式化数字（播放量等）
    fn format_number(&self, num: u64) -> String {
        if num >= 100_000_000 {
            format!("{:.1}亿", num as f64 / 100_000_000.0)
        } else if num >= 10_000 {
            format!("{:.1}万", num as f64 / 10_000.0)
        } else {
            num.to_string()
        }
    }

    /// 格式化时间戳
    fn format_timestamp(&self, timestamp: i64) -> String {
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

    /// 更新可见性（根据配置和数据）
    fn update_visibility(&self, widgets: &ListDetailWidgets) {
        let has_data = self.data.is_some();

        // 根据配置显示/隐藏元素
        widgets.cover_mask.set_visible(
            self.config.show_cover_mask && has_data
        );
        widgets.play_count.set_visible(
            self.config.show_play_count &&
            has_data &&
            self.data.as_ref().and_then(|d| d.play_count).is_some()
        );

        // 显示/隐藏搜索框
        widgets.search_box.set_visible(self.config.show_search && has_data);

        // 显示/隐藏标签页
        widgets.tabs.set_visible(self.config.show_tabs && has_data);

        // 根据滚动状态调整容器
        if self.is_scrolling {
            widgets.main_container.add_css_class("small");
        } else {
            widgets.main_container.remove_css_class("small");
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for ListDetail {
    type Init = DetailType;
    type Input = ListDetailMsg;
    type Output = ListDetailOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_css_classes: &["list-detail"],

            #[name = "main_container"]
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
                    #[name = "cover_mask"]
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

                    // 阴影效果（使用另一个图片实例）
                    gtk::Box {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        add_css_class: "cover-shadow",
                        set_width_request: 240,
                        set_height_request: 240,
                    }
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
                    #[name = "description_container"]
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
                        #[name = "metadata_box"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,
                            set_margin_top: 8,

                            // 创建者/艺术家
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
                            #[name = "play_button"]
                            gtk::Button {
                                set_label: "播放全部",
                                add_css_class: "play-button",

                                connect_clicked[sender] => move |_| {
                                    sender.input(ListDetailMsg::PlayAllClicked);
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
                            #[name = "search_box"]
                            gtk::Box {
                                #[name = "search_entry"]
                                gtk::Entry {
                                    set_placeholder_text: Some("模糊搜索"),
                                    set_width_request: 130,

                                    connect_changed[sender] => move |entry| {
                                        sender.input(ListDetailMsg::SearchChanged(entry.text().to_string()));
                                    }
                                }
                            },

                            // 标签页切换
                            #[name = "tabs"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 0,
                                add_css_class: "tabs",

                                gtk::ToggleButton {
                                    set_label: "歌曲",
                                    set_active: true,
                                    add_css_class: "tab-button",

                                    connect_clicked[sender] => move |_| {
                                        sender.input(ListDetailMsg::TabChanged(DetailTab::Songs));
                                    }
                                },

                                gtk::ToggleButton {
                                    set_label: "评论",
                                    add_css_class: "tab-button",

                                    connect_clicked[sender] => move |_| {
                                        sender.input(ListDetailMsg::TabChanged(DetailTab::Comments));
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        detail_type: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self::new(detail_type);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn pre_view() {
        // 更新可见性
        model.update_visibility(&widgets);

        // 更新数据
        if let Some(data) = &model.data {
            // 设置封面
            if !data.cover_url.is_empty() {
                widgets.cover_image.set_src(&data.cover_url);
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
                widgets.play_count.set_label(&format!("▶ {}", model.format_number(count)));
            }

            // 设置创建者/艺术家
            if model.config.show_creator {
                if let Some(creator) = &data.creator_name {
                    widgets.creator_label.set_label(&format!("👤 {}", creator));
                    widgets.creator_label.set_visible(true);
                }
            } else if model.config.show_artist {
                if let Some(artist) = &data.artist_name {
                    widgets.creator_label.set_label(&format!("🎤 {}", artist));
                    widgets.creator_label.set_visible(true);
                }
            }

            // 设置歌曲数量
            if model.config.show_count {
                if let Some(count) = data.song_count {
                    widgets.count_label.set_label(&format!("🎵 {} 首歌曲", count));
                    widgets.count_label.set_visible(true);
                }
            }

            // 设置时间
            if let Some(update_time) = data.update_time {
                widgets.time_label.set_label(&format!("🕒 {}", model.format_timestamp(update_time)));
                widgets.time_label.set_visible(true);
            } else if let Some(create_time) = data.create_time {
                widgets.time_label.set_label(&format!("🕒 {}", model.format_timestamp(create_time)));
                widgets.time_label.set_visible(true);
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

                    let tag_clone = tag.clone();
                    let sender = sender.clone();
                    tag_btn.connect_clicked(move |_| {
                        sender.input(ListDetailMsg::TagClicked(tag_clone.clone()));
                    });

                    widgets.tags_box.append(&tag_btn);
                }
                widgets.tags_box.set_visible(true);
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ListDetailMsg::UpdateData(data) => {
                self.data = Some(data);
                self.loading = false;
            }

            ListDetailMsg::UpdateConfig(config) => {
                self.config = config;
            }

            ListDetailMsg::SearchChanged(value) => {
                self.search_value = value.clone();
                sender.output(ListDetailOutput::SearchChanged(value));
            }

            ListDetailMsg::PlayAllClicked => {
                sender.output(ListDetailOutput::PlayAll);
            }

            ListDetailMsg::TabChanged(tab) => {
                self.current_tab = tab;
                sender.output(ListDetailOutput::TabChanged(tab));
            }

            ListDetailMsg::TagClicked(tag) => {
                sender.output(ListDetailOutput::TagClicked(tag));
            }

            ListDetailMsg::DescriptionClicked => {
                if let Some(data) = &self.data {
                    if let Some(desc) = &data.description {
                        sender.output(ListDetailOutput::DescriptionClicked(desc.clone()));
                    }
                }
            }

            ListDetailMsg::ArtistClicked(_name) => {
                if let Some(data) = &self.data {
                    sender.output(ListDetailOutput::ArtistClicked(data.id));
                }
            }

            ListDetailMsg::ScrollingChanged(scrolling) => {
                self.is_scrolling = scrolling;
            }
        }
    }
}
