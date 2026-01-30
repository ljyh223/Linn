# view! 宏

## 基本语法

```rust
view! {
    gtk::Window {
        set_title: Some("My App"),
        set_default_width: 400,

        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                set_label: "Hello",
            }
        }
    }
}
```

## Widget 构造方式

### 1. Default 实现

```rust
gtk::Window {
    // 使用 Default::default()
}
```

### 2. 构造函数

```rust
gtk::Label::new(Some("Text")) {
    // 使用构造函数
}

gtk::Button::with_label("Click me") {
    // 使用 with_label
}
```

### 3. Builder 模式

```rust
gtk::Label::builder()
    .label("Builder label")
    .selectable(true)
    .build() {
    // 使用 builder
}
```

### 4. 自定义函数

```rust
set_child: create_custom_widget() -> gtk::Box {
    // 指定返回类型
}
```

## 属性设置

### 基本属性

```rust
gtk::Window {
    set_title: Some("Title"),
    set_default_width: 400,
    set_resizable: true,
}
```

### 多参数属性

```rust
gtk::Grid {
    attach[0, 0, 1, 2] = &gtk::Label {
        // attach(label, 0, 0, 1, 2)
    }
}
```

### Option 属性（仅 Some 时设置）

```rust
gtk::Window {
    set_title?: Some("Title"),  // 设置
    set_icon?: None,            // 不调用
}
```

## 子 Widget

### 嵌套子 Widget

```rust
gtk::Window {
    gtk::Box {
        gtk::Label {
            set_label: "Child",
        }
    }
}
```

### 使用方法添加

```rust
gtk::Box {
    // 使用 append 方法
    append = &gtk::Label {
        set_label: "Using append",
    }
}
```

### Option 包装

```rust
#[wrap(Some)]
set_child = gtk::Label {
    set_label: "Wrapped in Some",
}
```

## 响应式更新

### #[watch] - 自动更新

每次 `update_view` 都会更新：

```rust
gtk::Label {
    #[watch]
    set_label: &format!("Count: {}", self.model.counter),
}
```

### #[track] - 条件更新

仅当条件为真时更新（需要 tracker）：

```rust
gtk::Window {
    #[track = "model.changed(AppModel::counter)"]
    set_title: &format!("Counter: {}", model.counter),
}
```

## 事件连接

### 简单消息发送

```rust
gtk::Button {
    connect_clicked => AppMsg::Increment,
}
```

### 带 closure 的连接

```rust
gtk::Button {
    connect_clicked[sender] => move |button| {
        sender.input(AppMsg::CustomAction);
    }
}
```

### 克隆变量

```rust
gtk::Entry {
    connect_changed[sender, entry] => move |entry| {
        let text = entry.text().to_string();
        sender.input(AppMsg::TextChanged(text));
    }
}
```

### 信号处理命名（用于阻塞）

```rust
gtk::SpinButton {
    connect_value_changed@changed => move |spin| {
        // 处理变化
    }
}
```

## 条件 Widget

### if 语句

```rust
gtk::Box {
    if model.show_label {
        gtk::Label {
            set_label: "Visible",
        }
    }
}
```

### match 语句

```rust
#[transition = "SlideRight"]
match model.state {
    AppState::Loading => {
        gtk::Spinner {
            set_spinning: true,
        }
    }
    AppState::Ready => {
        gtk::Label {
            set_label: "Ready",
        }
    }
}
```

### 条件解构（需 track/watch）

```rust
match &model.item {
    Some(value) => {
        gtk::Label {
            #[watch]
            set_label: &value.to_string(),
        }
    }
    None => {
        gtk::Label {
            set_label: "None",
        }
    }
}
```

## Widget 命名

### name 属性

```rust
#[name = "my_label"]
gtk::Label {
    set_label: "Named",
}

// 访问：widgets.my_label
```

### 赋值命名

```rust
set_child: my_button = gtk::Button {
    set_label: "Button",
}

// 访问：widgets.my_button
```

## 返回 Widget

某些方法返回新 widget：

```rust
gtk::Stack {
    add_child = &gtk::Label {
        set_label: "Page 1",
    } -> {
        set_title: "Page 1",
    }
}
```

## 局部引用（#[local_ref]）

引用 init 中的变量：

```rust
fn init(...) {
    let list_box = gtk::Box::new(...);

    view! {
        gtk::Window {
            #[local_ref]
            list_box -> gtk::Box {
                // 配置
            }
        }
    }
}
```

## 阻塞信号

防止循环触发：

```rust
gtk::SpinButton {
    connect_value_changed@changed => move |spin| {
        // 处理
    }

    #[watch]
    #[block_signal(changed)]
    set_value: model.value,
}
```

## 迭代器

```rust
#[iterate]
add_css_class: model.classes,
```

## 完整示例

```rust
view! {
    gtk::Window {
        set_title: Some("Counter App"),
        set_default_width: 300,

        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,

            gtk::Label {
                #[watch]
                set_label: &format!("Count: {}", self.model.counter),
            },

            gtk::Box {
                set_spacing: 10,

                gtk::Button {
                    set_label: "+",
                    connect_clicked => AppMsg::Increment,
                },

                gtk::Button {
                    set_label: "-",
                    connect_clicked => AppMsg::Decrement,
                },

                gtk::Button {
                    set_label: "Reset",
                    #[track = "model.changed(AppModel::counter)"]
                    set_sensitive: model.counter > 0,
                    connect_clicked => AppMsg::Reset,
                },
            }
        }
    }
}
```
