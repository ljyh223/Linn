# Worker 无 GUI 组件

## 用途

- 无 Widget 的后台组件
- 适合 CPU 密集型任务
- 在单独线程运行，不阻塞 UI

## 基本结构

```rust
pub struct Worker {
    counter: u32,
}

pub enum WorkerMsg {
    Increment,
    GetCount,
}

pub enum WorkerOutput {
    Count(u32),
}

impl Worker for Worker {
    type Init = u32;
    type Input = WorkerMsg;
    type Output = WorkerOutput;

    fn init(init: Self::Init, sender: ComponentSender<Self>) -> Self {
        Self { counter: init }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            WorkerMsg::Increment => {
                self.counter += 1;
            }
            WorkerMsg::GetCount => {
                sender.output(WorkerOutput::Count(self.counter));
            }
        }
    }
}
```

## 创建 Worker

```rust
struct AppModel {
    worker: WorkerController<Worker>,
}

fn init(...) {
    let worker = Worker::builder()
        .detach_worker(())
        .forward(&sender.input, |msg| AppMsg::WorkerOutput(msg));

    AppModel { worker }
}
```

## 与 Worker 通信

```rust
impl Component for App {
    fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::IncrementWorker => {
                self.worker.input(WorkerMsg::Increment);
            }
            AppMsg::WorkerOutput(WorkerOutput::Count(count)) => {
                self.count = count;
            }
        }
    }
}
```

## 完整示例

```rust
pub struct Worker {
    counter: u32,
}

#[derive(Debug)]
pub enum WorkerMsg {
    Increment,
    Decrement,
    GetCount,
}

#[derive(Debug)]
pub enum WorkerOutput {
    Count(u32),
}

impl Worker for Worker {
    type Init = u32;
    type Input = WorkerMsg;
    type Output = WorkerOutput;

    fn init(count: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { counter: count }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            WorkerMsg::Increment => {
                self.counter += 1;
            }
            WorkerMsg::Decrement => {
                self.counter -= 1;
            }
            WorkerMsg::GetCount => {
                sender.output(WorkerOutput::Count(self.counter));
            }
        }
    }
}

// 主组件使用 Worker

#[derive(Debug)]
pub enum AppMsg {
    Increment,
    Decrement,
    WorkerOutput(WorkerOutput),
}

pub struct App {
    worker: WorkerController<Worker>,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = u32;
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::Window {
            set_title: Some("Worker Example"),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Button {
                    set_label: "Increment (in worker)",
                    connect_clicked[sender] => move |_| {
                        sender.input(AppMsg::Increment);
                    }
                },

                gtk::Button {
                    set_label: "Decrement (in worker)",
                    connect_clicked[sender] => move |_| {
                        sender.input(AppMsg::Decrement);
                    }
                },

                gtk::Button {
                    set_label: "Get count from worker",
                    connect_clicked[sender] => move |_| {
                        sender.input(AppMsg::Increment);
                        sender.input(AppMsg::WorkerOutput(WorkerOutput::Count(0)));
                    }
                }
            }
        }
    }

    fn init(
        value: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let worker = Worker::builder()
            .detach_worker(value)
            .forward(sender.input_sender(), |msg| AppMsg::WorkerOutput(msg));

        let model = App { worker };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => {
                self.worker.input(WorkerMsg::Increment);
            }
            AppMsg::Decrement => {
                self.worker.input(WorkerMsg::Decrement);
            }
            AppMsg::WorkerOutput(WorkerOutput::Count(count)) => {
                println!("Worker count: {}", count);
            }
        }
    }
}
```

## Worker vs Commands

| 特性 | Worker | Commands |
|------|--------|----------|
| 线程 | 独立线程 | Tokio runtime |
| 适用场景 | CPU 密集型 | I/O 密集型 |
| 持久状态 | ✅ | ❌ |
| 串行处理 | ✅ | 可并行 |

## 使用建议

- **CPU 密集型任务**：使用 Worker
- **需要持久状态**：使用 Worker
- **简单后台任务**：使用 Commands
- **多个并行任务**：使用 Commands
