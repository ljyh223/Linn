use relm4::{gtk::{self, prelude::*}, ComponentParts, ComponentSender, SimpleComponent};

#[derive(Debug)]
pub enum NavigationOutput {
    Recommend,
    Discover,
    MyCollection,
    MyFavorites,
}

pub struct Navigation {
    active_item: NavigationItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    Recommend,
    Discover,
    MyCollection,
    MyFavorites,
}

#[relm4::component(pub)]
impl SimpleComponent for Navigation {
    type Init = ();
    type Input = ();
    type Output = NavigationOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_css_classes: &["navigation", "sidebar"],

            gtk::Label {
                set_label: "Linn",
                set_margin_top: 20,
                set_margin_bottom: 20,
                set_margin_start: 12,
                set_halign: gtk::Align::Start,
                add_css_class: "heading",
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_margin_start: 8,
                set_margin_end: 8,
                set_hexpand: true,

                gtk::Button {
                    set_label: "为我推荐",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::Recommend);
                    }
                },

                gtk::Button {
                    set_label: "发现音乐",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::Discover);
                    }
                },

                gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_margin_top: 12,
                    set_margin_bottom: 12,
                },

                gtk::Button {
                    set_label: "我的收藏",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::MyCollection);
                    }
                },

                gtk::Button {
                    set_label: "我喜欢的音乐",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::MyFavorites);
                    }
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Navigation {
            active_item: NavigationItem::Recommend,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}
