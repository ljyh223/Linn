use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::prelude::*;

// 使用导入
use crate::ui::components::AsyncImage;
use ncm_api::{MusicApi, SongList};

/// 歌单数据结构
#[derive(Debug, Clone)]
pub struct PlaylistData {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub author: String,
}

impl From<SongList> for PlaylistData {
    fn from(list: SongList) -> Self {
        Self {
            id: list.id,
            name: list.name,
            cover_url: list.cover_img_url,
            author: list.author,
        }
    }
}

// ============== PlaylistCard: 子组件（单张卡片） ==============

pub struct PlaylistCard {
    id: u64,
    name: String,
    author: String,
    // 存储 AsyncImage 控制器，防止被丢弃
    image_controller: Controller<AsyncImage>,
}

#[derive(Debug)]
pub enum PlaylistCardMsg {
    Selected,
}

#[derive(Debug)]
pub enum PlaylistCardOutput {
    Selected(u64),
}

#[relm4::factory(pub)]
impl FactoryComponent for PlaylistCard {
    type Init = PlaylistData;
    type Input = PlaylistCardMsg;
    type Output = PlaylistCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        root = gtk::Button {
            add_css_class: "flat",
            add_css_class: "playlist-card",
            set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_margin_top: 8,
                set_margin_bottom: 8,

                // AsyncImage 容器
                #[name(cover_container)]
                gtk::Box {
                    set_height_request: 140,
                    set_width_request: 140,
                },

                gtk::Label {
                    set_label: &self.name,
                    add_css_class: "playlist-title",
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_lines: 2,
                    set_wrap: true,
                    set_wrap_mode: gtk::pango::WrapMode::WordChar,
                    set_justify: gtk::Justification::Left,
                    set_halign: gtk::Align::Start,
                    set_max_width_chars: 20,
                },

                gtk::Label {
                    set_label: &self.author,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_halign: gtk::Align::Start,
                    set_max_width_chars: 20,
                }
            },

            connect_clicked[_sender] => move |_| {
                _sender.input(PlaylistCardMsg::Selected);
            },
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        // 初始化图片组件
        let image_controller = AsyncImage::builder()
            .launch((
                Some(init.cover_url.clone()),
                None,
                "playlist-cover".to_string(),
            ))
            .detach();

        Self {
            id: init.id,
            name: init.name,
            author: init.author,
            image_controller,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        _root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        // 将 AsyncImage widget 添加到 cover_container
        widgets
            .cover_container
            .append(self.image_controller.widget());

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            PlaylistCardMsg::Selected => {
                let _ = sender.output(PlaylistCardOutput::Selected(self.id));
            }
        }
    }
}

// ============== DiscoverModel: 父组件 ==============

/// 推荐页面组件
pub struct DiscoverModel {
    loading: bool,
    // Factory 管理所有的卡片
    playlists: FactoryVecDeque<PlaylistCard>,
    // 保存container的引用以便手动控制可见性
    container: gtk::Box,
}

#[derive(Debug)]
pub enum DiscoverInput {
    LoadPlaylists,
    UpdatePlaylists(Vec<PlaylistData>),
    PlaylistSelected(u64),
}

#[derive(Debug)]
pub enum DiscoverOutput {
    PlaylistSelected(u64),
}

#[relm4::component(pub)]
impl SimpleComponent for DiscoverModel {
    type Init = ();
    type Input = DiscoverInput;
    type Output = DiscoverOutput;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_vexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 24,
                set_margin_bottom: 24,
                set_hexpand: true, // 必须
                set_vexpand: true, // 必须

                // 标题
                gtk::Label {
                    set_label: "热门歌单",
                    add_css_class: "title-2",
                    set_halign: gtk::Align::Start,
                },

                // 加载中提示
                gtk::Spinner {
                    set_halign: gtk::Align::Center,
                    set_spinning: true,
                },

                // 容器
                #[name(container)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_vexpand: true,
                    set_hexpand: true,
                    set_visible: false,
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 创建工厂
        let playlists = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::default())
            .forward(sender.input_sender(), |output| match output {
                PlaylistCardOutput::Selected(id) => DiscoverInput::PlaylistSelected(id),
            });

        let model = DiscoverModel {
            loading: true,
            playlists,
            container: gtk::Box::default(), // 临时值，后面会替换
        };

        let widgets = view_output!();

        // 替换container为实际的widget
        let container_clone = widgets.container.clone();
        let mut model_mut = model;
        model_mut.container = container_clone;

        // 配置并添加 FlowBox 到容器
        let flowbox = model_mut.playlists.widget();
        flowbox.set_selection_mode(gtk::SelectionMode::None);
        flowbox.set_column_spacing(12);
        flowbox.set_row_spacing(12);
        flowbox.set_min_children_per_line(2);
        flowbox.set_max_children_per_line(6);
        flowbox.set_halign(gtk::Align::Center);
        flowbox.set_valign(gtk::Align::Start);
        flowbox.set_vexpand(true);
        flowbox.set_hexpand(true);
        widgets.container.append(flowbox);

        // 启动时加载推荐歌单
        sender.input(DiscoverInput::LoadPlaylists);

        ComponentParts { model: model_mut, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            DiscoverInput::LoadPlaylists => {
                self.loading = true;
                let sender_clone = sender.clone();
                sender.command(|_out, _shutdown| {
                    async move {
                        let api = MusicApi::default();
                        // 使用热门歌单 API（不需要登录）
                        match api.top_song_list("全部", "hot", 0, 20).await {
                            Ok(playlists) => {
                                let playlist_data: Vec<PlaylistData> =
                                    playlists.into_iter().map(PlaylistData::from).collect();
                                sender_clone.input(DiscoverInput::UpdatePlaylists(playlist_data));
                            }
                            Err(e) => {
                                eprintln!("加载推荐歌单失败: {:?}", e);
                                sender_clone.input(DiscoverInput::UpdatePlaylists(Vec::new()));
                            }
                        }
                    }
                });
            }
            DiscoverInput::UpdatePlaylists(data) => {
                self.loading = false;
                let mut guard = self.playlists.guard();
                guard.clear();
                for item in data {
                    guard.push_back(item);
                }
                drop(guard);

                // 手动设置container可见
                self.container.set_visible(true);
            }
            DiscoverInput::PlaylistSelected(id) => {
                // 向父组件发送消息
                let _ = sender.output(DiscoverOutput::PlaylistSelected(id));
            }
        }
    }
}
