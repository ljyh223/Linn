use crate::{api::init_client, db::Db, ui::window::Window};
use relm4::{
    RelmApp,
    gtk::gio::{self, Settings, prelude::SettingsExt},
};
use std::{env, process::Command, sync::{Arc, Mutex}};

mod api;
mod db;
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
    Command::new("glib-compile-schemas")
        .arg("data")
        .status()
        .expect("compile schemas failed");

    unsafe {
        #[cfg(debug_assertions)]
        env::set_var("GSETTINGS_SCHEMA_DIR", "data");
    }
    eprintln!("Starting Linn...");

    let cookie = get_cookie();
    init_client(cookie.clone());
    let css_path = std::env::current_dir().unwrap().join("src/style.css");

    let db = Arc::new(Mutex::new(Db::open().expect("Failed to open database")));

    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
    gst::init().expect("Failed to initialize GStreamer");
    relm4::set_global_css_from_file(&css_path).expect("load CSS failed");

    let app = RelmApp::new(APPLICATION_ID);
    app.run::<Window>((cookie, db));
}