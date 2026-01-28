use relm4::prelude::*;
use adw::prelude::AdwApplicationWindowExt;
use relm4::gtk::prelude::*;
use crate::pages::{self, Page};
use crate::ui::create_sidebar;

#[derive(Debug)]
pub enum AppInput {
    Navigate(Page),
    PlaySong(String),
    LikeSong(String),
}

pub struct AppModel {
    current_page: Page,
}

#[allow(dead_code)]
pub struct AppWidgets {
    page_title: gtk::Label,
    stack: gtk::Stack,
    discover_page: gtk::Label,
    explore_page: gtk::Label,
    library_page: gtk::Label,
    favorites_page: gtk::Label,
}

impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type Root = adw::ApplicationWindow;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder()
            .title("Linn - 网易云音乐")
            .default_width(1200)
            .default_height(800)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {
            current_page: Page::Discover,
        };

        // 创建左侧导航栏
        let sidebar = create_sidebar(&sender);

        // 创建右侧内容区
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        // 标题栏
        let header_bar = adw::HeaderBar::builder()
            .css_classes(["flat"])
            .build();

        let page_title = gtk::Label::builder()
            .label(Page::Discover.title())
            .css_classes(["title"])
            .build();
        header_bar.set_title_widget(Some(&page_title));
        content_box.append(&header_bar);

        // 内容页面栈
        let stack = gtk::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        // 创建各个页面
        let discover_page = pages::create_page_label(Page::Discover);
        stack.add_named(&discover_page, Some(Page::Discover.stack_name()));

        let explore_page = pages::create_page_label(Page::Explore);
        stack.add_named(&explore_page, Some(Page::Explore.stack_name()));

        let library_page = pages::create_page_label(Page::Library);
        stack.add_named(&library_page, Some(Page::Library.stack_name()));

        let favorites_page = pages::create_page_label(Page::Favorites);
        stack.add_named(&favorites_page, Some(Page::Favorites.stack_name()));

        stack.set_visible_child_name(Page::Discover.stack_name());
        content_box.append(&stack);

        // 组装窗口
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        main_box.append(&sidebar);
        main_box.append(&content_box);

        root.set_content(Some(&main_box));

        let widgets = AppWidgets {
            page_title,
            stack,
            discover_page,
            explore_page,
            library_page,
            favorites_page,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::Navigate(page) => {
                self.current_page = page;
            }
            AppInput::PlaySong(_) => {
                // TODO: 实现播放功能
            }
            AppInput::LikeSong(_) => {
                // TODO: 实现喜欢功能
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        // 更新标题
        widgets.page_title.set_label(self.current_page.title());

        // 切换页面
        widgets.stack.set_visible_child_name(self.current_page.stack_name());
    }
}
