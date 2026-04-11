// async_image_builder.rs

use relm4::gtk::{self, prelude::WidgetExt};

use super::async_image::{AsyncImage, BorderRadius, ImageSource};

pub struct AsyncImageBuilder {
    inner:       AsyncImage,
    src:         Option<String>,
    radius:      Option<BorderRadius>,
    placeholder: Option<ImageSource>,
    error_view:  Option<ImageSource>,
}

impl AsyncImageBuilder {
    pub fn new() -> Self {
        Self {
            inner:       AsyncImage::new(),
            src:         None,
            radius:      None,
            placeholder: None,
            error_view:  None,
        }
    }

    // ── 内容 ──────────────────────────────────────────────────────────────

    pub fn src(mut self, url: impl Into<String>) -> Self {
        self.src = Some(url.into());
        self
    }

    pub fn placeholder(mut self, source: ImageSource) -> Self {
        self.placeholder = Some(source);
        self
    }

    pub fn error(mut self, source: ImageSource) -> Self {
        self.error_view = Some(source);
        self
    }

    // ── 尺寸 ──────────────────────────────────────────────────────────────

    pub fn size(self, width: i32, height: i32) -> Self {
        self.inner.set_size(width, height);
        self
    }

    pub fn width(self, w: i32) -> Self {
        self.inner.stack.set_width_request(w);
        self
    }

    pub fn height(self, h: i32) -> Self {
        self.inner.stack.set_height_request(h);
        self
    }

    // ── 样式 ──────────────────────────────────────────────────────────────

    pub fn radius(mut self, r: BorderRadius) -> Self {
        self.radius = Some(r);
        self
    }

    // pub fn content_fit(self, fit: ContentFit) -> Self {
    //     self.inner.set_content_fit(fit);
    //     self
    // }

    pub fn halign(self, align: gtk::Align) -> Self {
        self.inner.set_halign(align);
        self
    }

    pub fn valign(self, align: gtk::Align) -> Self {
        self.inner.set_valign(align);
        self
    }

    pub fn hexpand(self, v: bool) -> Self {
        self.inner.set_hexpand(v);
        self
    }

    pub fn vexpand(self, v: bool) -> Self {
        self.inner.set_vexpand(v);
        self
    }

    // ── 构建 ──────────────────────────────────────────────────────────────

    pub fn build(self) -> AsyncImage {
        // 先设置占位/错误视图，再触发加载（避免 set_src 期间视图缺失）
        if let Some(p) = self.placeholder {
            self.inner.set_placeholder(p);
        }
        if let Some(e) = self.error_view {
            self.inner.set_error_view(e);
        }
        if let Some(r) = self.radius {
            self.inner.set_border_radius(r);
        }
        if let Some(url) = self.src {
            self.inner.set_src(url);
        }
        self.inner
    }
}

impl Default for AsyncImageBuilder {
    fn default() -> Self {
        Self::new()
    }
}