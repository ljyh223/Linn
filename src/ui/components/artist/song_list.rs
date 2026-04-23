
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// song_list.rs
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
use relm4::{
    ComponentParts, ComponentSender, SimpleComponent,
    factory::FactoryVecDeque,
    gtk::{self, prelude::*},
};

use crate::api::Song;
use crate::ui::components::track_row::{TrackRow, TrackRowInit, TrackRowOutput};

pub struct SongList {
    tracks: FactoryVecDeque<TrackRow>,
}

#[derive(Debug)]
pub enum SongListInput {
    /// ArtistPage 加载完数据后调用，替换整个列表
    SetSongs(Vec<Song>),
}

#[relm4::component(pub)]
impl SimpleComponent for SongList {
    type Init = ();
    type Input = SongListInput;
    type Output = TrackRowOutput; // 直接透传给父组件

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_margin_start: 24,
            set_margin_end: 24,

            #[local_ref]
            track_list_box -> gtk::ListBox {
                add_css_class: "boxed-list",
                add_css_class: "rich-list",
                set_selection_mode: gtk::SelectionMode::None,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // FactoryVecDeque 需要在 view_output!() 之前创建，
        // 并以 &local_ref 的方式注入到 view! 宏中。
        let tracks = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.output_sender(), |msg| msg);

        let model = SongList { tracks };

        // track_list_box 是 view! 里 #[local_ref] 引用的变量名
        let track_list_box = model.tracks.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SongListInput::SetSongs(songs) => {
                // 清空后重新填充
                let mut guard = self.tracks.guard();
                guard.clear();
                for (index, track) in songs.into_iter().enumerate() {
                    guard.push_back(TrackRowInit { track, index });
                }
            }
        }
    }
}