use relm4::gtk::prelude::*;
use relm4::gtk::{self, Adjustment, Orientation};

/// 可复用的横向滚动行组件
///
/// 包含：标题 + 左右箭头按钮 + 横向滚动窗口
pub struct ScrollableRow {
    pub container: gtk::Box,
    pub content_box: gtk::Box,
    pub adjustment: Adjustment,
}

impl ScrollableRow {
    /// 创建一个新的横向滚动行
    ///
    /// # Arguments
    /// * `title` - 标题文本
    /// * `min_height` - 滚动区域最小高度
    /// * `max_height` - 滚动区域最大高度
    pub fn new(title: &str, min_height: i32, max_height: i32) -> Self {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .build();

        // 标题栏：标题 + 左右箭头
        let header = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .margin_end(16)
            .build();

        let label = gtk::Label::builder()
            .label(title)
            .css_classes(["title-3"])
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build();

        let btn_left = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(["circular", "flat"])
            .tooltip_text("向左滚动")
            .build();

        let btn_right = gtk::Button::builder()
            .icon_name("go-next-symbolic")
            .css_classes(["circular", "flat"])
            .tooltip_text("向右滚动")
            .build();

        header.append(&label);
        header.append(&btn_left);
        header.append(&btn_right);

        // 滚动区域
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(min_height)
            .max_content_height(max_height)
            .hexpand(true)
            .build();

        let content_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .margin_start(4)
            .margin_end(4)
            .build();

        scrolled.set_child(Some(&content_box));

        container.append(&header);
        container.append(&scrolled);

        let adjustment = scrolled.hadjustment();

        // 绑定滚动按钮事件
        let adj = adjustment.clone();
        btn_left.connect_clicked(move |_| {
            let scroll_amount = 250.0;
            let new_value = (adj.value() - scroll_amount).max(adj.lower());
            adj.set_value(new_value);
        });

        let adj = adjustment.clone();
        btn_right.connect_clicked(move |_| {
            let scroll_amount = 250.0;
            let max_value = adj.upper() - adj.page_size();
            let new_value = (adj.value() + scroll_amount).min(max_value);
            adj.set_value(new_value);
        });

        Self {
            container,
            content_box,
            adjustment,
        }
    }

    /// 获取容器 widget，用于添加到父容器
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    /// 获取内容容器，用于添加子项
    pub fn content(&self) -> &gtk::Box {
        &self.content_box
    }

    /// 获取滚动调整器，用于自定义滚动行为
    pub fn hadjustment(&self) -> &Adjustment {
        &self.adjustment
    }
}
