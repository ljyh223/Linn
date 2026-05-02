//! GL 不可用时的降级方案
//!
//! 下载封面 → 作为 gtk::Picture 显示

use relm4::gtk;
use relm4::gtk::prelude::*;

/// 创建模糊背景降级组件
pub fn create_fallback_bg() -> gtk::Picture {
    let picture = gtk::Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_can_shrink(true);
    picture.set_size_request(100, 100);
    picture
}

/// 更新降级背景的封面
pub fn update_fallback_bg(picture: &gtk::Picture, cover_url: &str) {
    let url = format!("{}?param=200y200", cover_url);
    let picture = picture.clone();

    gtk::glib::spawn_future_local(async move {
        match reqwest::get(&url).await {
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    let glib_bytes = gtk::glib::Bytes::from(&bytes[..]);
                    match gtk::gdk::Texture::from_bytes(&glib_bytes) {
                        Ok(texture) => {
                            picture.set_paintable(Some(&texture));
                        }
                        Err(e) => {
                            log::error!("Failed to create texture: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to download cover: {}", e);
            }
        }
    });
}

/// 检测 GLArea 是否可用
pub fn check_gl_available(area: &gtk::GLArea) -> bool {
    area.error().is_none()
}
