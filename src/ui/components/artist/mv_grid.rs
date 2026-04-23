
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// mv_grid.rs  —— MvCard + MvGrid
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque}, gtk::{self, prelude::*}
};

use crate::api::Mv;
use crate::ui::components::image::AsyncImage;

// ── MvCard ──────────────────────────────────────────────────

#[derive(Debug)]
pub struct MvCardInit {
    pub mv: Mv,
}

#[derive(Debug)]
pub struct MvCard {
    id: u64,
    cover: String,
    name: String,
    /// 时长（毫秒）→ "mm:ss"
    duration_str: String,
}

#[derive(Debug)]
pub enum MvCardOutput {
    Clicked(u64),
}

fn fmt_duration_ms(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[relm4::factory(pub)]
impl FactoryComponent for MvCard {
    type Init = MvCardInit;
    type Input = ();
    type Output = MvCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            set_hexpand: false,
            set_width_request: 220,

            // 封面 + 时长角标
            gtk::Overlay {
                set_width_request: 220,
                set_height_request: 124, // 16:9

                AsyncImage {
                    set_width_request: 220,
                    set_height_request: 124,
                    set_corner_radius: 8.0,
                    set_url: self.cover.clone(),
                    set_placeholder_icon: "folder-videos-symbolic",
                    set_fallback_icon: "image-missing-symbolic",
                },

                // 右下角时长标签
                add_overlay = &gtk::Label {
                    set_label: &self.duration_str,
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::End,
                    set_margin_end: 8,
                    set_margin_bottom: 6,
                    add_css_class: "mv-duration-badge", // 自定义 CSS：深色圆角背景
                },
            },

            gtk::Label {
                set_label: &self.name,
                set_halign: gtk::Align::Start,
                set_max_width_chars: 18,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "heading",
            },

            // 点击整卡
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

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            id: init.mv.id,
            cover: init.mv.cover.clone(),
            name: init.mv.name.clone(),
            duration_str: fmt_duration_ms(init.mv.duration),
        }
    }
}

// ── MvGrid ──────────────────────────────────────────────────

pub struct MvGrid {
    mvs: FactoryVecDeque<MvCard>,
}

#[derive(Debug)]
pub enum MvGridInput {
    SetMvs(Vec<Mv>),
}

#[relm4::component(pub)]
impl SimpleComponent for MvGrid {
    type Init = ();
    type Input = MvGridInput;
    type Output = MvCardOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            #[local_ref]
            flow_box -> gtk::FlowBox {
                set_valign: gtk::Align::Start,
                set_max_children_per_line: 6,
                set_min_children_per_line: 2,
                set_column_spacing: 16,
                set_row_spacing: 16,
                set_margin_all: 24,
                set_selection_mode: gtk::SelectionMode::None,
                set_homogeneous: true,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mvs = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(sender.output_sender(), |msg| msg);

        let model = MvGrid { mvs };
        let flow_box = model.mvs.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            MvGridInput::SetMvs(mvs) => {
                let mut guard = self.mvs.guard();
                guard.clear();
                for mv in mvs {
                    guard.push_back(MvCardInit { mv });
                }
            }
        }
    }
}
