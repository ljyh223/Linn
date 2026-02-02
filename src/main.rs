use relm4::RelmApp;

mod app;
mod widgets;

use app::App;

fn main() {
    // 创建应用
    let app = RelmApp::new("com.github.linn");
    app.run::<App>(());
}
