# linn 开发文档

## 技术栈

- Rust + GTK4 (gtk4-rs 0.11) + libadwaita (adw 0.7) + relm4 0.11
- 参考项目: Tonearm (Go+GTK4), ratic (Rust+relm4)

## 组件架构

```
Window (ApplicationWindow)
  └── Header (OverlaySplitView)          ← 根布局，拆分侧边栏与内容
        ├── Sidebar (ToolbarView)         ← 侧边栏：播放器/歌词/队列
        └── Content (ToolbarView)         ← 内容区：首页/探索/收藏
```

每个子组件是一个独立的 `SimpleComponent`，有自己的 `view!`、`init()`、`update()`。

## 如何创建自定义组件

### 1. 基本模板

```rust
use log::trace;
use relm4::gtk::prelude::*;           // GTK trait 方法
use relm4::prelude::*;                // SimpleComponent, ComponentSender 等
use relm4::{adw, ComponentParts, gtk};

pub struct MyComponent {
    // 存放需要在 update() 中访问的 widget 引用
    // 以及业务状态
}

#[derive(Debug)]
pub enum MyMsg {
    // 输入消息 — 用户交互触发
}

#[derive(Debug)]
pub enum MyOutput {
    // 输出消息 — 通知父组件（没有可设为 ()）
}

#[relm4::component(pub)]
impl SimpleComponent for MyComponent {
    type Init = ();                    // 初始化参数
    type Input = MyMsg;               // 输入消息
    type Output = MyOutput;           // 输出消息

    view! {
        #[root]
        adw::ToolbarView {            // 根 widget
            // 声明式定义 widget 树
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self { /* ... */ };
        let mut widgets = view_output!();
        // 动态添加子 widget
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        trace!("MyComponent: {message:?}");
        // 处理消息，更新 model 和 widget
    }
}
```

### 2. 在父组件中使用子组件

```rust
// header.rs 中使用 Content 子组件
pub struct Header {
    content: Controller<Content>,      // 存储子组件的 Controller
}

fn init(..., sender: ComponentSender<Self>) -> ComponentParts<Self> {
    // 方式 A: 不需要接收输出
    let sidebar = Sidebar::builder().launch(()).detach();

    // 方式 B: 需要转发输出消息
    let content = Content::builder().launch(()).forward(
        sender.input_sender(),
        |output| match output {
            ContentOutput::ToggleSidebar => HeaderMsg::ToggleSidebar,
        },
    );

    let model = Self { sidebar, content, /* ... */ };
    let widgets = view_output!();
    ComponentParts { model, widgets }
}
```

在 `view!` 中引用子组件的 widget：

```rust
view! {
    #[root]
    adw::OverlaySplitView {
        set_sidebar: Some(model.sidebar.widget()),   // 子组件的根 widget
        #[wrap(Some)]
        set_content = model.content.widget(),
    }
}
```

## 关键设计模式

### `#[name]` 属性 — 在 init() 中访问 view! 中的 widget

`view!` 宏在 `init()` 返回前执行（`view_output!()` 调用时），但有时需要在 `init()` 中向
widget 动态添加子元素（比如循环创建按钮）。用 `#[name = "xxx"]` 或 `#[name(xxx)]` 标记
widget，之后通过 `widgets.xxx` 访问：

```rust
view! {
    #[root]
    adw::ToolbarView {
        #[name(stack)]                    // 命名这个 ViewStack
        #[wrap(Some)]
        set_content = &adw::ViewStack {},

        #[name(footer)]                   // 命名这个 Box
        add_bottom_bar = &gtk::Box { ... },
    }
}

fn init(...) -> ComponentParts<Self> {
    let mut model = Self { /* ... */ };
    let mut widgets = view_output!();     // 执行 view!，生成命名 widget

    // 现在可以访问 widgets.stack 和 widgets.footer
    widgets.stack.add_titled(&page, Some("home"), "Home");
    widgets.footer.append(&btn);

    ComponentParts { model, widgets }
}
```

### model 变量必须在 view_output!() 之前声明

`#[relm4::component]` 宏需要在 `view_output!()` 调用之前找到名为 `model` 的变量。
如果 model 的字段依赖于 view! 中的 widget（比如按钮列表），可以先用占位值初始化，
在 `view_output!()` 之后再填充：

```rust
let mut model = Self {
    stack: adw::ViewStack::default(),   // 占位
    buttons: Vec::new(),
};
let mut widgets = view_output!();

model.stack = widgets.stack.clone();     // 用真实 widget 替换占位值
// 循环创建按钮并 push 到 model.buttons
```

### 按钮选中状态 — CSS class 切换

参考 Tonearm 的做法，使用 CSS class `raised`/`flat` 实现按钮的选中/未选中效果：

```rust
// 初始化时
if tag == "home" {
    btn.add_css_class("raised");        // 选中
} else {
    btn.add_css_class("flat");          // 未选中
}

// 切换时
for btn in &self.buttons {
    btn.remove_css_class("raised");
    btn.add_css_class("flat");
}
if let Some(btn) = self.buttons.get(idx) {
    btn.remove_css_class("flat");
    btn.add_css_class("raised");
}
```

### 组件间通信

子组件通过 `Output` 消息向父组件通信，父组件通过 `.forward()` 接收：

```
Content (子组件)                    Header (父组件)
    │                                  │
    ├─ Output::ToggleSidebar  ───────> ├─ Input::ToggleSidebar
    │                                  │
    └─ sender.output(...)             └─ forward(sender.input_sender(), |out| ...)
```

如果子组件需要接收父组件的消息，使用 `Controller::emit()` 发送 `Input` 消息。

## 踩坑记录

### 1. `view!` 宏的执行时机

`view!` 宏（通过 `view_output!()` 调用）在 `init()` 函数体中执行，但它生成的
widget 会覆盖 `init()` 参数 `root` 的内容。这意味着**不能**在 `init()` 中先往
`root` 添加子 widget 再调用 `view_output!()` — 那些子 widget 会被覆盖。

正确做法：在 `view!` 中声明所有静态 widget 结构，用 `#[name]` 暴露需要动态操作的
widget，在 `init()` 中通过 `widgets.xxx` 动态添加子元素。

### 2. `builder()` 方法找不到

`SimpleComponent::builder()` 需要 `SimpleComponent` trait 在作用域。必须导入：

```rust
use relm4::prelude::*;    // 推荐，导入所有常用 trait
// 或者
use relm4::SimpleComponent;
```

注意：`relm4::Component` 和 `relm4::SimpleComponent` 是不同的 trait。
`Component` 提供 `builder()` 以及更多功能（CommandOutput 等），
`SimpleComponent` 也提供 `builder()` 但更轻量。

父组件调用 `SubComponent::builder()` 时，如果报 "no function named builder"，
说明 SubComponent 没有正确实现 `SimpleComponent`（通常是宏没有展开）。

### 3. `#[relm4::component]` 宏展开失败 — "unable to determine model name"

这个错误会导致宏停止展开，连锁产生 `Root`、`init_root`、`view_output` 都找不到。
原因是在 `view_output!()` 调用之前没有一个名为 `model` 的变量。

```rust
// 错误 — 宏找不到 model
let widgets = view_output!();
let model = Self { ... };

// 正确
let model = Self { ... };
let widgets = view_output!();
```

如果 model 需要可变，用 `let mut model = ...`。

### 4. `init()` 中的 `model` 必须可变才能 push 按钮

如果你在 `init()` 中循环创建按钮并 push 到 `model.buttons`，model 必须声明为 `mut`：

```rust
let mut model = Self { buttons: Vec::new(), /* ... */ };
```

### 5. 子组件 widget 引用 — `model.xxx.widget()` vs 直接引用

在 `view!` 中，通过 `model.xxx.widget()` 引用子组件的根 widget（其中 `xxx` 是
`Controller<SubComponent>` 类型）。这个方法返回的是 `&gtk::Widget`，可以直接传给
`set_sidebar`、`set_content` 等接受 `Option<Widget>` 的方法。

如果需要在 `init()` 中操作子组件，使用 Controller 的方法：`.emit(msg)` 发送消息，
`.widget()` 获取 widget 引用。

### 6. ToggleButton 信号连接

`ToggleButton` 的 `set_active` 方法需要 `ToggleButtonExt` trait：

```rust
use relm4::gtk::prelude::ToggleButtonExt;
```

在 `view!` 中用 `connect_toggled` 连接信号：

```rust
gtk::ToggleButton {
    set_active: true,
    connect_toggled[sender] => move |_| {
        sender.output(SomeOutput::Something).unwrap();
    },
}
```

### 7. `set_name` 不存在于 ViewStack

`adw::ViewStack` 没有 `set_name` 方法。如果需要给 ViewStack 设置 widget 名称，使用：

```rust
// view! 中
adw::ViewStack {
    set_widget_name: Some("my_stack"),   // WidgetExt::set_widget_name
}
```

但对于 `#[name(stack)]` 标记的 widget，不需要手动设置名称，宏已经处理好了。

## 文件结构

```
src/ui/
├── mod.rs          # 模块声明
├── window.rs       # 根窗口 (ApplicationWindow)
├── header.rs       # 布局控制器 (OverlaySplitView)
├── sidebar.rs      # 侧边栏 (Player/Lyrics/Queue)
├── content.rs      # 内容区 (Home/Explore/Collection)
└── about.rs        # 关于对话框
```

新增组件时：
1. 创建 `.rs` 文件，按上述模板实现 `SimpleComponent`
2. 在 `mod.rs` 中添加 `pub mod xxx;`
3. 在父组件中 `use super::xxx::Xxx;` 并创建 `Controller<Xxx>`
