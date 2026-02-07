//! 歌单详情页面
//!
//! 显示歌单的详细信息，包括封面、标题、简介、歌曲列表等。

use gtk::prelude::*;
use gtk::glib;
use netease_cloud_music_api::MusicApi;
use relm4::{
    factory::{DynamicIndex, FactoryComponent, FactoryVecDeque, FactorySender},
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};
use std::sync::Arc;

use crate::components::AsyncImage;

/// 歌曲数据
#[derive(Debug, Clone)]
pub struct SongData {
    pub id: u64,
    pub name: String,
    pub artist_name: String,
    pub album_name: String,
    pub duration: u64, // 毫秒
}

/// 详情数据
#[derive(Debug, Clone)]
pub struct DetailData {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub description: Option<String>,
    pub creator_name: Option<String>,
    pub song_count: u64,
    pub play_count: u64,
}

/// 歌单详情页面
pub struct PlaylistDetail {
    /// 详情数据
    detail_data: Option<DetailData>,
    /// 歌曲列表
    songs: FactoryVecDeque<SongItem>,
    /// API 客户端
    api: Arc<MusicApi>,
    /// 歌单 ID
    playlist_id: u64,
    /// 搜索值
    search_value: String,
    /// 是否正在加载
    loading: bool,
}

/// 页面消息
#[derive(Debug)]
pub enum PlaylistDetailMsg {
    /// 加载歌单详情
    LoadDetail(u64),
    /// 详情数据加载完成
    DetailLoaded(DetailData, Vec<SongData>),
    /// 播放歌曲
    PlaySong(u64),
    /// 搜索歌曲
    SearchChanged(String),
}

/// 歌曲项 Factory 组件
#[derive(Debug)]
struct SongItem {
    data: SongData,
    index: usize,
}

/// Factory 组件的消息
#[derive(Debug)]
pub enum SongItemMsg {
    Clicked(u64),
}

/// Factory 组件的输出
#[derive(Debug)]
pub enum SongItemOutput {
    PlaySong(u64),
}

#[relm4::factory]
impl FactoryComponent for SongItem {
    type Init = SongData;
    type Input = SongItemMsg;
    type Output = SongItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 12,
            set_margin_start: 12,
            set_margin_end: 12,
            set_margin_top: 8,
            set_margin_bottom: 8,

            #[watch]
            add_css_class: if self.index % 2 == 0 { "song-item-even" } else { "song-item-odd" },

            gtk::Label {
                set_label: &self.data.name,
                set_hexpand: true,
                set_width_request: 200,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
            },

            gtk::Label {
                set_label: &self.data.artist_name,
                set_width_request: 150,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "dim-label",
            },

            gtk::Label {
                set_label: &self.data.album_name,
                set_width_request: 150,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "dim-label",
            },

            gtk::Label {
                set_label: &format_duration(self.data.duration),
                set_width_request: 50,
                add_css_class: "dim-label",
            },

            #[name = "play_button"]
            gtk::Button {
                set_label: "播放",
                set_width_request: 80,
            }
        }
    }

    fn init_model(
        data: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        SongItem { data, index: 0 }
    }

    fn init_widgets(
        &mut self,
        index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        self.index = index.current_index();
        let widgets = view_output!();

        // 手动连接按钮点击事件
        let song_id = self.data.id;
        widgets.play_button.connect_clicked(move |_| {
            let _ = sender.output(SongItemOutput::PlaySong(song_id));
        });

        widgets
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

impl SongItem {
    fn index(&self) -> usize {
        self.index
    }
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
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 20,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 24,
                set_margin_bottom: 24,

                // 封面
                #[name = "cover_image"]
                AsyncImage {
                    set_width_request: 200,
                    set_height_request: 200,
                },

                // 信息区域
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_hexpand: true,
                    set_vexpand: false,

                    // 标题
                    #[name = "title_label"]
                    gtk::Label {
                        set_label: "加载中...",
                        set_halign: gtk::Align::Start,
                        add_css_class: "heading",
                    },

                    // 创建者
                    #[name = "creator_label"]
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        add_css_class: "dim-label",
                    },

                    // 统计信息
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,
                        set_halign: gtk::Align::Start,

                        #[name = "song_count_label"]
                        gtk::Label {
                            add_css_class: "dim-label",
                        },

                        #[name = "play_count_label"]
                        gtk::Label {
                            add_css_class: "dim-label",
                        },
                    },

                    // 描述
                    #[name = "description_label"]
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_visible: false,
                        set_wrap: true,
                        set_width_chars: 60,
                    },

                    // 按钮区域
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Start,
                        set_margin_top: 12,

                        gtk::Button {
                            set_label: "播放全部",
                            add_css_class: "suggested-action",

                            connect_clicked[sender] => move |_| {
                                // TODO: 播放全部
                                eprintln!("播放全部");
                            }
                        },

                        gtk::Button {
                            set_label: "收藏",
                        },

                        gtk::Button {
                            set_label: "分享",
                        },
                    },
                }
            },

            // 分隔线
            gtk::Separator {
                set_orientation: gtk::Orientation::Horizontal,
            },

            // 搜索框
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 12,
                set_margin_bottom: 12,

                gtk::Label {
                    set_label: "搜索:",
                },

                #[name = "search_entry"]
                gtk::Entry {
                    set_hexpand: true,
                    set_placeholder_text: Some("输入关键词搜索歌曲"),

                    connect_changed[sender] => move |entry| {
                        sender.input(PlaylistDetailMsg::SearchChanged(entry.text().to_string()));
                    }
                },
            },

            // 分隔线
            gtk::Separator {
                set_orientation: gtk::Orientation::Horizontal,
            },

            // 歌曲列表
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_vscrollbar_policy: gtk::PolicyType::Automatic,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_top: 12,
                    set_margin_bottom: 12,

                    // 列表头部
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_margin_start: 12,
                        set_margin_end: 12,
                        set_margin_bottom: 8,

                        gtk::Label {
                            set_label: "歌曲名",
                            set_width_request: 200,
                            add_css_class: "dim-label",
                        },

                        gtk::Label {
                            set_label: "歌手",
                            set_width_request: 150,
                            add_css_class: "dim-label",
                        },

                        gtk::Label {
                            set_label: "专辑",
                            set_width_request: 150,
                            add_css_class: "dim-label",
                        },

                        gtk::Label {
                            set_label: "时长",
                            set_width_request: 50,
                            add_css_class: "dim-label",
                        },

                        gtk::Label {
                            set_label: "操作",
                            set_width_request: 80,
                            add_css_class: "dim-label",
                        },
                    },

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    // 歌曲列表容器
                    #[name = "songs_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_vexpand: true,
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
        let widgets = view_output!();

        // 创建歌曲列表 Factory
        let songs = FactoryVecDeque::builder()
            .launch(widgets.songs_box.clone())
            .forward(sender.input_sender(), |output| {
                match output {
                    SongItemOutput::PlaySong(id) => PlaylistDetailMsg::PlaySong(id),
                }
            });

        let model = PlaylistDetail {
            detail_data: None,
            songs,
            api: Arc::new(MusicApi::default()),
            playlist_id,
            search_value: String::new(),
            loading: false,
        };

        // 发送初始加载消息
        sender.input(PlaylistDetailMsg::LoadDetail(playlist_id));

        ComponentParts { model, widgets }
    }

    fn pre_view() {
        // 更新详情数据
        if let Some(data) = &model.detail_data {
            // 设置封面
            if !data.cover_url.is_empty() {
                widgets.cover_image.set_src(Some(&data.cover_url));
            }

            // 设置标题
            widgets.title_label.set_label(&data.name);

            // 设置创建者
            if let Some(creator) = &data.creator_name {
                widgets.creator_label.set_label(&format!("创建者: {}", creator));
            }

            // 设置歌曲数量
            widgets.song_count_label.set_label(&format!("{} 首歌曲", data.song_count));

            // 设置播放量
            widgets.play_count_label.set_label(&format!("播放量: {}", format_number(data.play_count)));

            // 设置描述
            if let Some(desc) = &data.description {
                widgets.description_label.set_label(desc);
                widgets.description_label.set_visible(true);
            } else {
                widgets.description_label.set_visible(false);
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlaylistDetailMsg::LoadDetail(id) => {
                self.playlist_id = id;
                self.loading = true;

                // 使用 glib 的异步处理
                glib::MainContext::default().spawn_local(async move {
                    // TODO: 调用真实的 API 获取歌单详情
                    // 目前先使用模拟数据
                    let detail_data = DetailData {
                        id,
                        name: "我喜欢的音乐".to_string(),
                        cover_url: "https://p2.music.126.net/UeTuwE7pvjBpypWLudqukA==/109951165629056368.jpg".to_string(),
                        description: Some("这是我的私人歌单，包含了我喜欢的所有音乐。涵盖了流行、摇滚、民谣等多种风格。".to_string()),
                        creator_name: Some("网易云音乐用户".to_string()),
                        song_count: 100,
                        play_count: 1234567,
                    };

                    // 模拟歌曲数据
                    let songs = vec![
                        SongData {
                            id: 1,
                            name: "晴天".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "叶惠美".to_string(),
                            duration: 269000,
                        },
                        SongData {
                            id: 2,
                            name: "稻香".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "魔杰座".to_string(),
                            duration: 223000,
                        },
                        SongData {
                            id: 3,
                            name: "夜曲".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "十一月的萧邦".to_string(),
                            duration: 239000,
                        },
                        SongData {
                            id: 4,
                            name: "七里香".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "七里香".to_string(),
                            duration: 298000,
                        },
                        SongData {
                            id: 5,
                            name: "青花瓷".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "我很忙".to_string(),
                            duration: 239000,
                        },
                        SongData {
                            id: 6,
                            name: "简单爱".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "范特西".to_string(),
                            duration: 267000,
                        },
                        SongData {
                            id: 7,
                            name: "夜曲".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "十一月的萧邦".to_string(),
                            duration: 239000,
                        },
                        SongData {
                            id: 8,
                            name: "发如雪".to_string(),
                            artist_name: "周杰伦".to_string(),
                            album_name: "十一月的萧邦".to_string(),
                            duration: 299000,
                        },
                    ];

                    sender.input(PlaylistDetailMsg::DetailLoaded(detail_data, songs));
                });
            }

            PlaylistDetailMsg::DetailLoaded(detail, songs) => {
                self.detail_data = Some(detail);
                self.loading = false;

                // 更新歌曲列表
                let mut guard = self.songs.guard();
                guard.clear();
                for song in songs {
                    guard.push_back(song);
                }
            }

            PlaylistDetailMsg::PlaySong(id) => {
                eprintln!("播放歌曲: {}", id);
                // TODO: 实现播放功能
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

/// 格式化时长（毫秒 -> mm:ss）
fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
