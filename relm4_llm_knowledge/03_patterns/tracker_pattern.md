# Tracker 模式

## 概述

Tracker 跟踪 Model 字段的变化，实现高效的条件更新。

## 基本用法

### 1. 添加依赖

```toml
[dependencies]
tracker = "0.2"
```

### 2. 标记 Model

```rust
#[tracker::track]
struct AppModel {
    counter: u32,
    label: String,
    enabled: bool,
}
```

### 3. 初始化 tracker

```rust
fn init(...) {
    let model = AppModel {
        counter: 0,
        label: String::new(),
        enabled: true,
        tracker: 0,  // 必须初始化
    };
}
```

### 4. 在 update 中重置

```rust
fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
    self.reset();  // 重置 tracker

    match msg {
        AppMsg::Increment => {
            self.set_counter(self.counter + 1);  // 使用 setter
        }
        AppMsg::UpdateLabel(text) => {
            self.set_label(text);
        }
    }
}
```

### 5. 在 view 中使用 #[track]

```rust
view! {
    gtk::Window {
        #[track = "model.changed(AppModel::counter)"]
        set_title: &format!("Counter: {}", model.counter),

        gtk::Label {
            #[track = "model.changed(AppModel::label)"]
            set_label: &model.label,
        }
    }
}
```

## Tracker 生成的 API

```rust
#[tracker::track]
struct MyStruct {
    value: u32,
}
```

生成：

```rust
impl MyStruct {
    // 获取引用
    fn get_value(&self) -> &u32;
    fn get_mut_value(&mut self) -> &mut u32;

    // 设置值（自动标记为 changed）
    fn set_value(&mut self, value: u32);

    // 用函数更新
    fn update_value<F: FnOnce(&mut u32)>(&mut self, f: F);

    // 检查是否变化
    fn changed(&self, field: u32) -> bool;

    // 重置所有变化标记
    fn reset(&mut self);
}
```

## 字段标记

```rust
impl AppModel {
    // 获取字段标记
    fn counter() -> u32 {
        0  // 字段索引
    }

    fn label() -> u32 {
        1
    }
}
```

## 条件更新

### 单个字段

```rust
#[track = "model.changed(AppModel::value)"]
set_value: model.value,
```

### 多个字段

```rust
#[track = "model.changed(AppModel::x) || model.changed(AppModel::y)"]
set_position: (model.x, model.y),
```

### 复杂条件

```rust
#[track = "model.changed(AppModel::value) && model.value > 0"]
set_sensitive: true,
```

## 完整示例

```rust
use relm4::prelude::*;

#[derive(Debug)]
pub enum AppMsg {
    Increment,
    Decrement,
    SetLabel(String),
}

#[tracker::track]
struct AppModel {
    counter: u32,
    label: String,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = u32;
    type Input = AppMsg;
    type Output = ();
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        counter: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {
            counter,
            label: "Counter".to_string(),
            tracker: 0,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Tracker Example"),
            set_default_width: 300,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    // 只有 counter 改变时才更新
                    #[track = "model.changed(AppModel::counter)"]
                    set_label: &format!("Count: {}", model.counter),
                },

                gtk::Label {
                    // 只有 label 改变时才更新
                    #[track = "model.changed(AppModel::label)"]
                    set_label: &model.label,
                },

                gtk::Box {
                    set_spacing: 5,

                    gtk::Button {
                        set_label: "-",
                        connect_clicked => AppMsg::Decrement,
                    },

                    gtk::Button {
                        set_label: "+",
                        connect_clicked => AppMsg::Increment,
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.reset();  // 重置 tracker

        match msg {
            AppMsg::Increment => {
                self.set_counter(self.counter + 1);
            }
            AppMsg::Decrement => {
                self.set_counter(self.counter - 1);
            }
            AppMsg::SetLabel(text) => {
                self.set_label(text);
            }
        }
    }
}
```

## 条件样式

```rust
#[track = "model.changed(AppModel::is_active)"]
set_class_active: ("active", model.is_active),
```

## 多条件更新

```rust
#[track = "
    model.changed(AppModel::width) ||
    model.changed(AppModel::height) ||
    model.changed(AppModel::depth)
"]
set_label: &format!("{}x{}x{}", model.width, model.height, model.depth),
```

## 调试 Tracker

```rust
#[track = "{ println!(\"Checking field\"); model.changed(AppModel::value) }"]
set_value: model.value,
```

## 嵌套结构

```rust
#[tracker::track]
struct Inner {
    value: u32,
}

#[tracker::track]
struct Outer {
    inner: Inner,
    name: String,
}
```

## 与 Vec/List 配合

```rust
#[tracker::track]
struct AppModel {
    items: Vec<String>,
}

// 检查列表变化
#[track = "model.changed(AppModel::items)"]
set_label: &format!("{} items", model.items.len()),
```

## 性能考虑

Tracker 提供的性能优势：

1. **最小更新**：只有变化的字段才会触发更新
2. **避免不必要的调用**：条件不满足时跳过 setter
3. **自动比较**：set_value 只有在新值不同时才标记为 changed

## 何时使用 Tracker

| 场景 | 是否使用 Tracker |
|------|------------------|
| 简单计数器 | ❌（直接 #[watch]） |
| 复杂 Model，多字段 | ✅ |
| 需要条件更新 | ✅ |
| 性能敏感场景 | ✅ |

## 常见错误

### 忘记 reset

```rust
fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
    // ❌ 忘记 reset
    match msg {
        AppMsg::Update => {
            self.set_value(42);
        }
    }
}

// ✅ 正确
fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
    self.reset();  // 每次更新前 reset
    match msg {
        AppMsg::Update => {
            self.set_value(42);
        }
    }
}
```

### 直接赋值

```rust
// ❌ 直接赋值不触发 tracker
self.value = 42;

// ✅ 使用 setter
self.set_value(42);

// 或手动标记
self.value = 42;
// tracker 不会检测到变化
```

## 高级技巧

### 批量更新

```rust
fn update_multiple(&mut self) {
    self.reset();

    // 多个字段改变
    self.set_value1(10);
    self.set_value2(20);
    self.set_value3(30);

    // 所有改变会在 update_view 中一起处理
}
```

### 选择性重置

```rust
// 只重置特定字段
model.reset_partial(&[AppModel::field1(), AppModel::field2()]);
```

## 完整示例：图标匹配

```rust
use relm4::prelude::*;
use rand::Rng;

#[derive(Debug)]
pub enum AppMsg {
    UpdateLeft,
    UpdateRight,
}

#[tracker::track]
struct AppModel {
    left_icon: String,
    right_icon: String,
    identical: bool,
}

fn random_icon() -> String {
    let icons = ["edit-cut", "edit-copy", "edit-paste"];
    icons[rand::thread_rng().gen_range(0..3)].to_string()
}

fn unique_icon(current: &str) -> String {
    let mut new_icon = random_icon();
    while new_icon == current {
        new_icon = random_icon();
    }
    new_icon
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
        let left_icon = random_icon();
        let right_icon = random_icon();
        let identical = left_icon == right_icon;

        let model = AppModel {
            left_icon,
            right_icon,
            identical,
            tracker: 0,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Tracker Pattern"),
            set_default_width: 400,

            #[track = "model.changed(AppModel::identical)"]
            add_css_class: if model.identical { "identical" } else { "" },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 20,

                gtk::Image {
                    #[track = "model.changed(AppModel::left_icon)"]
                    set_icon_name: Some(&model.left_icon),
                },

                gtk::Label {
                    #[track = "model.changed(AppModel::identical)"]
                    set_label: if model.identical { "Match!" } else { "" },
                },

                gtk::Image {
                    #[track = "model.changed(AppModel::right_icon)"]
                    set_icon_name: Some(&model.right_icon),
                },

                gtk::Button {
                    set_label: "Update Left",
                    connect_clicked => AppMsg::UpdateLeft,
                },

                gtk::Button {
                    set_label: "Update Right",
                    connect_clicked => AppMsg::UpdateRight,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.reset();

        match msg {
            AppMsg::UpdateLeft => {
                let new_icon = unique_icon(&self.left_icon);
                self.set_left_icon(new_icon);
                self.set_identical(self.left_icon == self.right_icon);
            }
            AppMsg::UpdateRight => {
                let new_icon = unique_icon(&self.right_icon);
                self.set_right_icon(new_icon);
                self.set_identical(self.left_icon == self.right_icon);
            }
        }
    }
}

fn main() {
    // CSS 样式
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        ".identical { background-color: #4CAF50; }".as_bytes()
    );

    gtk::StyleContext::add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let app = RelmApp::new("com.example.tracker");
    app.run::<App>(());
}
```
