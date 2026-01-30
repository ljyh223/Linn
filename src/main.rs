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
            overflow: hidden;
            box-shadow: 0 4px 12px rgba(0,0,0,0.2);
            background-color: #333;
        }

        /* 圆形头像样式 */
        .avatar {
            border-radius: 999px;
            overflow: hidden;
        }

        /* 关键：强制固定卡片宽度，防止被 FlowBox 拉伸 */
        .playlist-card {
            min-width: 160px;
            max-width: 160px;
        }

        /* 确保容器严格140x140 */
        .playlist-card .playlist-cover {
            min-width: 140px;
            min-height: 140px;
            border-radius: 8px;
            overflow: hidden;
        }

        /* 真实图片：填充并保持比例（裁剪模式） */
        .playlist-cover image.real-image {
            min-width: 140px;
            min-height: 140px;
            object-fit: cover;
        }

        /* 占位图/错误图：居中显示，不拉伸 */
        .playlist-cover image.placeholder-style,
        .playlist-cover image.error-style {
            min-width: 48px;
            min-height: 48px;
            opacity: 0.6;
        }

        .playlist-cover {
            border-radius: 12px;
            overflow: hidden; /* 关键：裁剪掉溢出的图片 */
            background-color: alpha(currentColor, 0.1); /* 占位背景色 */
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
