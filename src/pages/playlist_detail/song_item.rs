use relm4::factory::{FactoryComponent, FactorySender, DynamicIndex};
use relm4::gtk;
use relm4::gtk::prelude::*;

use crate::components::AsyncImage;
use crate::pages::playlist_detail::model::SongData;
use crate::utils::utils::format_duration;


#[derive(Debug)]
struct SongItem {
    pub data: SongData,
    index: usize,
}

#[derive(Debug)]
pub enum SongItemOutput {
    Play(u64),
}

#[relm4::factory]
impl FactoryComponent for SongItem {
    type Init = SongData;
    type Input = ();
    type Output = SongItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_spacing: 12,
            set_margin_bottom: 8,
            set_margin_start: 8,
            set_margin_end: 8,
            set_margin_top: 8,
            set_hexpand: true,
            add_css_class: "song-row",

            gtk::Label {
                set_label: &format!("{:02}", self.index + 1),
                add_css_class: "song-index",
            },
            #[name = "song_cover"]
            AsyncImage {
                set_width_request: 40,
                set_height_request: 40,
                set_border_radius: 6,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,

                gtk::Label {
                    set_label: &self.data.name,
                    set_halign: gtk::Align::Start,
                    add_css_class: "song-title",
                },

                gtk::Label {
                    set_label: &format!("{} · {}", self.data.artist, self.data.album),
                    set_halign: gtk::Align::Start,
                    add_css_class: "song-subtitle",
                }
            },

            gtk::Label {
                set_label: &format_duration(self.data.duration),
                add_css_class: "song-duration",
            }
        }
    }

    fn init_model(
        data: Self::Init,
        index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            data,
            index: index.current_index(),
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets.song_cover.set_src(&self.data.cover);

        let song_id = self.data.id;
        // root.connect_button_press_event(move |_, _| {
        //     let _ = sender.output(SongItemOutput::Play(song_id));
        //     gtk::glib::Propagation::Stop
        // });

        widgets
    }
}
