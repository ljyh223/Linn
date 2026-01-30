# Widgets

## Widget 特性

GTK Widgets 类似 `Rc`：

- **Clone 增加引用**：`widget.clone()` 不创建新实例
- **自动生命周期**：只要有引用就保持存活
- **非线程安全**：不实现 `Send`，仅主线程使用

## 获取 Widgets

### 使用宏（推荐）

```rust
#[relm4::component]
impl SimpleComponent for App {
    view! {
        gtk::Window {
            #[name = "label"]
            gtk::Label {
                set_label: "Text",
            }
        }
    }
}

// 访问：widgets.label
```

### 手动定义

```rust
struct AppWidgets {
    window: gtk::Window,
    label: gtk::Label,
    button: gtk::Button,
}

impl Widgets for AppWidgets {
    type Root = gtk::Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }
}
```

## 命名 Widget

### 使用 #[name]

```rust
view! {
    gtk::Window {
        #[name = "header_bar"]
        gtk::HeaderBar {
            set_title: Some("App"),
        }
    }
}

// widgets.header_bar
```

### 使用赋值命名

```rust
view! {
    gtk::Window {
        set_titlebar = header_bar: gtk::HeaderBar {
            set_title: Some("App"),
        }
    }
}

// widgets.header_bar
```

## 不需要存储的 Widget

由于 GTK 自动管理引用，子 Widget 不需要显式存储：

```rust
view! {
    gtk::Window {
        gtk::Box {
            // 不需要命名，GTK 自动管理
            gtk::Button { set_label: "OK" },
            gtk::Button { set_label: "Cancel" },
        }
    }
}
```

## Root Widget

每个组件必须有一个 root widget：

```rust
impl SimpleComponent for App {
    type Root = gtk::Window;  // 主组件必须是 Window

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }
}
```

子组件可以是其他类型：

```rust
impl SimpleComponent for Dialog {
    type Root = gtk::Dialog;  // 子组件可以是 Dialog
}
```

## 手动操作 Widgets

### 在 update_view 中

```rust
fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
    widgets.label.set_label(&self.model.value);
    widgets.button.set_sensitive(self.model.value > 0);
}
```

### 使用 #[watch] 自动

```rust
view! {
    gtk::Label {
        #[watch]
        set_label: &self.model.value,
    }
}
```

## Widget 方法

### 设置属性

```rust
widget.set_property(value);
```

### 获取属性

```rust
let value = widget.property();
```

### 连接信号

```rust
widget.connect_signal(|widget| {
    // 处理
});
```

## 常用 Widget

### 容器

| Widget | 用途 |
|--------|------|
| `gtk::Window` | 主窗口 |
| `gtk::Box` | 线性布局 |
| `gtk::Grid` | 网格布局 |
| `gtk::Stack` | 页面切换 |
| `gtk::Notebook` | 标签页 |
| `gtk::ScrolledWindow` | 滚动容器 |

### 控件

| Widget | 用途 |
|--------|------|
| `gtk::Button` | 按钮 |
| `gtk::Label` | 文本标签 |
| `gtk::Entry` | 文本输入 |
| `gtk::TextView` | 多行文本 |
| `gtk::CheckButton` | 复选框 |
| `gtk::SpinButton` | 数字输入 |
| `gtk::ComboBox` | 下拉选择 |

### 显示

| Widget | 用途 |
|--------|------|
| `gtk::Image` | 图片 |
| `gtk::ProgressBar` | 进度条 |
| `gtk::Spinner` | 加载动画 |

## 预导入 Trait

使用 widget 方法前需要导入 trait：

```rust
use gtk::prelude::*;  // 导入所有 GTK traits
```

或指定导入：

```rust
use gtk::GtkWindowExt;  // 仅导入 Window trait
```

## CSS 样式

### 添加 CSS 类

```rust
widget.add_css_class("custom-class");
```

### 条件 CSS 类

```rust
widget.set_class_active("active", model.is_active);
```

### 设置样式

```rust
fn main() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data("
        .active {
            background-color: green;
        }
    ".as_bytes());

    gtk::StyleContext::add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
```

## 完整示例

```rust
#[relm4::component]
impl SimpleComponent for App {
    view! {
        gtk::Window {
            set_title: Some("Widget Example"),
            set_default_width: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                #[name = "label"]
                gtk::Label {
                    set_label: "Hello, World!",
                },

                #[name = "entry"]
                gtk::Entry {
                    set_placeholder_text: Some("Type something..."),
                    connect_changed[sender] => move |entry| {
                        let text = entry.text().to_string();
                        sender.input(AppMsg::TextChanged(text));
                    }
                },

                #[name = "button"]
                gtk::Button {
                    set_label: "Click Me",
                    connect_clicked => AppMsg::ButtonClicked,
                },
            }
        }
    }
}
```
