# Component 类型

## SimpleComponent vs Component

| 特性 | SimpleComponent | Component |
|------|-----------------|-----------|
| 后台任务 | ❌ | ✅ |
| 代码量 | 少 | 多 |
| 适用场景 | 简单 UI | 需要 async 任务 |

## SimpleComponent

```rust
impl SimpleComponent for App {
    type Init = u32;
    type Input = AppMsg;
    type Output = ();

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        value: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel { counter: value };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        // 更新 model
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        // 更新 UI（使用 #[watch] 可省略）
    }
}
```

## Component（支持 Commands）

```rust
impl Component for App {
    type Init = u32;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = CmdMsg;  // 新增：命令输出类型

    fn init(...) {
        // 同 SimpleComponent
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::FetchData => {
                // 启动后台任务
                sender.oneshot_command(|sender, _shutdown| {
                    async move {
                        let data = fetch().await;
                        sender.input(AppMsg::DataFetched(data));
                    }
                });
            }
            _ => {}
        }
    }

    // 新增：处理命令返回的消息
    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>) {
        match msg {
            CmdMsg::DataFetched(data) => {
                self.model.data = data;
            }
        }
    }
}
```

## 启动方式

```rust
fn main() {
    let app = RelmApp::new("com.example.app");
    app.run::<App>(0);  // 0 是 Init 参数
}
```

## 完整示例对比

### SimpleComponent

```rust
#[relm4::component]
impl SimpleComponent for Counter {
    view! {
        gtk::Window {
            gtk::Label {
                #[watch]
                set_label: &self.count.to_string(),
            }
        }
    }

    fn update(&mut self, msg: CounterMsg, _: ComponentSender<Self>) {
        match msg {
            CounterMsg::Increment => self.count += 1,
        }
    }
}
```

### Component（带后台任务）

```rust
#[relm4::component]
impl Component for App {
    view! {
        gtk::Window {
            gtk::Label {
                #[watch]
                set_label: &self.data.to_string(),
            }
        }
    }

    fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Load => {
                sender.oneshot_command(|s, _| {
                    async move {
                        let data = load_data().await;
                        s.input(AppMsg::Loaded(data));
                    }
                });
            }
            AppMsg::Loaded(data) => {
                self.data = data;
            }
        }
    }

    fn update_cmd(&mut self, msg: CmdMsg, _: ComponentSender<Self>) {
        // 处理其他命令输出
    }
}
```

## 使用建议

- 优先使用 `SimpleComponent`
- 仅当需要后台任务时使用 `Component`
- 使用宏简化代码
