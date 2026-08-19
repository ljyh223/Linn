use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender};
use relm4::gtk;
use relm4::gtk::prelude::*;

use crate::ui::components::image::AsyncImage;

#[derive(Debug)]
pub struct HomeBlockCardInit {
    pub index: usize,
    pub cover_url: String,
    pub title: String,
    pub subtitle: String,
    pub color: String,
}

#[derive(Debug)]
pub enum HomeBlockCardOutput {
    Clicked(usize),
}

pub struct HomeBlockCard {
    index: usize,
    cover_url: String,
    title: String,
    subtitle: String,
    color: String,
    color_class: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for HomeBlockCard {
    type Init = HomeBlockCardInit;
    type Input = ();
    type Output = HomeBlockCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 160,
            set_overflow: gtk::Overflow::Hidden,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            add_css_class: "home-block-card",

            gtk::Box {
                add_css_class: "home-block-image-wrap",
                set_overflow: gtk::Overflow::Hidden,

                AsyncImage {
                    set_width_request: 160,
                    set_height_request: 160,
                    set_corner_radius: 0.0,
                    set_url: self.cover_url.clone(),
                    set_placeholder_icon: "folder-music-symbolic",
                    set_fallback_icon: "image-missing-symbolic",
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 2,
                add_css_class: "home-block-info",
                add_css_class: &self.color_class,

                gtk::Label {
                    set_label: &self.title,
                    set_halign: gtk::Align::Start,
                    set_max_width_chars: 15,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "home-block-title",
                },

                gtk::Label {
                    set_label: &self.subtitle,
                    set_halign: gtk::Align::Start,
                    set_max_width_chars: 15,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    add_css_class: "home-block-subtitle",
                    set_visible: !self.subtitle.is_empty(),
                },
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, index = self.index] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(HomeBlockCardOutput::Clicked(index)).unwrap();
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let color_class = format!("hb-color-{}", init.index);

        let provider = gtk::CssProvider::new();
        let css = format!(".{} {{ background-color: {}; }}", color_class, init.color);
        provider.load_from_string(&css);

        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        Self {
            index: init.index,
            cover_url: init.cover_url,
            title: init.title,
            subtitle: init.subtitle,
            color: init.color,
            color_class,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}
