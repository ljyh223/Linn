use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender};
use relm4::gtk::prelude::*;
use relm4::gtk;

use crate::ui::components::image::AsyncImage;

#[derive(Debug, Clone)]
pub struct SongRowInit {
    pub id: u64,
    pub name: String,
    pub artists: String,
    pub cover_url: String,
}

#[derive(Debug, Clone)]
pub enum SongRowOutput {
    Clicked(u64),
}

pub struct SongRow {
    id: u64,
    name: String,
    artists: String,
    cover_url: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for SongRow {
    type Init = SongRowInit;
    type Input = ();
    type Output = SongRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 12,
            set_width_request: 200,
            set_margin_top: 6,
            set_margin_bottom: 6,

            AsyncImage {
                set_width_request: 52,
                set_height_request: 52,
                set_corner_radius: 6.0,
                set_placeholder_icon: "audio-x-generic-symbolic",
                set_url: self.cover_url.clone(),
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 3,
                set_hexpand: true,
                set_valign: gtk::Align::Center,

                gtk::Label {
                    set_label: &self.name,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_max_width_chars: 16,
                },

                gtk::Label {
                    set_label: &self.artists,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_max_width_chars: 16,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                },
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(SongRowOutput::Clicked(id)).unwrap();
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
            name: init.name,
            artists: init.artists,
            cover_url: init.cover_url,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}