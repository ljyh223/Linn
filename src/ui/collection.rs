use relm4::gtk::prelude::*;
use relm4::{Component, ComponentParts, ComponentSender, gtk};

pub struct Collection {}
#[derive(Debug)]
pub enum CollectionMsg {}
#[derive(Debug)]
pub enum CollectionCmdMsg {}
#[derive(Debug)]
pub enum CollectionOutput {
    OpenPlaylistDetail(u64)
}

#[relm4::component(pub)]
impl Component for Collection {
    type Init = ();
    type Input = CollectionMsg;
    type CommandOutput = CollectionCmdMsg;
    type Output = CollectionOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            gtk::Label {
                set_label: "收藏页面 - 建设中 ❤️",
                add_css_class: "title-1",
            }
        }
    }

    fn init(_init: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root){
        match message {
            // 这里可以添加一些交互逻辑，比如点击某个歌单卡片时发送 OpenPlaylistDetail 消息
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}