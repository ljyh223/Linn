// Factory example - dynamic list of counters
// Run: cargo run --example factory

use relm4::prelude::*;
use relm4::factory::*;

#[derive(Debug)]
pub enum CounterMsg {
    Increment,
    Decrement,
    Remove,
    MoveUp,
    MoveDown,
}

pub enum CounterOutput {
    Remove(DynamicIndex),
    MoveUp(DynamicIndex),
    MoveDown(DynamicIndex),
}

#[tracker::track]
struct Counter {
    value: i32,
}

#[relm4::factory]
impl FactoryComponent for Counter {
    type Init = i32;
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
            set_spacing: 5,
            set_margin_all: 5,

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

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            CounterMsg::Increment => self.value += 1,
            CounterMsg::Decrement => self.value -= 1,
            _ => {}
        }
    }
}

#[derive(Debug)]
pub enum AppMsg {
    AddCounter,
    RemoveCounter(DynamicIndex),
    MoveUp(DynamicIndex),
    MoveDown(DynamicIndex),
}

pub struct App {
    counters: FactoryVecDeque<Counter>,
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
        let counter_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(5)
            .margin_all(10)
            .build();

        let counters = FactoryVecDeque::builder()
            .launch(counter_box.clone())
            .forward(|msg| match msg {
                CounterOutput::Remove(idx) => AppMsg::RemoveCounter(idx),
                CounterOutput::MoveUp(idx) => AppMsg::MoveUp(idx),
                CounterOutput::MoveDown(idx) => AppMsg::MoveDown(idx),
            })
            .build();

        let model = App { counters };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Factory Example"),
            set_default_width: 400,
            set_default_height: 500,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::Button {
                    set_label: "Add Counter",
                    connect_clicked => AppMsg::AddCounter,
                },

                #[local_ref]
                counter_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::AddCounter => {
                let mut guard = self.counters.guard();
                guard.push_back(0);
            }
            AppMsg::RemoveCounter(index) => {
                let mut guard = self.counters.guard();
                guard.remove(index.current_index());
            }
            AppMsg::MoveUp(index) => {
                let mut guard = self.counters.guard();
                if let Some(current) = index.current() {
                    guard.move_front(current);
                }
            }
            AppMsg::MoveDown(index) => {
                let mut guard = self.counters.guard();
                if let Some(current) = index.current() {
                    guard.move_back(current);
                }
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("com.example.factory");
    app.run::<App>(());
}
