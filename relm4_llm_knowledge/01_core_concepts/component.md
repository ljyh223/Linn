# Component 组件

## 核心概念

Component 是 Relm4 的基础构建块，所有 UI 元素都通过 Component 实现。

## 两个核心 Trait

### `SimpleComponent` - 简化版

适用于大多数场景，移除了高级特性：

```rust
impl SimpleComponent for App {
    type Init = u32;
    type Input = AppMsg;
    type Output = ();

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        counter: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 初始化 model 和 widgets
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        // 处理消息，更新 model
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        // 根据 model 更新 UI
    }
}
```

### `Component` - 完整版

支持 Commands（后台任务）：

```rust
impl Component for App {
    // 额外需要
    type CommandOutput = AppMsg;

    // 其他方法同 SimpleComponent

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
    ) {
        // 处理后台任务返回的消息
    }
}
```

## 关键关联类型

| 类型 | 说明 |
|------|------|
| `Init` | 初始化参数类型 |
| `Input` | 接收的消息类型 |
| `Output` | 向父组件发送的消息类型（`()` 表示无） |
| `Root` | 根 widget 类型（主组件必须是 `gtk::Window`） |
| `Widgets` | 存储所有 widget 的结构体 |
| `CommandOutput` | 后台任务返回的消息类型（仅 Component） |

## 使用宏简化

```rust
#[relm4::component]
impl SimpleComponent for App {
    view! {
        gtk::Window {
            set_title: Some("My App"),
            #[watch]
            set_default_width: self.model.width,

            gtk::Button {
                connect_clicked => AppMsg::Increment,
            }
        }
    }
}
```

宏会自动生成 `Widgets` 结构体，无需手动定义。

## 运行组件

```rust
fn main() {
    let app = RelmApp::new("com.example.app");
    app.run::<App>(0);  // 0 是 Init 参数
}
```
