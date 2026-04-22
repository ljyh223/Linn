use relm4::gtk::{glib::{
    self, ParamSpec, Properties, Value,
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
    },
}, graphene, gsk, prelude::SnapshotExt, subclass::widget::WidgetImplExt};
use relm4::gtk::{
    self, Image, Picture, Stack, gdk, prelude::WidgetExt, subclass::widget::WidgetImpl,
};

use relm4::gtk::glib::prelude::ObjectExt;
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

    #[property(get, set)]
    pub corner_radius: RefCell<f32>,

    pub cancel_token: RefCell<Option<CancellationToken>>,
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
        // self.loaded_picture.set_can_shrink(true); 
        // self.loaded_picture.set_size_request(0, 0);
        // self.loaded_picture.set_halign(gtk::Align::Fill);
        // self.loaded_picture.set_valign(gtk::Align::Fill);
        // self.loaded_picture.set_hexpand(false);
        // self.loaded_picture.set_vexpand(false);

        self.stack.add_named(&self.loading_icon, Some("loading"));
        self.stack.add_named(&self.loaded_picture, Some("loaded"));
        self.stack.add_named(&self.error_icon, Some("error"));

        self.stack.set_parent(&*obj);
    }

    fn dispose(&self) {
        if let Some(token) = self.cancel_token.borrow_mut().take() {
            token.cancel();
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
                    // 没设 height 就用 width，强制正方形
                    return (w, w, -1, -1);
                }
            }
        }
        
        self.parent_measure(orientation, for_size)
    }

    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        // 直接用父级给的值，不要强制覆盖
        self.stack.allocate(width, height, baseline, None);
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let radius = *self.corner_radius.borrow();
        
        if radius > 0.0 {
            let width = obj.width() as f32;
            let height = obj.height() as f32;
            
            let rounded_rect = gsk::RoundedRect::new(
                graphene::Rect::new(0.0, 0.0, width, height),
                graphene::Size::new(radius, radius),
                graphene::Size::new(radius, radius),
                graphene::Size::new(radius, radius),
                graphene::Size::new(radius, radius),
            );
            
            snapshot.push_rounded_clip(&rounded_rect);
            self.parent_snapshot(snapshot);
            snapshot.pop();
        } else {
            self.parent_snapshot(snapshot);
        }
    }
}
