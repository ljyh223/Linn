use relm4::prelude::FactoryVecDeque;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use relm4::gtk;
use relm4::gtk::prelude::*;
use std::sync::Arc;

use crate::components::AsyncImage;
use crate::pages::playlist_detail::song_item::SongItemOutput;
use crate::pages::playlist_detail::{
    model::*,
    msg::*
};

#[relm4::component(pub)]
impl SimpleComponent for PlaylistDetail {
    type Init = u64;
    type Input = PlaylistDetailMsg;
    type Output = ();

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_bottom: 24,
                set_margin_top: 24,
                set_margin_start: 24,
                set_margin_end: 24,
                set_spacing: 24,

                // Header
                gtk::Box {
                    set_spacing: 24,
                    

                    gtk::AspectFrame {
                        set_ratio: 1.0,
                        set_obey_child: false,
                        
                        #[name = "cover"]
                        AsyncImage {
                            set_width_request: 160,
                            set_height_request: 160,
                            set_border_radius: 24,
                        }
                    },
                    
                    
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_hexpand: true,

                        #[name = "title"]
                        gtk::Label {
                            add_css_class: "playlist-title",
                            set_halign: gtk::Align::Start,
                        },

                        #[name = "meta"]
                        gtk::Label {
                            add_css_class: "playlist-meta",
                            set_halign: gtk::Align::Start,
                        },

                        #[name = "desc"]
                        gtk::Label {
                            set_wrap: true,
                            set_halign: gtk::Align::Start,
                        },
                    }
                },

                gtk::Separator {},

                #[name = "songs_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                }
            }
        }
    }

    fn init(id: u64, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let widgets = view_output!();

        let songs: FactoryVecDeque<SongData> = FactoryVecDeque::builder()
            .launch(widgets.songs_box.clone())
            .forward(sender.input_sender(), |o| match o {
                SongItemOutput::Play(id) => PlaylistDetailMsg::PlaySong(id),
            });

        let model = PlaylistDetail {
            detail: None,
            songs: Vec::new(),
            api: Arc::new(netease_cloud_music_api::MusicApi::default()),
            playlist_id: id,
            search: String::new(),
            loading: false,
        };

        sender.input(PlaylistDetailMsg::Load(id));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlaylistDetailMsg::Load(id) => {
                let api = self.api.clone();
                self.loading = true;

                relm4::gtk::glib::MainContext::default().spawn_local(async move {
                    if let Ok(p) = api.song_list_detail(id).await {
                        let detail = DetailData {
                            id: p.id,
                            name: p.name,
                            cover: p.cover_img_url,
                            description: Some(p.description),
                            creator: None,
                            song_count: p.songs.len() as u64,
                            play_count: 0,
                        };

                        let songs = p.songs.into_iter().map(|s| SongData {
                            id: s.id,
                            name: s.name,
                            artist: s.singer,
                            album: s.album,
                            cover: s.pic_url,
                            duration: s.duration,
                        }).collect();

                        sender.input(PlaylistDetailMsg::Loaded { detail, songs });
                    }
                });
            }

            PlaylistDetailMsg::Loaded { detail, songs } => {
                self.detail = Some(detail);
                self.loading = false;

                self.songs.clear();
                self.songs.extend(songs);
            }

            PlaylistDetailMsg::PlaySong(id) => {
                eprintln!("▶ 播放 {}", id);
            }

            PlaylistDetailMsg::Search(text) => {
                self.search = text;
            }
        }
    }

    fn pre_view() {
        if let Some(d) = &model.detail {
            widgets.cover.set_src(&d.cover);
            widgets.title.set_label(&d.name);
            widgets.meta.set_label(&format!(
                "{} 首歌曲 · 播放 {}",
                d.song_count,
                d.play_count
            ));

            if let Some(desc) = &d.description {
                widgets.desc.set_label(desc);
            }
        }
    }
}
