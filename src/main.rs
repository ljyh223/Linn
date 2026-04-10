use relm4::RelmApp;

use crate::ui::window::Window;

mod api;
mod utils;
mod ui;

pub const APPLICATION_ID: &str = "org.ljyh.linn";

mod icon_names {
    pub use shipped::*;
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

fn main() {
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
    gst::init().expect("Failed to initialize GStreamer");

    let app = RelmApp::new(APPLICATION_ID);
    app.run::<Window>(());
}
