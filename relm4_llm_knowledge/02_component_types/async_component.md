# AsyncComponent 异步组件

## 用途

允许在 `init`、`update`、`update_cmd` 中使用 `await`。

## 基本用法

```rust
#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 可以 await
        let data = load_data().await;

        let model = AppModel { data };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
    ) {
        // 可以 await
        match msg {
            AppMsg::Load => {
                self.data = fetch_data().await;
            }
        }
    }
}
```

## Loading Widgets

当 `init` 耗时较长时，显示加载界面：

```rust
fn init_loading_widgets(root: &Self::Root) -> Option<LoadingWidgets> {
    Some(view! {
        gtk::Window {
            gtk::Spinner {
                set_spinning: true,
            }
        }
    })
}

async fn init(...) {
    // 执行耗时操作
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 返回后，spinner 被真实 widgets 替换
}
```

## 完整示例

```rust
#[derive(Debug)]
pub enum AppMsg {
    Load,
}

pub struct App {
    data: String,
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    // 初始化时显示 loading
    fn init_loading_widgets(root: &Self::Root) -> Option<LoadingWidgets> {
        view! {
            gtk::Window {
                set_title: Some("Loading..."),
                set_default_width: 300,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_margin_all: 10,

                    gtk::Spinner {
                        set_spinning: true,
                        set_halign: gtk::Align::Center,
                    },

                    gtk::Label {
                        set_label: "Loading data...",
                        set_halign: gtk::Align::Center,
                    }
                }
            }
        }
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 模拟异步加载
        tokio::time::sleep(Duration::from_secs(2)).await;

        let model = App {
            data: "Loaded data".to_string(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Async App"),
            set_default_width: 300,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    #[watch]
                    set_label: &format!("Data: {}", self.data),
                },

                gtk::Button {
                    set_label: "Reload",
                    connect_clicked => AppMsg::Load,
                }
            }
        }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            AppMsg::Load => {
                // 异步更新
                tokio::time::sleep(Duration::from_secs(1)).await;
                self.data = format!("Reloaded at {}", Local::now().format("%H:%M:%S"));
            }
        }
    }
}
```

## Async Factory

```rust
#[relm4::factory(async)]
impl AsyncFactoryComponent for Item {
    type Init = u32;
    type Input = ItemMsg;
    type Output = ItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    async fn init_model(
        value: Self::Init,
        index: &DynamicIndex,
        sender: AsyncFactorySender<Self>,
    ) -> Self {
        // 可以 await
        let data = fetch_item_data(value).await;
        Self { value: data, tracker: 0 }
    }

    // ...
}
```

## 异步更新注意

- **阻塞组件**：async update 会阻塞该组件的消息处理
- **不影响其他组件**：其他组件继续正常工作
- **并发处理**：如需并发，使用 Commands

## 何时使用

| 需求 | 方案 |
|------|------|
| 初始化需要异步加载 | `AsyncComponent` + `init_loading_widgets` |
| 用户操作触发异步任务 | `update` 中 await |
| 并发处理多个任务 | 使用 `Component` + Commands |
| 简单 UI | `SimpleComponent` |
