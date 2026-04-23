
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// album_grid.rs
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, factory::FactoryVecDeque, gtk::{self, prelude::*}
};

use crate::api::Album;
use crate::ui::components::playlist_card::{PlaylistCard, PlaylistCardInit, PlaylistCardOutput};

pub struct AlbumGrid {
    albums: FactoryVecDeque<PlaylistCard>,
}

#[derive(Debug)]
pub enum AlbumGridInput {
    SetAlbums(Vec<Album>),
}

#[relm4::component(pub)]
impl SimpleComponent for AlbumGrid {
    type Init = ();
    type Input = AlbumGridInput;
    type Output = PlaylistCardOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            #[local_ref]
            flow_box -> gtk::FlowBox {
                set_valign: gtk::Align::Start,
                set_max_children_per_line: 8,
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
        let albums = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(sender.output_sender(), |msg| msg);

        let model = AlbumGrid { albums };
        let flow_box = model.albums.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AlbumGridInput::SetAlbums(albums) => {
                let mut guard = self.albums.guard();
                guard.clear();
                for album in albums {
                    guard.push_back(PlaylistCardInit {
                        id: album.id,
                        cover_url: format!("{}?param=160y160",album.cover_url),
                        title: album.name,
                        show_play_button: true,
                    });
                }
            }
        }
    }
}