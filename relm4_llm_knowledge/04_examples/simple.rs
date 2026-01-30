// Simple counter example using relm4
// Run: cargo run --example simple

use relm4::prelude::*;

#[derive(Debug)]
pub enum AppMsg {
    Increment,
    Decrement,
}

pub struct AppModel {
    counter: u8,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = u8;
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
        let model = AppModel { counter };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Simple Counter"),
            set_default_width: 300,
            set_default_height: 100,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,

                gtk::Label {
                    #[watch]
                    set_label: &format!("Counter: {}", self.model.counter),
                },

                gtk::Box {
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "-",
                        connect_clicked => AppMsg::Decrement,
                    },

                    gtk::Button {
                        set_label: "+",
                        connect_clicked => AppMsg::Increment,
                    },
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => {
                self.model.counter = self.model.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.model.counter = self.model.counter.wrapping_sub(1);
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("com.example.simple");
    app.run::<App>(0);
}
