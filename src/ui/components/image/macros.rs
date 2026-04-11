// macros.rs

/// 快速构建 AsyncImage
///
/// # 示例
/// ```
/// // 最简单：只传 URL
/// let img = async_image!("https://example.com/cover.jpg");
///
/// // 指定尺寸
/// let img = async_image!("https://...", size: 120x120);
///
/// // 带圆角
/// let img = async_image!("https://...", size: 80x80, radius: Lg);
///
/// // 带自定义占位图
/// let img = async_image!("https://...",
///     size: 200x200,
///     radius: Full,
///     placeholder: icon("avatar-default-symbolic", 48),
/// );
/// ```
/// 快速构建 AsyncImage
///
/// # 示例
/// ```
/// // 最简单：只传 URL
/// let img = async_image!("https://example.com/cover.jpg");
///
/// // 指定尺寸 (使用元组语法)
/// let img = async_image!("https://...", size: (120, 120));
///
/// // 带圆角
/// let img = async_image!("https://...", size: (80, 80), radius: Lg);
/// ```
#[macro_export]
macro_rules! async_image {
    // 仅 URL
    ($url:expr $(,)?) => {
        $crate::ui::components::AsyncImageBuilder::new().src($url).build()
    };

    // URL + 尺寸 (改为元组语法更不容易出错)
    ($url:expr, size: ($w:expr, $h:expr) $(,)?) => {
        $crate::ui::components::AsyncImageBuilder::new()
            .src($url)
            .size($w, $h)
            .build()
    };

    // URL + 尺寸 + 圆角
    ($url:expr, size: ($w:expr, $h:expr), radius: $r:ident $(,)?) => {
        $crate::ui::components::AsyncImageBuilder::new()
            .src($url)
            .size($w, $h)
            .radius($crate::ui::components::BorderRadius::$r)
            .build()
    };

    // URL + 尺寸 + 圆角 + 占位 icon
    ($url:expr,
     size: ($w:expr, $h:expr),
     radius: $r:ident,
     placeholder: icon($icon:expr, $sz:expr)
     $(,)?
    ) => {
        $crate::ui::components::AsyncImageBuilder::new()
            .src($url)
            .size($w, $h)
            .radius($crate::ui::components::BorderRadius::$r)
            .placeholder($crate::ui::components::ImageSource::Icon {
                name: $icon.to_string(),
                size: $sz,
            })
            .build()
    };
}