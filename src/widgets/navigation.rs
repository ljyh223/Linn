use gtk::prelude::*;
use relm4::{gtk, ComponentParts, SimpleComponent};

#[derive(Debug)]
pub enum NavigationInput {
    // 切换选中的导航项
    SetActive(NavigationItem),
}

#[derive(Debug)]
pub enum NavigationOutput {
    Recommend,
    Discover,
    MyCollection,
    MyFavorites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    Recommend,
    Discover,
    MyCollection,
    MyFavorites,
}

pub struct Navigation {
    active_item: NavigationItem,
}

impl SimpleComponent for Navigation {
    type Init = ();
    type Input = NavigationInput;
    type Output = NavigationOutput;
    type Widgets = NavigationWidgets;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_css_classes: &["navigation", "sidebar"],

            // 应用标题
            append = &gtk::Label {
                set_label: "Linn",
                set_css_classes: &["title-label"],
                set_margin_top: 20,
                set_margin_bottom: 20,
                set_margin_start: 12,
                set_halign: gtk::Align::Start,
                add_css_class: "heading"
            }

            // 导航按钮组
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_margin_start: 8,
                set_margin_end: 8,
                set_hexpand: true,

                // 为我推荐
                append: nav_button = &gtk::Button {
                    set_label: "为我推荐",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::Recommend);
                    }
                },

                // 发现音乐
                append: nav_button = &gtk::Button {
                    set_label: "发现音乐",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::Discover);
                    }
                },

                // 分隔线
                append = &gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_margin_top: 12,
                    set_margin_bottom: 12,
                }

                // 我的收藏
                append: nav_button = &gtk::Button {
                    set_label: "我的收藏",
                    set_hexpand: true,
                    set_css_classes: &["navigation-button"],
                    connect_clicked[sender] => move |_| {
                        sender.output(NavigationOutput::MyCollection);
                    }
                },

                // 我喜欢的音乐
                append: nav_button = &gtk::Button {
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
        _root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Navigation {
            active_item: NavigationItem::Recommend,
        };

        let widgets = view_output!();

        // 设置默认选中状态
        widgets
            .navigation_buttons
            .get(0)
            .unwrap()
            .set_css_classes(&["navigation-button", "selected"]);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            NavigationInput::SetActive(item) => {
                self.active_item = item;
            }
        }
    }

    fn pre_view() {
        // 更新按钮的选中状态
        // TODO: 实现 UI 更新逻辑
    }
}
