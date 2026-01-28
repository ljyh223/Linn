mod app;
mod pages;
mod ui;

use relm4::prelude::*;
use app::AppModel;

fn main() {
    let app = RelmApp::new("com.linn.music-player");
    app.run::<AppModel>(());
}
