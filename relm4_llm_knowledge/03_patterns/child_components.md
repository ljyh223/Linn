# Child Components 子组件模式

## 概述

将 UI 拆分为可复用的子组件，提高代码组织和复用性。

## 基本模式

### 1. 定义子组件

```rust
use relm4::prelude::*;

// 子组件消息
pub enum DialogMsg {
    Show,
    Hide,
    Response(gtk::ResponseType),
}

// 子组件输出
pub enum DialogOutput {
    Accepted,
    Cancelled,
}

// 子组件配置
#[derive(Debug, Clone)]
pub struct DialogSettings {
    pub title: String,
    pub message: String,
}

// 子组件模型
pub struct Dialog {
    is_visible: bool,
    settings: DialogSettings,
}

// 子组件实现
#[relm4::component]
impl SimpleComponent for Dialog {
    type Init = DialogSettings;
    type Input = DialogMsg;
    type Output = DialogOutput;
    type Root = gtk::Dialog;

    fn init_root() -> Self::Root {
        gtk::Dialog::builder()
            .use_header_bar(1)
            .build()
    }

    fn init(
        settings: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            is_visible: false,
            settings,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Dialog {
            #[watch]
            set_title: Some(&model.settings.title),
            #[watch]
            set_visible: model.is_visible,

            #[name = "content_area"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    #[watch]
                    set_label: &model.settings.message,
                }
            },

            // 添加操作按钮
            add_action = &gtk::Button {
                set_label: "OK",
                connect_clicked[sender] => move |_| {
                    sender.output(DialogOutput::Accepted);
                    sender.input(DialogMsg::Hide);
                }
            },

            add_action = &gtk::Button {
                set_label: "Cancel",
                connect_clicked[sender] => move |_| {
                    sender.output(DialogOutput::Cancelled);
                    sender.input(DialogMsg::Hide);
                }
            },
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogMsg::Show => self.is_visible = true,
            DialogMsg::Hide => self.is_visible = false,
            DialogMsg::Response(response) => {
                match response {
                    gtk::ResponseType::Accept => {
                        // 输出已处理
                    }
                    _ => {}
                }
                self.is_visible = false;
            }
        }
    }
}
```

### 2. 父组件使用子组件

```rust
use relm4::prelude::*;

#[derive(Debug)]
pub enum AppMsg {
    ShowDialog,
    DialogAccepted,
    DialogCancelled,
}

pub struct App {
    dialog: Controller<Dialog>,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 创建子组件
        let dialog = Dialog::builder()
            .transient_for(root)  // 设置父窗口
            .launch(DialogSettings {
                title: "Confirm".to_string(),
                message: "Are you sure?".to_string(),
            })
            .forward(sender.input_sender(), |msg| match msg {
                DialogOutput::Accepted => AppMsg::DialogAccepted,
                DialogOutput::Cancelled => AppMsg::DialogCancelled,
            });

        let model = App { dialog };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Child Components Example"),
            set_default_width: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    set_label: "Main Window",
                },

                gtk::Button {
                    set_label: "Show Dialog",
                    connect_clicked => AppMsg::ShowDialog,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::ShowDialog => {
                self.dialog.emit(DialogMsg::Show);
            }
            AppMsg::DialogAccepted => {
                println!("Dialog accepted!");
            }
            AppMsg::DialogCancelled => {
                println!("Dialog cancelled!");
            }
        }
    }
}
```

## 通信模式

### 父 → 子：发送消息

```rust
// 方法 1: 通过 emit
self.child.emit(ChildMsg::Update(value));

// 方法 2: 通过 input
self.child.input(ChildMsg::Update(value));
```

### 子 → 父：输出消息

```rust
// 子组件定义输出
pub enum ChildOutput {
    ValueChanged(i32),
}

// 创建时 forward
let child = Child::builder()
    .launch(initial_value)
    .forward(sender.input_sender(), |msg| match msg {
        ChildOutput::ValueChanged(v) => AppMsg::ChildValueChanged(v),
    });

// 父组件处理
fn update(&mut self, msg: AppMsg, _: ComponentSender<Self>) {
    match msg {
        AppMsg::ChildValueChanged(v) => {
            self.value = v;
        }
    }
}
```

## 复用性增强

### 可配置子组件

```rust
#[derive(Debug, Clone)]
pub struct CardConfig {
    pub title: String,
    pub show_close: bool,
    pub icon: Option<String>,
}

impl Component for Card {
    type Init = CardConfig;

    fn init(config: Self::Init, ...) -> ComponentParts<Self> {
        // 根据配置初始化
    }
}

// 使用不同配置
let card1 = Card::builder()
    .launch(CardConfig {
        title: "Card 1".to_string(),
        show_close: true,
        icon: Some("document".to_string()),
    });

let card2 = Card::builder()
    .launch(CardConfig {
        title: "Card 2".to_string(),
        show_close: false,
        icon: None,
    });
```

### 内部消息隐藏

```rust
pub enum ChildMsg {
    PublicMessage,
    #[doc(hidden)]  // 不在文档中显示
    InternalUpdate(u32),
}
```

## 多个子组件

```rust
pub struct App {
    header: Controller<Header>,
    sidebar: Controller<Sidebar>,
    content: Controller<Content>,
    dialog: Controller<Dialog>,
}

fn init(...) {
    let header = Header::builder().launch(());
    let sidebar = Sidebar::builder().launch(());
    let content = Content::builder().launch(());
    let dialog = Dialog::builder()
        .transient_for(root)
        .launch(());

    App { header, sidebar, content, dialog }
}
```

## 动态子组件列表

```rust
pub struct App {
    tabs: FactoryVecDeque<Tab>,
}

// Tab 是 FactoryComponent
// 可以动态添加、删除、移动标签页
```

## 完整示例：Alert 对话框

```rust
use relm4::prelude::*;
use gtk::prelude::*;

#[derive(Debug)]
pub enum AlertMsg {
    Show,
    #[doc(hidden)]
    Response(gtk::ResponseType),
}

pub enum AlertOutput {
    Ok,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub title: String,
    pub message: String,
}

pub struct Alert {
    is_visible: bool,
    config: AlertConfig,
}

#[relm4::component]
impl SimpleComponent for Alert {
    type Init = AlertConfig;
    type Input = AlertMsg;
    type Output = AlertOutput;
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .resizable(false)
            .modal(true)
            .build()
    }

    fn init(
        config: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Alert {
            is_visible: false,
            config,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            #[watch]
            set_title: Some(&model.config.title),
            #[watch]
            set_visible: model.is_visible,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 20,

                gtk::Label {
                    #[watch]
                    set_label: &model.config.message,
                },

                gtk::Box {
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "OK",
                        connect_clicked[sender] => move |_| {
                            sender.output(AlertOutput::Ok);
                            sender.input(AlertMsg::Show);
                        }
                    },

                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked[sender] => move |_| {
                            sender.output(AlertOutput::Cancel);
                            sender.input(AlertMsg::Show);
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AlertMsg::Show => {
                self.is_visible = !self.is_visible;
            }
            AlertMsg::Response(_) => {}
        }
    }
}
```
