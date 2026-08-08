use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender};
use relm4::gtk::prelude::*;
use relm4::gtk;

#[derive(Debug, Clone)]
pub struct SuggestSongInit {
    pub id: u64,
    pub name: String,
    #[doc = "已拼接好的歌手名，如 `周杰伦, 蔡依林`"]
    pub artists: String,
}

#[derive(Debug, Clone)]
pub struct SuggestEntityInit {
    pub id: u64,
    pub icon_name: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone)]
pub enum SuggestRowOutput {
    Clicked(u64),
}

// ---------------------------------------------------------------

pub struct SuggestSongRow {
    id: u64,
    name: String,
    artists: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for SuggestSongRow {
    type Init = SuggestSongInit;
    type Input = ();
    type Output = SuggestRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 10,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_margin_start: 8,
            set_margin_end: 8,

            gtk::Image {
                set_icon_name: Some("audio-x-generic-symbolic"),
                set_pixel_size: 18,
            },

            gtk::Label {
                set_label: &format!("{} - {}", self.name, self.artists),
                set_halign: gtk::Align::Start,
                set_hexpand: true,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(SuggestRowOutput::Clicked(id)).unwrap();
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
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}

// -------------------------------------------------

pub struct SuggestEntityRow {
    id: u64,
    icon_name: String,
    title: String,
    subtitle: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for SuggestEntityRow {
    type Init = SuggestEntityInit;
    type Input = ();
    type Output = SuggestRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 10,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_margin_start: 8,
            set_margin_end: 8,

            gtk::Image {
                set_icon_name: Some(&self.icon_name),
                set_pixel_size: 18,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 2,

                gtk::Label {
                    set_label: &self.title,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },

                gtk::Label {
                    set_label: &self.subtitle,
                    set_halign: gtk::Align::Start,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                },
            },

            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_released[sender, id = self.id] => move |_, n_press, _, _| {
                    if n_press == 1 {
                        sender.output(SuggestRowOutput::Clicked(id)).unwrap();
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
            icon_name: init.icon_name,
            title: init.title,
            subtitle: init.subtitle,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {}
}