use crate::api::{Artist, get_artist_detail};
use crate::ui::components::image::AsyncImage;
use futures::FutureExt;
use gst::glib::object::ObjectExt;
use relm4::adw::prelude::AdwDialogExt;
use relm4::gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{Component, ComponentParts, ComponentSender, FactorySender, RelmWidgetExt, adw, gtk};

// ─── ArtistItem（Factory 子项）──────────────────────────────────────────────

pub struct ArtistItem {
    artist: Artist,
    avatar_url: Option<String>,
}

pub struct ArtistItemInit {
    pub artist: Artist,
}

#[derive(Debug)]
pub enum ArtistItemMsg {
    AvatarLoaded(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for ArtistItem {
    type Init = ArtistItemInit;
    type Input = ArtistItemMsg;
    type Output = u64; // 输出点击的 artist id
    type CommandOutput = (); // 子项无需异步
    type ParentWidget = gtk::Box;

    view! {
    gtk::Box {
        set_orientation: gtk::Orientation::Horizontal,
        set_spacing: 12,
        set_margin_top: 6,
        set_margin_bottom: 6,
        set_margin_start: 16,
        set_margin_end: 16,

        AsyncImage {
            set_width_request: 40,
            set_height_request: 40,
            set_corner_radius: 20.0,
            #[watch]
            set_url: format!("{}?param=100y100", self.avatar_url.clone().unwrap_or_default()),
        },

        gtk::Label {
            set_label: &self.artist.name,
            set_halign: gtk::Align::Start,
            set_valign: gtk::Align::Center,
            set_hexpand: true,
        },

        gtk::Button {
            set_label: "查看",
            set_valign: gtk::Align::Center,
            // add_css_class: "pill",        // adwaita 圆角按钮样式
            connect_clicked[sender, id = self.artist.id] => move |_| {
                sender.output(id).ok();
            }
        }
    }
}

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            artist: init.artist,
            avatar_url: None,
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            ArtistItemMsg::AvatarLoaded(url) => {
                self.avatar_url = Some(url);
            }
        }
    }
}

// ─── ArtistDialog ────────────────────────────────────────────────────────────

pub struct ArtistDialog {
    factory: FactoryVecDeque<ArtistItem>,
}

#[derive(Debug)]
pub enum ArtistDialogMsg {
    ArtistClicked(u64),
    FetchAvatar(u64),
}

// Component trait 才有 CommandOutput，SimpleComponent 没有
#[derive(Debug)]
pub enum ArtistDialogCmdMsg {
    AvatarFetched { artist_id: u64, url: String },
    FetchFailed(u64),
}

#[relm4::component(pub)]
impl Component for ArtistDialog {
    type Init = Vec<Artist>;
    type Input = ArtistDialogMsg;
    type Output = u64;
    type CommandOutput = ArtistDialogCmdMsg;

    view! {
    #[root]
    adw::Dialog {
        set_title: "Artists",
        set_content_width: 350,
        // 删掉 set_content_height: 400,
        set_follows_content_size: true,  // 高度跟随内容
    
        #[wrap(Some)]
        set_child = &gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_propagate_natural_height: true,  // ScrolledWindow 把内容高度传递出去
            set_max_content_height: 500,          // 最大高度，超过才出现滚动条

            #[local_ref]
            factory_box -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_top: 8,
                set_margin_bottom: 8,
                set_spacing: 4,
            }
        }
    }
}

    fn init(
        artists: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut factory = FactoryVecDeque::builder()
            .launch(gtk::Box::new(gtk::Orientation::Vertical, 0))
            .forward(sender.input_sender(), ArtistDialogMsg::ArtistClicked);

        {
            let mut guard = factory.guard();
            for artist in &artists {
                guard.push_back(ArtistItemInit { artist: artist.clone() });
            }
        }

        for artist in &artists {
            sender.input(ArtistDialogMsg::FetchAvatar(artist.id));
        }

        let model = ArtistDialog { factory };

        // 关键：在 view_output!() 之前定义这个局部变量，名字对应 view! 里的 #[local_ref]
        let factory_box = model.factory.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            ArtistDialogMsg::ArtistClicked(id) => {
                sender.output(id).ok();
                root.close();
            }
            ArtistDialogMsg::FetchAvatar(id) => {
                // sender.command() 是 Component trait 提供的异步命令接口
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match get_artist_detail(id).await {
                                Ok(detail) => {
                                    out.send(ArtistDialogCmdMsg::AvatarFetched {
                                        artist_id: id,
                                        url: detail.avatar,
                                    })
                                    .ok();
                                }
                                Err(_) => {
                                    out.send(ArtistDialogCmdMsg::FetchFailed(id)).ok();
                                }
                            }
                        })
                        .drop_on_shutdown()
                        .boxed()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ArtistDialogCmdMsg::AvatarFetched { artist_id, url } => {
                let index = {
                    let guard = self.factory.guard();
                    guard.iter().position(|item| item.artist.id == artist_id)
                }; // guard 在这里 drop，可变借用释放

                if let Some(index) = index {
                    self.factory.send(index, ArtistItemMsg::AvatarLoaded(url)); // 现在安全
                }
            }
            ArtistDialogCmdMsg::FetchFailed(_) => {
                // 保持默认头像，忽略即可
            }
        }
    }
}
