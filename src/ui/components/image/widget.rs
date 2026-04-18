
use relm4::gtk::glib::{self, Object};
use relm4::gtk::{Accessible, Buildable, ConstraintTarget, Widget};
use super::imp;

glib::wrapper! {
    pub struct AsyncImage(ObjectSubclass<imp::AsyncImage>)
        @extends Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}
impl AsyncImage {
    /// 标准构造器
    pub fn new() -> Self {
        Object::builder()
            .property("placeholder-icon", "image-loading-symbolic")
            .property("fallback-icon", "image-missing-symbolic")
            .build()
    }
}

impl Default for AsyncImage {
    fn default() -> Self {
        Self::new()
    }
}