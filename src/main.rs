use crate::{api::init_client, ui::window::Window};
use relm4::{
    RelmApp,
    gtk::gio::{Settings, prelude::SettingsExt},
};
use std::env;

mod api;
mod player;
mod ui;
mod utils;

pub const APPLICATION_ID: &str = "org.ljyh.linn";
pub const APP_NAME: &str = "Linn";

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

fn get_cookie() -> String {
    let settings = Settings::new(APPLICATION_ID);
    settings.string("cookie").to_string()
}

fn main() {

    unsafe {
        #[cfg(debug_assertions)]
        env::set_var("GSETTINGS_SCHEMA_DIR", "data");
    }
    eprintln!("Starting Linn...");
    eprintln!("Current user cookie: {}", get_cookie());

    
    let cookie = get_cookie();
    init_client(cookie);

    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
    gst::init().expect("Failed to initialize GStreamer");

    let app = RelmApp::new(APPLICATION_ID);
    app.run::<Window>(());
}
