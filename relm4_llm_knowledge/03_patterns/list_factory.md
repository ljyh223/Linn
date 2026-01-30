# List Factory 模式

## 概述

使用 Factory 动态渲染列表数据，自动处理添加、删除、移动等操作。

## 基本模式

### 1. 定义 Factory 元素

```rust
#[derive(Debug)]
pub enum ItemMsg {
    Increment,
    Remove,
}

pub enum ItemOutput {
    Remove(DynamicIndex),
}

#[tracker::track]
struct Item {
    value: u32,
}

#[relm4::factory]
impl FactoryComponent for Item {
    type Init = u32;
    type Input = ItemMsg;
    type Output = ItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    fn init_model(
        value: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { value, tracker: 0 }
    }

    view! {
        gtk::Box {
            set_spacing: 5,

            gtk::Label {
                #[watch]
                set_label: &self.value.to_string(),
            },

            gtk::Button {
                set_label: "+",
                connect_clicked => ItemMsg::Increment,
            },

            gtk::Button {
                set_label: "Remove",
                connect_clicked[sender, index] => move |_| {
                    sender.output(ItemOutput::Remove(index.clone()));
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            ItemMsg::Increment => {
                self.value += 1;
            }
            ItemMsg::Remove => {
                // 由父组件处理
            }
        }
    }
}
```

### 2. 主组件使用 Factory

```rust
#[derive(Debug)]
pub enum AppMsg {
    AddItem,
    RemoveItem(DynamicIndex),
    ItemOutput(ItemOutput),
}

pub struct App {
    items: FactoryVecDeque<Item>,
}

#[relm4::component]
impl SimpleComponent for App {
    view! {
        gtk::Window {
            set_title: Some("List Factory"),
            set_default_width: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::Button {
                    set_label: "Add Item",
                    connect_clicked => AppMsg::AddItem,
                },

                #[local_ref]
                items_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let items_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(5)
            .build();

        let items = FactoryVecDeque::builder()
            .launch(items_box.clone())
            .forward(|msg| AppMsg::ItemOutput(msg))
            .build();

        // 初始化一些数据
        let mut guard = items.guard();
        for i in 0..5 {
            guard.push_back(i);
        }

        let model = App { items };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AppMsg, _: ComponentSender<Self>) {
        match msg {
            AppMsg::AddItem => {
                let len = self.items.len();
                let mut guard = self.items.guard();
                guard.push_back(len as u32);
            }
            AppMsg::RemoveItem(index) => {
                let mut guard = self.items.guard();
                guard.remove(index.current_index());
            }
            AppMsg::ItemOutput(ItemOutput::Remove(index)) => {
                let mut guard = self.items.guard();
                guard.remove(index.current_index());
            }
        }
    }
}
```

## 高级操作

### 移动元素

```rust
fn update(&mut self, msg: AppMsg, _: ComponentSender<Self>) {
    match msg {
        AppMsg::MoveUp(index) => {
            let mut guard = self.items.guard();
            if let Some(current) = index.current() {
                guard.move_up(current);
            }
        }
        AppMsg::MoveDown(index) => {
            let mut guard = self.items.guard();
            if let Some(current) = index.current() {
                guard.move_down(current);
            }
        }
        AppMsg::MoveToTop(index) => {
            let mut guard = self.items.guard();
            if let Some(current) = index.current() {
                guard.move_front(current);
            }
        }
    }
}
```

### 批量操作

```rust
fn add_multiple_items(&mut self, values: Vec<u32>) {
    let mut guard = self.items.guard();
    for value in values {
        guard.push_back(value);
    }
    // guard drop 时一次性更新 UI
}
```

### 过滤和转换

```rust
fn remove_all_greater_than(&mut self, threshold: u32) {
    let mut indices_to_remove = Vec::new();

    for (index, item) in self.items.iter().enumerate() {
        if item.value > threshold {
            indices_to_remove.push(index);
        }
    }

    let mut guard = self.items.guard();
    // 从后往前删除，避免索引问题
    for index in indices_to_remove.into_iter().rev() {
        guard.remove(index);
    }
}
```

## Grid 布局列表

```rust
impl FactoryComponent for GridItem {
    // ... 其他方法

    fn position(&self, index: &usize) -> GridPosition {
        let index = *index as i32;
        let items_per_row = 3;

        GridPosition {
            column: index % items_per_row,
            row: index / items_per_row,
            width: 1,
            height: 1,
        }
    }
}

// 使用 gtk::Grid 作为容器
let grid = gtk::Grid::new();

let items = FactoryVecDeque::builder()
    .launch(grid)
    .forward(|msg| AppMsg::ItemOutput(msg))
    .build();
```

## 性能优化

Factory 自动优化：
- 只更新变化的元素
- 复用现有 widgets
- 最小化重排

无需手动优化，factory 已处理。

## 常见模式

### 带索引的操作

```rust
pub enum ItemMsg {
    Update { index: usize, value: String },
}

// 元素输出时携带自己的 index
connect_changed[sender, index] => move |entry| {
    let text = entry.text().to_string();
    sender.input(ItemMsg::Update {
        index: index.current_index(),
        value: text,
    });
}
```

### 选择模式

```rust
#[tracker::track]
struct Item {
    value: String,
    selected: bool,
}

pub enum ItemMsg {
    ToggleSelect,
    UpdateValue(String),
}
```

## 完整示例

```rust
use relm4::prelude::*;
use relm4::factory::*;
use gtk::prelude::*;

#[derive(Debug)]
pub enum ItemMsg {
    Increment,
    Decrement,
    Remove,
    MoveUp,
    MoveDown,
}

pub enum ItemOutput {
    Remove(DynamicIndex),
    MoveUp(DynamicIndex),
    MoveDown(DynamicIndex),
}

#[tracker::track]
struct Item {
    value: i32,
}

#[relm4::factory]
impl FactoryComponent for Item {
    type Init = i32;
    type Input = ItemMsg;
    type Output = ItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    fn init_model(
        value: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { value, tracker: 0 }
    }

    view! {
        gtk::Box {
            set_spacing: 5,
            set_margin_all: 5,

            gtk::Button {
                set_label: "-",
                connect_clicked => ItemMsg::Decrement,
            },

            gtk::Label {
                #[watch]
                set_label: &self.value.to_string(),
                set_width_chars: 3,
            },

            gtk::Button {
                set_label: "+",
                connect_clicked => ItemMsg::Increment,
            },

            gtk::Button {
                set_label: "↑",
                connect_clicked[sender, index] => move |_| {
                    sender.output(ItemOutput::MoveUp(index.clone()));
                }
            },

            gtk::Button {
                set_label: "↓",
                connect_clicked[sender, index] => move |_| {
                    sender.output(ItemOutput::MoveDown(index.clone()));
                }
            },

            gtk::Button {
                set_label: "×",
                connect_clicked[sender, index] => move |_| {
                    sender.output(ItemOutput::Remove(index.clone()));
                }
            },
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            ItemMsg::Increment => self.value += 1,
            ItemMsg::Decrement => self.value -= 1,
            _ => {}
        }
    }
}
```
