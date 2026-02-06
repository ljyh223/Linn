//! 发现音乐页面
//!
//! 展示推荐歌单的页面组件。

use gtk::prelude::*;
use netease_cloud_music_api::MusicApi;
use relm4::{gtk, ComponentParts, ComponentSender, SimpleComponent};

/// 发现音乐页面组件
pub struct Discover {
    playlists: Vec<PlaylistData>,
    loading: bool,
    api: MusicApi,
    // 保存 widgets 引用以便在 update 中访问
    playlists_box: gtk::FlowBox,
    status_label: gtk::Label,
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

                #[name = "status_label"]
                gtk::Label {
                    set_label: "加载中...",
                    set_halign: gtk::Align::Center,
                    set_visible: true,
                },

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

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        eprintln!("Discover 页面初始化中...");
        let widgets = view_output!();

        let model = Discover {
            playlists: Vec::new(),
            loading: true,
            api: MusicApi::default(),
            playlists_box: widgets.playlists_box.clone(),
            status_label: widgets.status_label.clone(),
        };

        eprintln!("Discover 页面初始化完成，发送 LoadPlaylists 消息");
        // 发送加载消息
        sender.input(DiscoverMsg::LoadPlaylists);

        eprintln!("LoadPlaylists 消息已发送");
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            DiscoverMsg::LoadPlaylists => {
                eprintln!("开始加载排行榜...");
                let api = self.api.clone();

                // 在后台线程加载排行榜（不需要登录）
                std::thread::spawn(move || {
                    eprintln!("API调用线程已启动");
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(async {
                        eprintln!("正在调用 toplist API...");
                        api.toplist().await
                    });

                    eprintln!("API调用完成，result: {:?}", result.is_ok());

                    match result {
                        Ok(toplists) => {
                            eprintln!("成功加载 {} 个排行榜", toplists.len());

                            let playlists: Vec<PlaylistData> = toplists
                                .into_iter()
                                .map(|tl| PlaylistData {
                                    id: tl.id,
                                    name: tl.name,
                                    cover_url: tl.cover,
                                    author: "网易云音乐".to_string(),
                                })
                                .collect();

                            eprintln!("准备发送 PlaylistsLoaded 消息");
                            sender.input(DiscoverMsg::PlaylistsLoaded(playlists));
                            eprintln!("PlaylistsLoaded 消息已发送");
                        }
                        Err(e) => {
                            eprintln!("加载排行榜失败: {:?}", e);
                            eprintln!("错误详细信息: {:?}", e);
                        }
                    }
                });
            }

            DiscoverMsg::PlaylistsLoaded(playlists) => {
                eprintln!("收到 PlaylistsLoaded 消息，共 {} 个歌单", playlists.len());
                self.playlists = playlists.clone();
                self.loading = false;

                // 显示数据到UI
                self.display_playlists_ui(&playlists);
            }
        }
    }

    fn pre_view() {
        // 在 view 更新前处理 UI 更新
    }
}

impl Discover {
    fn display_playlists_ui(&self, playlists: &[PlaylistData]) {
        // 隐藏加载标签
        self.status_label.set_visible(false);

        // 清空现有的 FlowBox
        while let Some(child) = self.playlists_box.first_child() {
            self.playlists_box.remove(&child);
        }

        eprintln!("开始渲染 {} 个歌单到UI", playlists.len());

        // 为每个歌单创建卡片并添加到 FlowBox
        for (index, playlist) in playlists.iter().enumerate() {
            let button = gtk::Button::new();
            button.set_halign(gtk::Align::Start);

            let box_widget = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(8)
                .margin_start(8)
                .margin_end(8)
                .margin_top(8)
                .margin_bottom(8)
                .build();

            // 封面图片（暂时使用占位图标）
            let image = gtk::Image::builder()
                .width_request(144)
                .height_request(144)
                .icon_name("music-note-symbolic")
                .build();

            // 歌单名称
            let name_label = gtk::Label::builder()
                .label(&playlist.name)
                .max_width_chars(20)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .wrap(true)
                .lines(2)
                .justify(gtk::Justification::Center)
                .build();

            // 作者
            let author_label = gtk::Label::builder()
                .label(&playlist.author)
                .max_width_chars(20)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            author_label.add_css_class("dim-label");

            box_widget.append(&image);
            box_widget.append(&name_label);
            box_widget.append(&author_label);
            button.set_child(Some(&box_widget));

            self.playlists_box.append(&button);

            eprintln!("已渲染第 {} 个歌单: {}", index + 1, playlist.name);
        }

        eprintln!("UI渲染完成，共 {} 个歌单卡片", playlists.len());
    }
}
