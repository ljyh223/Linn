use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender};
use relm4::gtk;
use relm4::gtk::prelude::*;

use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub struct ArtistCardInit {
    pub id: u64,
    pub avatar_url: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum ArtistCardOutput {
    Clicked(u64),
}

pub struct ArtistCard {
    id: u64,
    avatar_url: String,
    name: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for ArtistCard {
    type Init = ArtistCardInit;
    type Input = ();
    type Output = ArtistCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            set_hexpand: false,
            set_vexpand: false,
            set_width_request: 110,
            add_css_class: "artist-card",

            AsyncImage {
                set_width_request: 110,
                set_height_request: 110,
                set_corner_radius: 55.0,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_url: self.avatar_url.clone(),
                set_placeholder_icon: "avatar-default-symbolic",
                set_fallback_icon: "image-missing-symbolic",
            },

            gtk::Label {
                set_label: &self.name,
                set_halign: gtk::Align::Center,
                set_max_width_chars: 12,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: "caption",
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(ArtistCardOutput::Clicked(id)).unwrap();
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            id: init.id,
            avatar_url: init.avatar_url,
            name: init.name,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}
