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
            set_hexpand: false, 
            set_vexpand: false,
            set_width_request: 156,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_all: 8,
                set_hexpand: false,
                set_halign: gtk::Align::Start, // 或 Align::Center

                #[name(cover_container)]
                gtk::Box {
                    set_width_request: 140,
                    set_height_request: 140,
                    set_halign: gtk::Align::Center,
                    set_hexpand: false,
                    set_vexpand: false,
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
        returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        returned_widget.set_hexpand(false);

        // 获取 AsyncImage 的 widget
        let image_widget = self.image_controller.widget();
        image_widget.set_halign(gtk::Align::Fill);
        image_widget.set_valign(gtk::Align::Fill);
        image_widget.set_size_request(140, 140);

        widgets.cover_container.append(image_widget);

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
    playlists: FactoryVecDeque<PlaylistCard>
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
            set_hexpand: true,



            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 24, // 统一 margin
                set_hexpand: true,
                set_vexpand: true,

                gtk::Label {
                    set_label: "热门歌单",
                    add_css_class: "title-2",
                    set_halign: gtk::Align::Start,
                },

    
                gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    set_vexpand: true,
                    
                    // 状态绑定：根据 loading 切换页面
                    #[watch]
                    set_visible_child_name: if model.loading { "loading" } else { "content" },

                    // 页面 1：加载动画
                    add_named[Some("loading")] = &gtk::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_spinning: true, // 只要显示出来就一直转
                    },

                    // 页面 2：内容网格
                    // 1. 属性放在最前面
                    #[name(container)]
                    add_named[Some("content")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                    },
                }
            }
        }
    }

     fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let playlists = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::default())
            .forward(sender.input_sender(), |output| match output {
                PlaylistCardOutput::Selected(id) => DiscoverInput::PlaylistSelected(id),
            });

        let model = DiscoverModel {
            loading: true,
            playlists,
        };

        let widgets = view_output!();

        // 配置 FlowBox
        let flowbox = model.playlists.widget();
        flowbox.set_selection_mode(gtk::SelectionMode::None);
        flowbox.set_column_spacing(12);
        flowbox.set_row_spacing(12);

        // 【修改点】：允许最少1个，最多很多个（比如30个）
        // flowbox.set_min_children_per_line(1); 
        flowbox.set_max_children_per_line(30); // 把它改成 20、30 或更大，千万别是 1

        // FlowBox 必须占满横向空间，这样它才能计算能不能塞下更多列
        flowbox.set_halign(gtk::Align::Fill); 
        flowbox.set_valign(gtk::Align::Start);
        flowbox.set_hexpand(true);
        
        widgets.container.append(flowbox);

        sender.input(DiscoverInput::LoadPlaylists);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            DiscoverInput::LoadPlaylists => {
                self.loading = true; // 触发 view 更新，切换到 spinner
                let sender_clone = sender.clone();
                sender.command(|_out, _shutdown| {
                    async move {
                        // ... API 请求代码不变 ...
                        // 这里稍微简写了
                        let api = MusicApi::default();
                        let res = api.top_song_list("全部", "hot", 0, 20).await;
                         match res {
                            Ok(playlists) => {
                                let data = playlists.into_iter().map(PlaylistData::from).collect();
                                sender_clone.input(DiscoverInput::UpdatePlaylists(data));
                            }
                            Err(_) => {
                                sender_clone.input(DiscoverInput::UpdatePlaylists(Vec::new()));
                            }
                        }
                    }
                });
            }
            DiscoverInput::UpdatePlaylists(data) => {
                self.loading = false; // 触发 view 更新，切换到 content
                let mut guard = self.playlists.guard();
                guard.clear();
                for item in data {
                    guard.push_back(item);
                }
                // 不需要手动 set_visible 了，#[watch] 会处理 Stack 的切换
            }
            DiscoverInput::PlaylistSelected(id) => {
                let _ = sender.output(DiscoverOutput::PlaylistSelected(id));
            }
        }
    }
}
