use relm4::gtk::prelude::*;
use relm4::gtk::{self, Adjustment, Orientation};
use relm4::prelude::*;

/// 可复用的横向滚动行组件
///
/// 包含：标题 + 左右箭头按钮 + 横向滚动窗口
#[derive(Debug)]
pub struct ScrollableRowInit {
    pub title: String,
    pub min_height: i32,
    pub max_height: i32,
}

impl ScrollableRowInit {
    pub fn new(title: impl Into<String>, min_height: i32, max_height: i32) -> Self {
        Self {
            title: title.into(),
            min_height,
            max_height,
        }
    }
}

#[derive(Debug)]
pub enum ScrollableRowInput {
    ScrollLeft,
    ScrollRight,
}

pub struct ScrollableRow {
    title: String,
    min_height: i32,
    max_height: i32,
    adjustment: Adjustment,
    content: gtk::Box,
}

impl ScrollableRow {
    /// 挂载工厂卡片等内容到滚动区域内部
    pub fn content_box(&self) -> gtk::Box {
        self.content.clone()
    }

    /// 创建并启动一个新的横向滚动行，返回控制器
    ///
    /// # Arguments
    /// * `title` - 标题文本
    /// * `min_height` - 滚动区域最小高度
    /// * `max_height` - 滚动区域最大高度
    pub fn new(title: impl Into<String>, min_height: i32, max_height: i32) -> Controller<Self> {
        Self::builder()
            .launch(ScrollableRowInit::new(title, min_height, max_height))
            .detach()
    }
}

#[relm4::component(pub)]
impl Component for ScrollableRow {
    type Init = ScrollableRowInit;
    type Input = ScrollableRowInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: Orientation::Vertical,
            set_spacing: 8,

            // 标题栏：标题 + 左右箭头
            gtk::Box {
                set_orientation: Orientation::Horizontal,
                set_margin_end: 16,

                gtk::Label {
                    set_label: &model.title,
                    add_css_class: "title-3",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },

                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    set_tooltip_text: Some("向左滚动"),
                    set_halign: gtk::Align::End,
                    add_css_class: "circular",
                    add_css_class: "flat",
                    connect_clicked[sender] => move |_| {
                        let _ = sender.input(ScrollableRowInput::ScrollLeft);
                    },
                },

                gtk::Button {
                    set_icon_name: "go-next-symbolic",
                    set_tooltip_text: Some("向右滚动"),
                    set_halign: gtk::Align::End,
                    add_css_class: "circular",
                    add_css_class: "flat",
                    connect_clicked[sender] => move |_| {
                        let _ = sender.input(ScrollableRowInput::ScrollRight);
                    },
                },
            },

            // 滚动区域
            #[name(scrolled)]
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::External,
                set_vscrollbar_policy: gtk::PolicyType::Never,
                set_min_content_height: model.min_height,
                set_max_content_height: model.max_height,
                set_hexpand: true,

                #[name(content_box)]
                gtk::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 16,
                    set_margin_start: 4,
                    set_margin_end: 4,
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            title: init.title,
            min_height: init.min_height,
            max_height: init.max_height,
            adjustment: gtk::Adjustment::default(),
            content: gtk::Box::default(),
        };

        let widgets = view_output!();

        model.adjustment = widgets.scrolled.hadjustment();
        model.content = widgets.content_box.clone();

        let _ = (root, sender);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        const SCROLL_AMOUNT: f64 = 250.0;

        match message {
            ScrollableRowInput::ScrollLeft => {
                let new_value =
                    (self.adjustment.value() - SCROLL_AMOUNT).max(self.adjustment.lower());
                self.adjustment.set_value(new_value);
            }
            ScrollableRowInput::ScrollRight => {
                let new_value = (self.adjustment.value() + SCROLL_AMOUNT)
                    .min(self.adjustment.upper() - self.adjustment.page_size());
                self.adjustment.set_value(new_value);
            }
        }
    }
}
