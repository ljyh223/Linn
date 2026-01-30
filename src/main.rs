mod app;
mod pages;
mod ui;

use relm4::prelude::*;
use app::AppModel;
fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data("
        /* 播放列表封面样式 */
        .cover-art {
            border-radius: 12px;
            overflow: hidden; /* 关键：裁剪内容 */
            box-shadow: 0 4px 12px rgba(0,0,0,0.2);
            background-color: #333;
        }

        /* 圆形头像样式 */
        .avatar {
            border-radius: 999px;
            overflow: hidden;
        }
    ");

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn main() {
    
    let app = RelmApp::new("com.linn.music-player");
    app.run::<AppModel>(());
    // load_css();
}
