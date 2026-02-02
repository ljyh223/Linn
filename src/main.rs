use gtk::prelude::*;
use relm4::{
    gtk, gtk::glib, Component, ComponentController, Controller, RelmApp, Sender,
    SimpleComponent,
};

mod app;
mod widgets;

use app::App;

#[async_std::main]
async fn main() {
    // 创建应用
    let app = RelmApp::new("com.github.linn");
    app.run_async::<App>(()).await;
}
