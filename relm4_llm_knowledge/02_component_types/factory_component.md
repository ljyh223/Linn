# FactoryComponent 动态列表

## 用途

用于渲染动态数据集合（如列表、网格）。

## 基本结构

```rust
#[relm4::factory]
impl FactoryComponent for Counter {
    type Init = u8;
    type Input = CounterMsg;
    type Output = CounterOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    fn init_model(
        value: Self::Init,
        index: &DynamicIndex,
        sender: FactorySender<Self>,
    ) -> Self {
        Self { value }
    }

    fn init_widgets(
        &mut self,
        index: &DynamicIndex,
        root: &Self::Root,
        returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        view_output!()
    }
}
```

## 使用 FactoryVecDeque

```rust
struct AppModel {
    counters: FactoryVecDeque<Counter>,
}

fn init(...) {
    let counters = FactoryVecDeque::builder()
        .launch(box_widget)
        .forward(|msg| AppMsg::CounterOutput(msg))
        .build();

    // 添加数据
    let mut guard = counters.guard();
    guard.push_back(0);
}
```

## 更新 Factory 数据

所有修改操作都需要 guard：

```rust
fn update(&mut self, msg: AppMsg, _: ComponentSender<Self>) {
    match msg {
        AppMsg::AddCounter => {
            let mut guard = self.counters.guard();
            guard.push_back(0);
            // guard drop 后自动更新 UI
        }

        AppMsg::RemoveCounter(index) => {
            let mut guard = self.counters.guard();
            guard.remove(index.current_index());
        }

        AppMsg::MoveUp(index) => {
            let mut guard = self.counters.guard();
            if let Some(current) = index.current() {
                guard.move_up(current);
            }
        }

        AppMsg::MoveDown(index) => {
            let mut guard = self.counters.guard();
            if let Some(current) = index.current() {
                guard.move_down(current);
            }
        }
    }
}
```

## 完整示例

```rust
#[derive(Debug)]
pub enum CounterMsg {
    Increment,
    Decrement,
}

pub enum CounterOutput {
    MoveUp(DynamicIndex),
    MoveDown(DynamicIndex),
    Remove(DynamicIndex),
}

#[tracker::track]
struct Counter {
    value: u8,
}

#[relm4::factory]
impl FactoryComponent for Counter {
    type Init = u8;
    type Input = CounterMsg;
    type Output = CounterOutput;
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
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,

            gtk::Button {
                set_label: "-",
                connect_clicked => CounterMsg::Decrement,
            },

            gtk::Label {
                #[watch]
                set_label: &self.value.to_string(),
                set_width_chars: 3,
            },

            gtk::Button {
                set_label: "+",
                connect_clicked => CounterMsg::Increment,
            },

            gtk::Button {
                set_label: "↑",
                connect_clicked[sender, index] => move |_| {
                    sender.output(CounterOutput::MoveUp(index.clone()));
                }
            },

            gtk::Button {
                set_label: "↓",
                connect_clicked[sender, index] => move |_| {
                    sender.output(CounterOutput::MoveDown(index.clone()));
                }
            },

            gtk::Button {
                set_label: "×",
                connect_clicked[sender, index] => move |_| {
                    sender.output(CounterOutput::Remove(index.clone()));
                }
            },
        }
    }
}
```

## 主组件使用 Factory

```rust
#[relm4::component]
impl SimpleComponent for App {
    view! {
        gtk::Window {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Button {
                    set_label: "Add counter",
                    connect_clicked => AppMsg::AddCounter,
                },

                #[local_ref]
                counter_box -> gtk::Box {
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
        let counter_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let counters = FactoryVecDeque::builder()
            .launch(counter_box.clone())
            .forward(|msg| AppMsg::CounterOutput(msg))
            .build();

        let model = AppModel { counters };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AppMsg, _: ComponentSender<Self>) {
        match msg {
            AppMsg::AddCounter => {
                let mut guard = self.counters.guard();
                guard.push_back(0);
            }
            AppMsg::CounterOutput(CounterOutput::MoveUp(index)) => {
                let mut guard = self.counters.guard();
                if let Some(idx) = index.current() {
                    guard.move_front(idx);
                }
            }
            AppMsg::CounterOutput(CounterOutput::MoveDown(index)) => {
                let mut guard = self.counters.guard();
                if let Some(idx) = index.current() {
                    guard.move_back(idx);
                }
            }
            AppMsg::CounterOutput(CounterOutput::Remove(index)) => {
                let mut guard = self.counters.guard();
                guard.remove(index.current_index());
            }
        }
    }
}
```

## Grid Factory（位置函数）

对于 `gtk::Grid` 等需要固定位置的容器：

```rust
impl FactoryComponent for GridItem {
    // ... 其他方法

    fn position(&self, index: &usize) -> GridPosition {
        let index = *index as i32;
        let row = index / 3;
        let col = index % 3;

        GridPosition {
            column: col,
            row,
            width: 1,
            height: 1,
        }
    }
}
```

## Factory 方法

| 方法 | 说明 |
|------|------|
| `push_back(value)` | 末尾添加 |
| `push_front(value)` | 开头添加 |
| `insert(index, value)` | 指定位置插入 |
| `remove(index)` | 删除元素 |
| `move_to_front(index)` | 移到开头 |
| `move_to_back(index)` | 移到末尾 |
| `move_up(index)` | 向上移动 |
| `move_down(index)` | 向下移动 |
| `clear()` | 清空 |
| `get(index)` | 获取元素 |
| `len()` | 获取长度 |

## 注意事项

1. **必须使用 guard**：所有修改操作都需要在 guard 内完成
2. **自动更新**：guard drop 后自动更新 UI
3. **DynamicIndex**：元素可能移动，使用 `DynamicIndex` 而非 `usize`
4. **性能优化**：factory 会最小化 UI 更新
