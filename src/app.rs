use relm4::prelude::*;
use relm4::adw;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::adw::prelude::*;
use crate::pages::{Page, create_page_icon};
use crate::ui::discover::{DiscoverModel, DiscoverOutput};

#[derive(Debug)]
pub enum AppInput {
    Navigate(Page),
}

pub struct AppModel {
    current_page: Page,
    discover_controller: Controller<DiscoverModel>,
    // 存储导航图标
    discover_icon: gtk::Image,
    explore_icon: gtk::Image,
    library_icon: gtk::Image,
    favorites_icon: gtk::Image,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppInput;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_default_width: 1100,
            set_default_height: 750,

            // 使用 Box 作为主容器
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                // 主内容区域
                gtk::Paned {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_vexpand: true,
                    set_wide_handle: false,
                    set_resize_start_child: false,
                    set_shrink_start_child: false,

                    // 左侧边栏 - 固定宽度，不扩展
                    #[wrap(Some)]
                    set_start_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_width_request: 250,
                        set_hexpand: false,
                        add_css_class: "sidebar",

                        // Logo 区域
                        gtk::Label {
                            set_label: "Linn",
                            add_css_class: "title-1",
                            set_margin_start: 20,
                            set_margin_end: 20,
                            set_margin_top: 20,
                            set_margin_bottom: 10,
                        },

                        gtk::Separator {
                            set_margin_bottom: 10,
                        },

                        // 导航列表
                        gtk::ListBox {
                            add_css_class: "navigation-sidebar",
                            set_selection_mode: gtk::SelectionMode::None,
                            set_vexpand: true,

                            // 发现音乐按钮
                            append = &adw::ActionRow {
                                set_title: Page::Discover.title(),
                                set_activatable: true,
                                add_prefix: &model.discover_icon,
                                connect_activated[sender] => move |_| {
                                    sender.input(AppInput::Navigate(Page::Discover))
                                },
                            },
                            // 探索页面
                            append = &adw::ActionRow {
                                set_title: Page::Explore.title(),
                                set_activatable: true,
                                add_prefix: &model.explore_icon,
                                connect_activated[sender] => move |_| {
                                    sender.input(AppInput::Navigate(Page::Explore))
                                },
                            },
                            // 我的收藏
                            append = &adw::ActionRow {
                                set_title: Page::Library.title(),
                                set_activatable: true,
                                add_prefix: &model.library_icon,
                                connect_activated[sender] => move |_| {
                                    sender.input(AppInput::Navigate(Page::Library))
                                },
                            },
                            // 我喜欢的歌曲
                            append = &adw::ActionRow {
                                set_title: Page::Favorites.title(),
                                set_activatable: true,
                                add_prefix: &model.favorites_icon,
                                connect_activated[sender] => move |_| {
                                    sender.input(AppInput::Navigate(Page::Favorites))
                                },
                            },
                        }
                    },

                    // 右侧主内容区域 - 可以扩展
                    #[wrap(Some)]
                    set_end_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,

                        // 标题栏
                        adw::HeaderBar {
                            set_show_start_title_buttons: true,
                            set_show_end_title_buttons: true,
                        },

                        // 页面切换的 Stack
                        #[name(main_stack)]
                        gtk::Stack {
                            set_vexpand: true,
                            set_hexpand: true,
                            #[watch]
                            set_visible_child_name: model.current_page.stack_name(),
                            set_transition_type: gtk::StackTransitionType::Crossfade,
                        }
                    }
                },

                // 底部播放条
                adw::HeaderBar {
                    add_css_class: "flat",
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    set_title_widget: Some(&gtk::Label::new(Some("正在播放: 尚未选择歌曲"))),
                }
            }
        }
    }

    fn init(_init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let discover_controller = DiscoverModel::builder()
            .launch(())
            .forward(sender.input_sender(), |output| {
                match output {
                    DiscoverOutput::PlaylistSelected(id) => {
                        // 暂时只打印，后续可以实现打开歌单详情
                        println!("选择了歌单: {}", id);
                        AppInput::Navigate(Page::Discover)
                    }
                }
            });

        // 创建导航图标
        let discover_icon = create_page_icon(Page::Discover);
        let explore_icon = create_page_icon(Page::Explore);
        let library_icon = create_page_icon(Page::Library);
        let favorites_icon = create_page_icon(Page::Favorites);

        // 先创建 model，初始页面设为我为推荐
        let model = AppModel {
            current_page: Page::Discover,
            discover_controller,
            discover_icon,
            explore_icon,
            library_icon,
            favorites_icon,
        };

        let widgets = view_output!();

        // 手动添加页面到 Stack
        widgets.main_stack.add_named(
            model.discover_controller.widget(),
            Some(Page::Discover.stack_name())
        );

        widgets.main_stack.add_named(
            &gtk::Label::builder()
                .label(Page::Explore.content_label())
                .css_classes(["dim-label"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build(),
            Some(Page::Explore.stack_name())
        );

        widgets.main_stack.add_named(
            &gtk::Label::builder()
                .label(Page::Library.content_label())
                .css_classes(["dim-label"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build(),
            Some(Page::Library.stack_name())
        );

        widgets.main_stack.add_named(
            &gtk::Label::builder()
                .label(Page::Favorites.content_label())
                .css_classes(["dim-label"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build(),
            Some(Page::Favorites.stack_name())
        );

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::Navigate(page) => {
                self.current_page = page;
            }
        }
    }
}
