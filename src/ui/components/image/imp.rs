use relm4::gtk::glib::prelude::ObjectExt;
use relm4::gtk::{
    self, Image, Picture, Stack, gdk, prelude::WidgetExt, subclass::widget::WidgetImpl,
};
use relm4::gtk::{
    glib::{
        self, ParamSpec, Properties, Value,
        object::ObjectType,
        subclass::{
            object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
            types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        },
    },
    prelude::SnapshotExt,
    subclass::widget::WidgetImplExt,
};
use relm4::gtk::{graphene, gsk};
use std::cell::RefCell;
use tokio_util::sync::CancellationToken;

use super::image_manager::ImageManager;

#[derive(Default, Properties)]
#[properties(wrapper_type = super::widget::AsyncImage)]
pub struct AsyncImage {
    pub stack: Stack,
    pub loading_icon: Image,
    pub loaded_picture: Picture,
    pub error_icon: Image,

    #[property(get, set = Self::set_url)]
    pub url: RefCell<String>,

    #[property(get, set = Self::set_placeholder_icon)]
    pub placeholder_icon: RefCell<String>,

    #[property(get, set = Self::set_fallback_icon)]
    pub fallback_icon: RefCell<String>,

    #[property(get, set = Self::set_corner_radius)]
    pub corner_radius: RefCell<f32>,

    #[property(get, set)]
    pub shadow: RefCell<bool>,

    pub cancel_token: RefCell<Option<CancellationToken>>,

    // 持有 CssProvider，防止被 drop
    pub css_provider: RefCell<Option<gtk::CssProvider>>,
}

impl AsyncImage {
    fn set_url(&self, new_url: &str) {
        if *self.url.borrow() == new_url {
            return;
        }
        self.url.replace(new_url.to_string());

        if let Some(token) = self.cancel_token.borrow_mut().take() {
            token.cancel();
        }

        self.loaded_picture.set_paintable(gdk::Paintable::NONE);

        if new_url.is_empty() {
            self.stack.set_visible_child_name("loading");
            return;
        }

        let token = CancellationToken::new();
        *self.cancel_token.borrow_mut() = Some(token.clone());
        self.stack.set_visible_child_name("loading");
        let url_clone = new_url.to_string();

        let obj = self.obj().clone();
        glib::MainContext::default().spawn_local(async move {
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let token_clone = token.clone();

            tokio::spawn(async move {
                let res = ImageManager::global().fetch(url_clone, token_clone).await;
                let _ = sender.send(res);
            });

            match receiver.await {
                Ok(Ok(bytes)) => {
                    if token.is_cancelled() {
                        return;
                    }
                    let glib_bytes = glib::Bytes::from(&bytes);
                    if let Ok(texture) = gdk::Texture::from_bytes(&glib_bytes) {
                        obj.imp().loaded_picture.set_paintable(Some(&texture));
                        obj.imp().stack.set_visible_child_name("loaded");

                        let w = obj.width_request();
                        let h = obj.height_request();
                        obj.imp().stack.set_size_request(w, h);
                    } else {
                        obj.imp().stack.set_visible_child_name("error");
                    }
                }
                Ok(Err(super::image_manager::FetchError::Cancelled)) => {}
                _ => {
                    if !token.is_cancelled() {
                        obj.imp().stack.set_visible_child_name("error");
                    }
                }
            }
        });
    }

    fn set_placeholder_icon(&self, icon: String) {
        self.placeholder_icon.replace(icon.clone());
        self.loading_icon.set_icon_name(Some(&icon));
    }

    fn set_fallback_icon(&self, icon: String) {
        self.fallback_icon.replace(icon.clone());
        self.error_icon.set_icon_name(Some(&icon));
    }

    fn set_corner_radius(&self, radius: f32) {
        *self.corner_radius.borrow_mut() = radius;
        self.apply_corner_radius(radius);
    }

    fn apply_corner_radius(&self, radius: f32) {
        let obj = self.obj();
        self.stack.set_overflow(gtk::Overflow::Hidden);

        // 用指针地址作为唯一 ID，避免实例间冲突
        let id = obj.as_ptr() as usize;
        let class_name = format!("async-image-{id}");

        let css =
            format!(".{class_name} {{ border-radius: {radius}px; background: transparent; }}");

        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);

        // 替换旧 provider
        if let Some(old) = self.css_provider.borrow().as_ref() {
            gtk::style_context_remove_provider_for_display(&gdk::Display::default().unwrap(), old);
        }

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        *self.css_provider.borrow_mut() = Some(provider);

        // 确保 class 挂在 widget 上
        obj.add_css_class(&class_name);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for AsyncImage {
    const NAME: &'static str = "AsyncImageWidget";
    type Type = super::widget::AsyncImage;
    type ParentType = gtk::Widget;
}

impl ObjectImpl for AsyncImage {
    fn properties() -> &'static [ParamSpec] {
        Self::derived_properties()
    }
    fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
        self.derived_set_property(id, value, pspec)
    }
    fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
        self.derived_property(id, pspec)
    }

    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        self.stack
            .set_transition_type(gtk::StackTransitionType::Crossfade);
        self.stack.set_transition_duration(300);

        self.loading_icon.set_pixel_size(32);
        self.error_icon.set_pixel_size(32);
        self.loaded_picture.set_content_fit(gtk::ContentFit::Cover);

        self.stack.add_named(&self.loading_icon, Some("loading"));
        self.stack.add_named(&self.loaded_picture, Some("loaded"));
        self.stack.add_named(&self.error_icon, Some("error"));

        self.stack.set_parent(&*obj);
    }

    fn dispose(&self) {
        if let Some(token) = self.cancel_token.borrow_mut().take() {
            token.cancel();
        }
        // 清理全局 CSS provider
        if let Some(provider) = self.css_provider.borrow().as_ref() {
            gtk::style_context_remove_provider_for_display(
                &gdk::Display::default().unwrap(),
                provider,
            );
        }
        self.stack.unparent();
    }
}

impl WidgetImpl for AsyncImage {
    fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let obj = self.obj();
        let w = obj.width_request();
        let h = obj.height_request();

        match orientation {
            gtk::Orientation::Horizontal => {
                if w > 0 {
                    return (w, w, -1, -1);
                }
            }
            _ => {
                if h > 0 {
                    return (h, h, -1, -1);
                } else if w > 0 {
                    return (w, w, -1, -1);
                }
            }
        }

        self.parent_measure(orientation, for_size)
    }

    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.stack.allocate(width, height, baseline, None);
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let w = obj.width() as f32;
        let h = obj.height() as f32;
        let radius = *self.corner_radius.borrow();

        if *self.shadow.borrow() && self.stack.visible_child_name().as_deref() == Some("loaded") {
            let blur_radius = 18.0f64;
            let y_offset = 10.0f32;
            let shadow_alpha = 0.30f32;
            let expand = blur_radius as f32;

            // 1. 开启模糊层
            snapshot.push_blur(blur_radius);

            // 2. 在模糊层内画实心圆角矩形作为阴影源
            //    向下偏移 y_offset，四周扩展 expand 让模糊有扩散空间
            let shadow_rect = graphene::Rect::new(
                -expand,
                y_offset - expand,
                w + expand * 2.0,
                h + expand * 2.0,
            );
            let shadow_rounded = gsk::RoundedRect::from_rect(shadow_rect, radius);
            snapshot.push_rounded_clip(&shadow_rounded);
            snapshot.append_color(&gdk::RGBA::new(0.0, 0.0, 0.0, shadow_alpha), &shadow_rect);
            snapshot.pop(); // pop rounded_clip

            snapshot.pop(); // pop blur

            // 3. 再叠一层更近更小的阴影增加立体感（可选）
            let blur_radius2 = 6.0f64;
            let expand2 = blur_radius2 as f32;
            snapshot.push_blur(blur_radius2);
            let shadow_rect2 = graphene::Rect::new(
                -expand2,
                y_offset * 0.5 - expand2,
                w + expand2 * 2.0,
                h + expand2 * 2.0,
            );
            let shadow_rounded2 = gsk::RoundedRect::from_rect(shadow_rect2, radius);
            snapshot.push_rounded_clip(&shadow_rounded2);
            snapshot.append_color(
                &gdk::RGBA::new(0.0, 0.0, 0.0, shadow_alpha * 0.35),
                &shadow_rect2,
            );
            snapshot.pop();
            snapshot.pop();
        }

        // 子节点圆角裁剪
        if radius > 0.0 {
            let rect = graphene::Rect::new(0.0, 0.0, w, h);
            let rounded = gsk::RoundedRect::from_rect(rect, radius);
            snapshot.push_rounded_clip(&rounded);
            self.parent_snapshot(snapshot);
            snapshot.pop();
        } else {
            self.parent_snapshot(snapshot);
        }
    }
}
