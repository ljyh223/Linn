//! Linn - 网易云音乐第三方客户端
//!
//! 基于 GTK4 和 Relm4 构建的现代化网易云音乐客户端。

use relm4::RelmApp;

mod api;
mod app;
mod components;
mod pages;
mod utils;

use app::App;

fn main() {
    // 创建应用
    let app = RelmApp::new("com.github.linn");
    app.run::<App>(());
}
