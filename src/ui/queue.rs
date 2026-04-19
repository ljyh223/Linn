use std::sync::Arc;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{
    gtk, prelude::*, ComponentParts, ComponentSender,
    factory::FactoryVecDeque,
};

use crate::api::Song;
use crate::ui::components::image::AsyncImage;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 1. 消息定义
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug)]
pub enum QueueMsg {
    /// 接收全新队列 (切歌单时)
    SetQueue{songs: Arc<Vec<Song>>, start_index: usize},
    /// 仅更新当前播放索引 (上一首/下一首时)
    SetCurrentIndex(usize),
    /// 清空队列
    Clear,
    /// 内部：工厂子组件的事件转发
    RowAction(QueueRowOutput),
}

#[derive(Debug)]
pub enum QueuePageOutput {
    Play(usize),
    Remove(usize),
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 2. 子组件工厂 (QueueRow) - 完全模仿你的 TrackRow 写法
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug)]
pub struct QueueRowInit {
    pub index: usize,
    pub song: Arc<Song>,
    pub is_playing: bool,
}

#[derive(Debug)]
pub enum QueueRowOutput {
    Play(usize),
    Remove(usize),
}

#[derive(Debug)]
pub struct QueueRow {
    index_str: String, // 改为 String，避免 to_string() 生命周期报错
    song: Arc<Song>,
    is_playing: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for QueueRow {
    type Init = QueueRowInit;
    type Input = ();
    type Output = QueueRowOutput; 
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
            set_margin_all: 8,
            set_valign: gtk::Align::Center,

            // --- 1. 左侧：序号 / 正在播放图标 ---
            gtk::Box {
                set_width_request: 16,
                set_halign: gtk::Align::Center,

                // 正在播放时的图标
                gtk::Image {
                    #[watch]
                    set_visible: self.is_playing,
                    set_icon_name: Some("media-playback-start-symbolic"),
                    add_css_class: "accent",
                },

                // 非播放时的数字序号
                gtk::Label {
                    #[watch]
                    set_visible: !self.is_playing,
                    set_text: &self.index_str, 
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                }
            },

            AsyncImage {
                set_width_request: 48,
                set_height_request: 48,
                set_url: format!("{}?param=100y100",self.song.cover_url.clone()),
                set_corner_radius: 4.0,
                set_placeholder_icon: "missing-album-symbolic",
            },

            // --- 2. 中间：歌名与歌手 ---
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_valign: gtk::Align::Center,
                set_spacing: 4,
                set_hexpand: true,

                gtk::Label {
                    set_label: &self.song.name,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    
                    #[watch]
                    set_css_classes: if self.is_playing {
                        &["heading", "accent"]
                    } else {
                        &["heading"]
                    },
                },
                gtk::Label {
                    set_label: &self.song.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                }
            },

            // --- 3. 右侧：移除按钮 ---
            gtk::Button {
                set_icon_name: "window-close-symbolic",
                set_valign: gtk::Align::Center,
                add_css_class: "circular",
                add_css_class: "flat",
                set_tooltip_text: Some("从队列移除"),
                
                // 注意这里要改成 self.index_str.parse() 或者直接用闭包捕获前的变量
                // 最简单的是在 init 时把 usize 也存下来
                connect_clicked[sender, index = self.index_str.parse::<usize>().unwrap_or(0)] => move |_| {
                    sender.output(QueueRowOutput::Remove(index)).unwrap();
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        // 提取出来，方便闭包使用
        let index = init.index; 
        Self {
            index_str: index.to_string(), // 预先转为 String
            song: init.song,
            is_playing: init.is_playing,
        }
    }
}



pub struct QueuePage {
    queue: FactoryVecDeque<QueueRow>,
    current_index: usize,
}



#[relm4::component(pub)]
impl Component for QueuePage {
    type Init = ();
    type Input = QueueMsg;
    type Output = QueuePageOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_vexpand: true,

            #[local_ref]
            list_box -> gtk::ListBox {
                add_css_class: "boxed-list",
                add_css_class: "rich-list",
                set_selection_mode: gtk::SelectionMode::None,
                set_show_separators: false,
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            queue: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), QueueMsg::RowAction),
            current_index: 0,
        };

        let list_box = model.queue.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            QueueMsg::SetQueue { songs, start_index } => {
                self.queue.guard().clear();
                
                let mut guard = self.queue.guard();
                for (index, song) in songs.iter().enumerate() {
                    guard.push_back(QueueRowInit {
                        index,
                        song: Arc::new(song.clone()), // 零成本克隆指针
                        is_playing: index == start_index,
                    });
                }
                drop(guard); // 释放锁让 UI 更新
                
                self.current_index = start_index;
            }

            QueueMsg::SetCurrentIndex(new_index) => {
                if new_index == self.current_index { return; }
                let old_index = self.current_index;

                // 【性能核心】：精准局部刷新，不重建列表
                let mut guard = self.queue.guard();
                if let Some(old_row) = guard.get_mut(old_index) {
                    old_row.is_playing = false;
                }
                if let Some(new_row) = guard.get_mut(new_index) {
                    new_row.is_playing = true;
                }

                self.current_index = new_index;
            }

            QueueMsg::Clear => {
                self.queue.guard().clear();
                self.current_index = 0;
            }

            QueueMsg::RowAction(row_msg) => {
                match row_msg {
                    QueueRowOutput::Play(index) => sender.output(QueuePageOutput::Play(index)).unwrap(),
                    QueueRowOutput::Remove(index) => sender.output(QueuePageOutput::Remove(index)).unwrap(),
                }
            }
        }
    }
}
