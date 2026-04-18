//! 侧边栏子组件 — Player / Lyrics / Queue

use log::trace;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::gtk::Orientation;
use relm4::prelude::*;
use relm4::{adw, ComponentParts, ComponentSender, gtk};

use crate::icon_names;
use crate::player::messages::{PlaybackState, PlayerEvent};
use crate::ui::player::{PlayerPage, PlayerPageMsg, PlayerPageOutput};


pub struct Sidebar {
    stack: adw::ViewStack,
    buttons: Vec<gtk::Button>,
    current_page: String,
    player_page: Controller<PlayerPage>
}

#[derive(Debug)]
pub enum SidebarMsg {
    SwitchPage(String),
    PlayerCommand(PlayerPageOutput),
    PlayerEvent(PlayerEvent)
}

#[derive(Debug)]
pub enum SidebarOutput {
    PlayerCommand(PlayerPageOutput),
}

#[relm4::component(pub)]
impl SimpleComponent for Sidebar {
    type Init = ();
    type Input = SidebarMsg;
    type Output = SidebarOutput; 

    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                set_show_start_title_buttons: true,
                set_show_end_title_buttons: true,
            },

            #[name(stack)]
            #[wrap(Some)]
            set_content = &adw::ViewStack {},

            #[name(footer)]
            add_bottom_bar = &gtk::Box {
                set_orientation: Orientation::Horizontal,
                set_homogeneous: true,
                set_spacing: 0,
                set_margin_start: 7,
                set_margin_end: 7,
                set_margin_top: 6,
                set_margin_bottom: 6,
                add_css_class: "linked",
            },

            set_bottom_bar_style: adw::ToolbarStyle::Flat,
        }
    }


    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let player_page = PlayerPage::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| {
                SidebarMsg::PlayerCommand(msg)
            });
        let mut model = Self {
            stack: adw::ViewStack::default(),
            buttons: Vec::new(),
            current_page: "player".into(),
            player_page: player_page,
        };
        let widgets = view_output!();

        model.stack = widgets.stack.clone();

        // 添加页面到 stack
        let pages = [
            // ("player", icon_names::MUSIC_NOTE_OUTLINE, "Player", "No song playing"),
            ("lyrics", icon_names::CHAT_BUBBLE_TEXT, "Lyrics", "Lyrics will appear here"),
            ("queue", icon_names::MUSIC_QUEUE, "Queue", "Queue is empty"),
        ];

        
        widgets.stack.add_titled(model.player_page.widget(), Some("player"), "Player");

        for (name, icon, title, subtitle) in pages {
            let page = gtk::Box::builder()
                .orientation(Orientation::Vertical)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .vexpand(true)
                .spacing(12)
                .build();
            page.append(&gtk::Image::builder().icon_name(icon).pixel_size(64).opacity(0.4).build());
            page.append(&gtk::Label::builder().label(title).css_classes(["title-2"]).build());
            page.append(&gtk::Label::builder().label(subtitle).opacity(0.6).build());
            widgets.stack.add_titled(&page, Some(name), title);
        }
        widgets.stack.set_visible_child_name("player");

        // 添加按钮到 footer
        let button_defs = [
            ("player", icon_names::MUSIC_NOTE_OUTLINE, "Player"),
            ("lyrics", icon_names::CHAT_BUBBLE_TEXT, "Lyrics"),
            ("queue", icon_names::MUSIC_QUEUE, "Queue"),
        ];

        for (tag, icon, label) in button_defs {
            let btn = gtk::Button::builder().hexpand(true).build();
            btn.set_child(Some(
                &adw::ButtonContent::builder().icon_name(icon).label(label).build(),
            ));
            if tag == "player" { btn.add_css_class("raised"); } else { btn.add_css_class("flat"); }
            let s = sender.clone();
            let t = tag.to_string();
            btn.connect_clicked(move |_| s.input(SidebarMsg::SwitchPage(t.clone())));
            widgets.footer.append(&btn);
            model.buttons.push(btn);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        // eprintln!("Sidebar: {:?}", message);
        match message {
            SidebarMsg::SwitchPage(tag) => {
                self.stack.set_visible_child_name(&tag);
                for btn in &self.buttons {
                    btn.remove_css_class("raised");
                    btn.add_css_class("flat");
                }
                let idx = match tag.as_str() {
                    "player" => 0, "lyrics" => 1, "queue" => 2, _ => return,
                };
                if let Some(btn) = self.buttons.get(idx) {
                    btn.remove_css_class("flat");
                    btn.add_css_class("raised");
                }
                self.current_page = tag;
            
            },
            SidebarMsg::PlayerCommand(player_page_output) => {
                eprint!("SidebarMsg::PlayerCommand:: {:?}", player_page_output);
                sender.output(SidebarOutput::PlayerCommand(player_page_output)).ok();
                // match player_page_output {
                //     PlayerPageOutput::TogglePlay => { },
                //     PlayerPageOutput::PrevTrack => todo!(),
                //     PlayerPageOutput::NextTrack => todo!(),
                //     PlayerPageOutput::Seek(_) => todo!(),
                // }
            },
            SidebarMsg::PlayerEvent(player_event) => {
                match player_event {
                    PlayerEvent::StateChanged(state) => {
                        // 暂时只做两种状态，后续可以添加加载中，加载失败状态
                        self.player_page.emit(PlayerPageMsg::UpdatePlayback(state == PlaybackState::Playing));
                    }
                    PlayerEvent::TimeUpdated { position, duration } => {
                        // 后端发的是毫秒，UI 的 Scale 用的是秒，做个转换
                        self.player_page.emit(PlayerPageMsg::UpdateProgress {
                            position: position,
                            duration: duration,
                        });
                    }
                    PlayerEvent::TrackChanged(song) => {
                        self.player_page.emit(PlayerPageMsg::UpdateTrack {
                            title: song.name.clone(),
                            artist: song.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join("/"),
                            album: song.album.clone().name.clone(),
                            cover: song.cover_url.clone(),
                            source: "播放列表".to_string(), // 暂时写死
                        });
                    }
                    _ => {}
                }
            },
        }
    }
}
