//! 相关 MV 的 ListBox 行组件

use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent,
    factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque},
    gtk::{self, prelude::*},
};

use crate::api::Mv;
use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub struct MvRowInit {
    pub mv: Mv,
}

#[derive(Debug)]
pub struct MvRow {
    id: u64,
    cover: String,
    name: String,
    duration_str: String,
}

#[derive(Debug)]
pub enum MvRowOutput {
    Clicked(u64),
}

fn fmt_duration_ms(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[relm4::factory(pub)]
impl FactoryComponent for MvRow {
    type Init = MvRowInit;
    type Input = ();
    type Output = MvRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 12,
            set_margin_all: 6,
            set_valign: gtk::Align::Center,

            AsyncImage {
                set_width_request: 88,
                set_height_request: 50,
                set_corner_radius: 6.0,
                set_url: format!("{}?param=100y56", self.cover),
                set_placeholder_icon: "folder-videos-symbolic",
                set_fallback_icon: "image-missing-symbolic",
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_hexpand: true,

                gtk::Label {
                    set_label: &self.name,
                    set_halign: gtk::Align::Start,
                    set_max_width_chars: 24,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "heading",
                },

                gtk::Label {
                    set_label: &self.duration_str,
                    set_halign: gtk::Align::Start,
                    add_css_class: "caption",
                    add_css_class: "dim-label",
                },
            },

            gtk::Button {
                set_icon_name: "media-playback-start-symbolic",
                add_css_class: "circular",
                add_css_class: "flat",
                set_tooltip_text: Some("播放"),
                connect_clicked[sender, id = self.id] => move |_| {
                    sender.output(MvRowOutput::Clicked(id)).unwrap();
                },
            },
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

/// 相关 MV 的简单容器（ListBox + 可滚动）
pub struct MvList {
    rows: FactoryVecDeque<MvRow>,
}

#[derive(Debug)]
pub enum MvListInput {
    SetMvs(Vec<Mv>),
}

#[relm4::component(pub)]
impl SimpleComponent for MvList {
    type Init = ();
    type Input = MvListInput;
    type Output = MvRowOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            #[local_ref]
            list_box -> gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::None,
                set_show_separators: true,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let rows = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.output_sender(), |msg| msg);

        let model = MvList { rows };
        let list_box = model.rows.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            MvListInput::SetMvs(mvs) => {
                let mut guard = self.rows.guard();
                guard.clear();
                for mv in mvs {
                    guard.push_back(MvRowInit { mv });
                }
            }
        }
    }
}
