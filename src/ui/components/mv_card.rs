//! 公共 MV 卡组件（宏模板生成 FlowBox / Box 两个变体）
//!
//! 用法：
//! - `MvCard`     → 父容器 `gtk::FlowBox`（artist 页 MvGrid 使用）
//! - `BoxMvCard`  → 父容器 `gtk::Box`（Explore 横向滚动行使用）

use relm4::gtk::prelude::*;
use relm4::{gtk, prelude::*};

use crate::api::Mv;
use crate::ui::components::image::AsyncImage;

// ------------------- 公共数据结构 -------------------

#[derive(Debug)]
pub struct MvCardInit {
    pub id: u64,
    pub cover_url: String,
    pub name: String,
    /// 角标文本（如 "03:24" 或 "1.2万 播放"），None 时不显示
    pub badge: Option<String>,
}

impl MvCardInit {
    /// 直接由 Mv 构造，角标显示时长（mm:ss）
    pub fn from_duration(mv: &Mv) -> Self {
        Self {
            id: mv.id,
            cover_url: mv.cover.clone(),
            name: mv.name.clone(),
            badge: Some(fmt_duration_ms(mv.duration)),
        }
    }

    /// 直接由 Mv 构造，角标显示播放量（如 "1.2万"）
    pub fn from_play_count(mv: &Mv) -> Self {
        Self {
            id: mv.id,
            cover_url: mv.cover.clone(),
            name: mv.name.clone(),
            badge: Some(fmt_play_count(mv.play_count)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MvCardOutput {
    Clicked(u64),
}

pub fn fmt_duration_ms(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

pub fn fmt_play_count(count: u64) -> String {
    if count >= 100_000_000 {
        format!("{:.1}亿", count as f64 / 100_000_000.0)
    } else if count >= 10_000 {
        format!("{:.1}万", count as f64 / 10_000.0)
    } else {
        count.to_string()
    }
}

// ------------------- 核心宏：消除重复代码 -------------------

macro_rules! define_mv_card {
    ($name:ident, $parent_widget:ty) => {
        pub struct $name {
            id: u64,
            cover_url: String,
            name: String,
            badge: Option<String>,
        }

        #[relm4::factory(pub)]
        impl FactoryComponent for $name {
            type Init = MvCardInit;
            type Input = ();
            type Output = MvCardOutput;
            type CommandOutput = ();
            type ParentWidget = $parent_widget;

            view! {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    set_hexpand: false,
                    set_width_request: 220,

                    gtk::Overlay {
                        set_width_request: 220,
                        set_height_request: 124, // 16:9

                        AsyncImage {
                            set_width_request: 220,
                            set_height_request: 124,
                            set_corner_radius: 8.0,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_url: format!("{}?param=250y140", self.cover_url),
                            set_placeholder_icon: "folder-videos-symbolic",
                            set_fallback_icon: "image-missing-symbolic",
                        },

                        add_overlay = &gtk::Label {
                            set_label: self.badge.as_deref().unwrap_or_default(),
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::End,
                            set_margin_end: 8,
                            set_margin_bottom: 6,
                            set_visible: self.badge.is_some(),
                            add_css_class: "mv-duration-badge",
                        },
                    },

                    gtk::Label {
                        set_label: &self.name,
                        set_halign: gtk::Align::Start,
                        set_max_width_chars: 18,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        add_css_class: "heading",
                    },

                    add_controller = gtk::GestureClick {
                        set_button: 1,
                        connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                            if n_press == 1 {
                                sender.output(MvCardOutput::Clicked(id)).unwrap();
                            }
                        }
                    }
                }
            }

            fn init_model(
                init: Self::Init,
                _index: &DynamicIndex,
                _sender: FactorySender<Self>,
            ) -> Self {
                Self {
                    id: init.id,
                    cover_url: init.cover_url,
                    name: init.name,
                    badge: init.badge,
                }
            }

            fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
        }
    };
}

// 1. 用于 FlowBox 的卡片（artist 页 MV 网格）
define_mv_card!(MvCard, gtk::FlowBox);

// 2. 用于普通 gtk::Box 的卡片（Explore 横向滚动行）
define_mv_card!(BoxMvCard, gtk::Box);
