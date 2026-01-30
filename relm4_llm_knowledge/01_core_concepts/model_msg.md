# Model 和 Message

## Model（模型）

Model 是应用的"大脑"，存储应用状态。

### 定义 Model

```rust
struct AppModel {
    counter: u8,
    label_text: String,
    is_enabled: bool,
}
```

### Model 原则

1. **仅存储数据**：不包含 widget 引用
2. **可简单复制**：优先使用基本类型或 `Copy` 类型
3. **派生必要 trait**：
   ```rust
   #[derive(Clone, Debug)]
   struct AppModel { ... }
   ```

## Message（消息）

Message 描述如何改变 Model，通常使用 `enum`。

### 基本消息类型

```rust
pub enum AppMsg {
    Increment,
    Decrement,
    SetValue(u32),
    Reset,
}
```

### 输入输出消息

```rust
// 输入：组件接收的消息
pub enum AppMsg {
    UserInput(String),
    Submit,
}

// 输出：向父组件发送的消息
pub enum AppOutput {
    DataChanged(String),
    Cancelled,
}
```

### 消息设计模式

```rust
pub enum AppMsg {
    // 用户操作
    Increment,
    Decrement,

    // 数据更新
    SetValue(u32),

    // 内部消息（不显示在文档）
    #[doc(hidden)]
    InternalUpdate(u32),

    // 带数据的消息
    FetchData(String),

    // 带多个数据的消息（元组）
    UpdatePosition((i32, i32)),
}
```

## Update 函数

处理消息并更新 Model：

```rust
fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
    match msg {
        AppMsg::Increment => {
            self.counter = self.counter.wrapping_add(1);
        }
        AppMsg::Decrement => {
            self.counter = self.counter.wrapping_sub(1);
        }
        AppMsg::SetValue(val) => {
            self.counter = val;
        }
        AppMsg::FetchData(url) => {
            // 启动后台任务
            sender.command(|sender, _shutdown| {
                async move {
                    let data = fetch_from_url(url).await;
                    sender.input(AppMsg::DataFetched(data));
                }
            });
        }
    }
}
```

## 消息发送

### 发送 Input 到自己

```rust
sender.input(AppMsg::Increment);
```

### 发送 Output 到父组件

```rust
sender.output(AppOutput::DataChanged("new value".to_string()));
```

## 完整示例

```rust
struct AppModel {
    counter: u8,
}

#[derive(Debug)]
pub enum AppMsg {
    Increment,
    Decrement,
}

impl SimpleComponent for App {
    type Init = u8;
    type Input = AppMsg;
    type Output = ();

    fn init(counter: Self::Init, ...) -> ComponentParts<Self> {
        AppModel { counter }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _: ComponentSender<Self>) {
        widgets.label.set_label(&self.counter.to_string());
    }
}
```
