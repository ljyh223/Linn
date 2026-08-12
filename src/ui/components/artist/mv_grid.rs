
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// mv_grid.rs  —— MvGrid（复用公共 MvCard）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent,
    factory::{FactoryVecDeque},
    gtk::{self, prelude::*},
};

use crate::api::Mv;
pub use crate::ui::components::mv_card::MvCardOutput;
use crate::ui::components::mv_card::{MvCard, MvCardInit};

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
                    guard.push_back(MvCardInit::from_duration(&mv));
                }
            }
        }
    }
}
