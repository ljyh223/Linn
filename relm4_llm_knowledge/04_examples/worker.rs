// Worker example - background computation
// Run: cargo run --example worker

use relm4::prelude::*;

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
                // Simulate CPU-intensive work
                let mut sum = 0u64;
                for i in 0..1_000_000 {
                    sum = sum.wrapping_add(i);
                }
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

#[derive(Debug)]
pub enum AppMsg {
    IncrementWorker,
    DecrementWorker,
    GetWorkerCount,
    WorkerCount(u32),
}

pub struct App {
    worker: WorkerController<Worker>,
    display_count: u32,
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
        initial_count: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let worker = Worker::builder()
            .detach_worker(initial_count)
            .forward(sender.input_sender(), |msg| match msg {
                WorkerOutput::Count(count) => AppMsg::WorkerCount(count),
            });

        let model = App {
            worker,
            display_count: initial_count,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Worker Example"),
            set_default_width: 300,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 20,

                gtk::Label {
                    #[watch]
                    set_label: &format!("Worker count: {}", self.model.display_count),
                },

                gtk::Label {
                    set_label: "(Worker runs on separate thread)",
                },

                gtk::Box {
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "- (Worker)",
                        connect_clicked => AppMsg::DecrementWorker,
                    },

                    gtk::Button {
                        set_label: "+ (Worker)",
                        connect_clicked => AppMsg::IncrementWorker,
                    },
                },

                gtk::Button {
                    set_label: "Get count from worker",
                    connect_clicked => AppMsg::GetWorkerCount,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::IncrementWorker => {
                self.worker.input(WorkerMsg::Increment);
            }
            AppMsg::DecrementWorker => {
                self.worker.input(WorkerMsg::Decrement);
            }
            AppMsg::GetWorkerCount => {
                self.worker.input(WorkerMsg::GetCount);
            }
            AppMsg::WorkerCount(count) => {
                self.display_count = count;
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("com.example.worker");
    app.run::<App>(0);
}
