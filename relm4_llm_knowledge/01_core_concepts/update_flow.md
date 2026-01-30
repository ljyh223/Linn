# Update 流程

## 核心更新循环

Relm4 的更新流程：

```
用户交互 → 发送消息 → update() → model 改变 → update_view() → UI 更新
    ↑                                                           ↓
    └───────────────────────────────────────────────────────────┘
```

## update() 函数

处理消息，更新 model：

```rust
fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
    match msg {
        AppMsg::Increment => {
            self.model.counter += 1;
        }
        AppMsg::Decrement => {
            self.model.counter -= 1;
        }
        AppMsg::Reset => {
            self.model.counter = 0;
        }
    }
}
```

## update_view() 函数

根据 model 更新 UI：

```rust
fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
    widgets.counter_label.set_label(&self.counter.to_string());
    widgets.reset_button.set_sensitive(self.counter != 0);
}
```

## 使用宏自动更新

使用 `#[watch]` 属性，自动在 `update_view` 中更新：

```rust
view! {
    gtk::Window {
        gtk::Label {
            #[watch]
            set_label: &format!("Count: {}", self.model.counter),
        }
    }
}
```

等价于手动实现：

```rust
fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
    widgets.counter_label.set_label(&format!("Count: {}", self.model.counter));
}
```

## 条件更新

使用 `#[track]` 配合 tracker：

```rust
view! {
    gtk::Window {
        #[track = "model.changed(AppModel::counter)"]
        set_title: &format!("Counter: {}", model.counter),
    }
}
```

只有当 `counter` 改变时才调用 `set_title`。

## 消息传递

### 发送消息给自己

```rust
fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
    match msg {
        AppMsg::Fetch => {
            // 完成后发送另一个消息
            sender.input(AppMsg::ProcessData);
        }
        AppMsg::ProcessData => {
            // 处理数据
        }
    }
}
```

### 发送消息给父组件

```rust
sender.output(ParentMsg::ChildValueChanged(self.value));
```

### 发送消息给子组件

```rust
impl Component for Parent {
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.child_controller.input(ChildMsg::Update(42));
    }
}
```

## 后台任务（仅 Component trait）

```rust
impl Component for App {
    type CommandOutput = AppMsg;

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::StartFetch => {
                // 启动后台任务
                sender.oneshot_command(|sender, _shutdown| {
                    async move {
                        let result = fetch_data().await;
                        sender.input(AppMsg::FetchComplete(result));
                    }
                });
            }
            _ => {}
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::FetchComplete(data) => {
                self.model.data = data;
            }
            _ => {}
        }
    }
}
```

## 完整流程示例

```rust
#[relm4::component]
impl SimpleComponent for App {
    view! {
        gtk::Window {
            gtk::Box {
                gtk::Button {
                    set_label: "+",
                    connect_clicked => AppMsg::Increment,
                },
                gtk::Label {
                    #[watch]
                    set_label: &self.counter.to_string(),
                },
                gtk::Button {
                    set_label: "-",
                    connect_clicked => AppMsg::Decrement,
                },
            }
        }
    }

    fn update(&mut self, msg: AppMsg, _: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => self.counter += 1,
            AppMsg::Decrement => self.counter -= 1,
        }
        // update_view 自动调用，更新 UI
    }
}
```

## 异步组件的 Update

异步组件的 `update` 是 async 的：

```rust
impl AsyncComponent for App {
    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
    ) {
        match msg {
            AppMsg::Load => {
                let data = load_from_disk().await;
                self.data = data;
            }
        }
    }
}
```

**注意**：async update 会阻塞该组件的消息处理，但不影响其他组件。
