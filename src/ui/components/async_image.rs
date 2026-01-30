use iced::widget::{container, image, text};
use iced::{Element, Length, Task};
use std::sync::Arc;

// Import ImageCache for integrated loading
use super::image_cache::ImageCache;
use crate::utils::ImageSize;

/// 图片填充模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFit {
    /// 填充整个容器，可能裁剪图片（类似 CSS object-fit: cover）
    Cover,
    /// 完整显示图片，可能留有空白（类似 CSS object-fit: contain）
    Contain,
    /// 拉伸图片以填充容器（类似 CSS object-fit: fill）
    Fill,
}

/// 异步图片加载状态
#[derive(Debug, Clone)]
pub enum AsyncImage {
    Loading,
    Loaded(image::Handle),
    Failed,
}

/// AsyncImage 构建器 - 实现链式调用 API
///
/// 使用示例:
/// ```rust
/// AsyncImage::loading()
///     .build()
///     .width(Length::Fixed(200.0))
///     .height(Length::Fixed(200.0))
///     .corner_radius(16.0)
///     .into()
/// ```
#[derive(Debug, Clone)]
pub struct AsyncImageBuilder {
    image: AsyncImage,
    width: Option<Length>,
    height: Option<Length>,
    fit: ImageFit,
    corner_radius: Option<f32>,
}

impl AsyncImage {
    /// 创建新的加载中图片
    pub fn loading() -> Self {
        Self::Loading
    }

    /// 创建已加载的图片
    pub fn loaded(handle: image::Handle) -> Self {
        Self::Loaded(handle)
    }

    /// 创建失败的图片
    pub fn failed() -> Self {
        Self::Failed
    }

    /// 检查是否正在加载
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// 检查是否已加载
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    /// 开始构建链式配置
    ///
    /// 返回一个 AsyncImageBuilder，可以使用链式调用配置各种属性
    pub fn build(self) -> AsyncImageBuilder {
        AsyncImageBuilder {
            image: self,
            width: None,
            height: None,
            fit: ImageFit::Cover,
            corner_radius: None,
        }
    }

    /// 渲染图片组件（带填充模式）- 消费 self
    pub fn view_with_fit(
        self,
        width: Length,
        height: Length,
        fit: ImageFit,
        corner_radius: Option<f32>,
    ) -> Element<'static, ()> {
        self.build()
            .width(width)
            .height(height)
            .fit(fit)
            .corner_radius_option(corner_radius)
            .into()
    }

    /// 渲染图片组件（默认使用 Cover 模式）- 消费 self
    pub fn view(self, width: Length, height: Length) -> Element<'static, ()> {
        self.build()
            .width(width)
            .height(height)
            .into()
    }

    /// 渲染图片组件（带圆角，默认使用 Cover 模式）- 消费 self
    pub fn view_rounded(
        self,
        width: Length,
        height: Length,
        corner_radius: f32,
    ) -> Element<'static, ()> {
        self.build()
            .width(width)
            .height(height)
            .corner_radius(corner_radius)
            .into()
    }
}

impl AsyncImageBuilder {
    /// 设置宽度
    pub fn width(mut self, width: Length) -> Self {
        self.width = Some(width);
        self
    }

    /// 设置高度
    pub fn height(mut self, height: Length) -> Self {
        self.height = Some(height);
        self
    }

    /// 设置图片填充模式
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// 设置圆角半径
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// 设置圆角半径（内部方法，接受 Option）
    fn corner_radius_option(mut self, radius: Option<f32>) -> Self {
        self.corner_radius = radius;
        self
    }

    /// 渲染为 Element
    pub fn view(self) -> Element<'static, ()> {
        self.build_view()
    }

    /// 渲染为 Element 并映射消息类型
    pub fn map_message<Message: 'static>(self, f: impl Fn(()) -> Message + 'static) -> Element<'static, Message> {
        self.build_view().map(move |_| f(()))
    }

    /// 内部构建方法
    fn build_view(self) -> Element<'static, ()> {
        // 使用默认值 if not specified
        let width = self.width.unwrap_or(Length::Fill);
        let height = self.height.unwrap_or(Length::Fill);

        match self.image {
            AsyncImage::Loaded(handle) => {
                let img = image(handle);

                match self.fit {
                    ImageFit::Cover => {
                        // Cover 模式：图片填满容器并居中裁剪
                        let mut c = container(img.width(Length::Fill).height(Length::Fill))
                            .width(width)
                            .height(height)
                            .clip(true);

                        // Apply radius if specified
                        if let Some(radius) = self.corner_radius {
                            c = c.style(move |_theme| container::Style {
                                border: iced::border::Border {
                                    radius: radius.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                        }

                        c.into()
                    }
                    ImageFit::Contain => {
                        // Contain 模式：完整显示图片
                        let mut c = container(img.width(Length::Fill).height(Length::Fill))
                            .width(width)
                            .height(height)
                            .align_x(iced::alignment::Horizontal::Center)
                            .align_y(iced::alignment::Vertical::Center)
                            .clip(true);

                        // Apply radius if specified
                        if let Some(radius) = self.corner_radius {
                            c = c.style(move |_theme| container::Style {
                                border: iced::border::Border {
                                    radius: radius.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                        }

                        c.into()
                    }
                    ImageFit::Fill => {
                        // Fill 模式：拉伸填充
                        // Note: Image widget doesn't support border radius directly
                        // For Fill mode with rounded corners, we wrap in container
                        if let Some(radius) = self.corner_radius {
                            container(img.width(Length::Fill).height(Length::Fill))
                                .width(width)
                                .height(height)
                                .clip(true)
                                .style(move |_theme| container::Style {
                                    border: iced::border::Border {
                                        radius: radius.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .into()
                        } else {
                            img.width(width).height(height).into()
                        }
                    }
                }
            }
            AsyncImage::Loading => {
                // 现代化的加载占位符
                let mut c = container(text("⏳"))
                    .width(width)
                    .height(height)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.1, 0.1, 0.12,
                        ))),
                        ..Default::default()
                    });

                // Apply radius if specified
                if let Some(radius) = self.corner_radius {
                    c = c
                        .clip(true)
                        .style(move |_theme| container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.1, 0.1, 0.12,
                            ))),
                            border: iced::border::Border {
                                radius: radius.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                }

                c.into()
            }
            AsyncImage::Failed => {
                // 现代化的失败占位符
                let mut c = container(text("❌"))
                    .width(width)
                    .height(height)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.12, 0.08, 0.08,
                        ))),
                        ..Default::default()
                    });

                // Apply radius if specified
                if let Some(radius) = self.corner_radius {
                    c = c
                        .clip(true)
                        .style(move |_theme| container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.12, 0.08, 0.08,
                            ))),
                            border: iced::border::Border {
                                radius: radius.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                }

                c.into()
            }
        }
    }
}

/// 实现 Into<Element> 让调用更自然
impl Into<Element<'static, ()>> for AsyncImageBuilder {
    fn into(self) -> Element<'static, ()> {
        self.view()
    }
}
