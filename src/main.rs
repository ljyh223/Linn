use crate::{api::init_client, db::Db, ui::window::Window};
use relm4::{
    RelmApp,
    gtk::gio::{Settings, prelude::SettingsExt},
};
use std::sync::{Arc, Mutex};

mod api;
mod db;
mod player;
mod ui;
mod utils;

pub const APPLICATION_ID: &str = "io.github.ljyh223.Linn";
pub const APP_NAME: &str = "Linn";

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

const STYLE_CSS: &str = include_str!("style.css");

fn get_cookie() -> String {
    let settings = Settings::new(APPLICATION_ID);
    settings.string("cookie").to_string()
}

fn main() {
    env_logger::init();

    #[cfg(debug_assertions)]
    {
        std::process::Command::new("glib-compile-schemas")
            .arg("data")
            .status()
            .expect("compile schemas failed");
        unsafe {
            std::env::set_var("GSETTINGS_SCHEMA_DIR", "data");
        }
    }

    eprintln!("Starting Linn...");

    let cookie = get_cookie();
    init_client(cookie.clone());

    let db = Arc::new(Mutex::new(Db::open().expect("Failed to open database")));

    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
    gst::init().expect("Failed to initialize GStreamer");
    relm4::set_global_css(STYLE_CSS);

    let app = RelmApp::new(APPLICATION_ID);
    app.run::<Window>((cookie, db));
}