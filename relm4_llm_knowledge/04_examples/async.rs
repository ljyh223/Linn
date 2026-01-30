// Async component example
// Run: cargo run --example async

use relm4::prelude::*;
use std::time::Duration;

#[derive(Debug)]
pub enum AppMsg {
    Load,
    DataLoaded(String),
}

pub struct App {
    data: String,
    loading: bool,
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init_loading_widgets(root: &Self::Root) -> Option<LoadingWidgets> {
        view! {
            gtk::Window {
                set_title: Some("Loading..."),
                set_default_width: 300,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_margin_all: 20,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,

                    gtk::Spinner {
                        set_spinning: true,
                    },

                    gtk::Label {
                        set_label: "Loading data...",
                    }
                }
            }
        }
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Simulate async loading
        tokio::time::sleep(Duration::from_secs(2)).await;

        let model = App {
            data: "Initial data loaded".to_string(),
            loading: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("Async Example"),
            set_default_width: 300,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 20,

                gtk::Label {
                    #[watch]
                    set_label: &format!("Data: {}", self.model.data),
                },

                gtk::Spinner {
                    #[watch]
                    set_spinning: self.model.loading,
                },

                gtk::Button {
                    set_label: "Reload",
                    connect_clicked => AppMsg::Load,
                }
            }
        }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            AppMsg::Load => {
                self.loading = true;
                // Simulate async operation
                tokio::time::sleep(Duration::from_secs(1)).await;
                self.data = format!("Reloaded at {:?}", std::time::SystemTime::now());
                self.loading = false;
            }
            AppMsg::DataLoaded(data) => {
                self.data = data;
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("com.example.async");
    app.run::<App>(());
}
