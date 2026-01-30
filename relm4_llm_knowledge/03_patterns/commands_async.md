# Commands & Async 模式

## Commands 概述

Commands 允许在后台运行异步任务，不阻塞 UI。

## 使用场景

| 场景 | 方案 |
|------|------|
| 异步初始化 | `AsyncComponent` |
| 用户操作触发异步任务 | Commands |
| 并发执行多个任务 | Commands |
| CPU 密集型任务 | Worker |

## 基本命令

### oneshot_command - 一次性任务

```rust
impl Component for App {
    type CommandOutput = AppMsg;

    fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::FetchData => {
                let url = "https://api.example.com/data".to_string();

                sender.oneshot_command(|sender, _shutdown| {
                    async move {
                        // 异步获取数据
                        let data = fetch_from_url(&url).await;

                        // 返回结果
                        sender.input(AppMsg::DataFetched(data));
                    }
                });
            }
            AppMsg::DataFetched(data) => {
                self.model.data = data;
            }
            _ => {}
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>) {
        // 处理命令输出（如果 CommandOutput != Input）
    }
}
```

### command - 多次发送消息

```rust
sender.command(|sender, _shutdown| {
    async move {
        // 可以发送多个消息
        sender.input(AppMsg::Progress(0));
        tokio::time::sleep(Duration::from_secs(1)).await;
        sender.input(AppMsg::Progress(50));
        tokio::time::sleep(Duration::from_secs(1)).await;
        sender.input(AppMsg::Progress(100));
        sender.input(AppMsg::Complete);
    }
});
```

### 同步命令

```rust
sender.spawn_oneshot_command(|sender, _shutdown| {
    // 同步函数
    let result = expensive_computation();

    sender.input(AppMsg::ComputationComplete(result));
});
```

## 完整示例

```rust
use relm4::prelude::*;

#[derive(Debug)]
pub enum AppMsg {
    Fetch,
    DataFetched(String),
    Error(String),
}

pub struct App {
    data: String,
    loading: bool,
}

impl Component for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Root = gtk::Window;
    type CommandOutput = AppMsg;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = App {
            data: "Not loaded".to_string(),
            loading: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Commands Example"),
            set_default_width: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    #[watch]
                    set_label: &format!("Data: {}", model.data),
                },

                gtk::Spinner {
                    #[watch]
                    set_spinning: model.loading,
                },

                gtk::Button {
                    set_label: "Fetch",
                    connect_clicked => AppMsg::Fetch,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Fetch => {
                self.loading = true;

                // 启动异步任务
                sender.oneshot_command(|sender, _shutdown| {
                    async move {
                        // 模拟网络请求
                        tokio::time::sleep(Duration::from_secs(2)).await;

                        sender.input(AppMsg::DataFetched("Hello from async!".to_string()));
                    }
                });
            }
            AppMsg::DataFetched(data) => {
                self.data = data;
                self.loading = false;
            }
            AppMsg::Error(err) => {
                self.data = format!("Error: {}", err);
                self.loading = false;
            }
        }
    }
}
```

## 进度报告

```rust
sender.command(|sender, _shutdown| {
    async move {
        for i in 0..=100 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            sender.input(AppMsg::Progress(i));
        }
        sender.input(AppMsg::Complete);
    }
});
```

## 并发执行

Commands 自动在 Tokio runtime 上并发执行：

```rust
fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
    match msg {
        AppMsg::FetchAll => {
            // 多个命令并发执行
            sender.oneshot_command(|s, _| {
                async move {
                    let data1 = fetch1().await;
                    s.input(AppMsg::Data1(data1));
                }
            });

            sender.oneshot_command(|s, _| {
                async move {
                    let data2 = fetch2().await;
                    s.input(AppMsg::Data2(data2));
                }
            });
        }
        _ => {}
    }
}
```

## 错误处理

```rust
sender.oneshot_command(|sender, _shutdown| {
    async move {
        match fetch_data().await {
            Ok(data) => {
                sender.input(AppMsg::Success(data));
            }
            Err(e) => {
                sender.input(AppMsg::Error(e.to_string()));
            }
        }
    }
});
```

## 运行时配置

```rust
fn main() {
    // 设置线程数（默认 1）
    relm4::set_relm_threads(4);

    // 设置异步栈大小（默认 2MB）
    relm4::set_relm_async_stack_size(1024 * 1024); // 1MB

    let app = RelmApp::new("com.example.app");
    app.run::<App>(());
}
```

## Commands vs AsyncComponent

| 特性 | Commands | AsyncComponent |
|------|----------|----------------|
| 异步 update | ❌ | ✅ |
| 并发任务 | ✅ | ❌ |
| 适用场景 | 后台任务 | 整体异步 |

## 完整示例：文件加载

```rust
use relm4::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub enum AppMsg {
    OpenFile,
    FileSelected(PathBuf),
    FileLoaded(String),
    LoadError(String),
}

pub struct App {
    content: String,
    loading: bool,
}

impl Component for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Root = gtk::Window;
    type CommandOutput = AppMsg;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = App {
            content: String::new(),
            loading: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("File Loader"),
            set_default_width: 600,
            set_default_height: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::Button {
                    set_label: "Open File",
                    connect_clicked => AppMsg::OpenFile,
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,

                    #[name = "text_view"]
                    gtk::TextView {
                        set_editable: false,
                        set_wrap_mode: gtk::WrapMode::Word,

                        #[watch]
                        set_sensitive: !model.loading,
                    }
                },

                gtk::Label {
                    #[watch]
                    set_label: if model.loading { "Loading..." } else { "" },
                    #[watch]
                    set_visible: model.loading,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::OpenFile => {
                // 使用文件选择器
                let dialog = gtk::FileChooserDialog::new(
                    Some("Open File"),
                    Some(&widgets.root.clone()),
                    gtk::FileChooserAction::Open,
                    &[("Open", gtk::ResponseType::Accept), ("Cancel", gtk::ResponseType::Cancel)]
                );

                dialog.connect_response(
                    sender.clone(),
                    |dialog, response, sender| {
                        if response == gtk::ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(path) = file.path() {
                                    sender.input(AppMsg::FileSelected(path));
                                }
                            }
                        }
                        dialog.close();
                    }
                );

                dialog.show();
            }
            AppMsg::FileSelected(path) => {
                self.loading = true;

                let path_clone = path.clone();
                sender.oneshot_command(move |sender, _shutdown| {
                    async move {
                        match tokio::fs::read_to_string(path_clone).await {
                            Ok(content) => {
                                sender.input(AppMsg::FileLoaded(content));
                            }
                            Err(e) => {
                                sender.input(AppMsg::LoadError(e.to_string()));
                            }
                        }
                    }
                });
            }
            AppMsg::FileLoaded(content) => {
                self.content = content;
                self.loading = false;

                // 更新 TextView
                if let Some(buffer) = widgets.text_view.buffer() {
                    buffer.set_text(&self.content);
                }
            }
            AppMsg::LoadError(err) => {
                self.content = format!("Error: {}", err);
                self.loading = false;

                if let Some(buffer) = widgets.text_view.buffer() {
                    buffer.set_text(&self.content);
                }
            }
        }
    }
}
```

## 最佳实践

1. **使用 oneshot_command**：大多数情况下够用
2. **错误处理**：始终处理可能的错误
3. **避免阻塞**：异步操作不应使用 `await` 阻塞太久
4. **状态管理**：使用 loading 状态提供反馈
5. **资源清理**：使用 shutdown 信号清理资源
