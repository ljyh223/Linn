/// 图片尺寸等级，用于优化网易云音乐的图片加载速度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSize {
    /// 极小缩略图 (100x100) - 用于列表项等小图标
    Thumbnail,
    /// 小尺寸 (200x200) - 用于卡片封面
    Small,
    /// 中等尺寸 (400x400) - 用于详情页封面
    Medium,
    /// 大尺寸 (800x800) - 用于全屏显示
    Large,
    /// 原始尺寸 - 不添加参数
    Original,
}

impl ImageSize {
    /// 获取尺寸的宽高值
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match self {
            ImageSize::Thumbnail => Some((100, 100)),
            ImageSize::Small => Some((200, 200)),
            ImageSize::Medium => Some((400, 400)),
            ImageSize::Large => Some((800, 800)),
            ImageSize::Original => None,
        }
    }

    /// 获取尺寸参数字符串
    pub fn param_string(&self) -> Option<String> {
        self.dimensions().map(|(w, h)| format!("{}x{}", w, h))
    }

    /// 为网易云音乐的图片URL添加尺寸参数
    ///
    /// # 示例
    /// ```
    /// let url = "http://example.com/image.jpg";
    /// let sized_url = ImageSize::Small.apply_to_url(url);
    /// // 结果: "http://example.com/image.jpg?param=200x200"
    /// ```
    pub fn apply_to_url(&self, url: &str) -> String {
        if let Some(param) = self.param_string() {
            // 检查URL是否已经有参数
            if url.contains('?') {
                format!("{}&param={}", url, param)
            } else {
                format!("{}?param={}", url, param)
            }
        } else {
            url.to_string()
        }
    }

    /// 根据容器像素宽度推荐合适的图片尺寸
    ///
    /// 这可以帮助选择合适尺寸的图片以优化加载速度
    pub fn recommend_for_width(width: f32) -> Self {
        // 考虑到设备像素比（Retina屏幕通常是2x或3x）
        // 我们使用2倍作为基准
        let pixel_width = width * 2.0;

        match pixel_width {
            w if w <= 150.0 => ImageSize::Thumbnail,
            w if w <= 300.0 => ImageSize::Small,
            w if w <= 600.0 => ImageSize::Medium,
            w if w <= 1200.0 => ImageSize::Large,
            _ => ImageSize::Original,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_to_url() {
        let url = "http://p1.music.126.net/test.jpg";
        assert_eq!(
            ImageSize::Small.apply_to_url(url),
            "http://p1.music.126.net/test.jpg?param=200x200"
        );
    }

    #[test]
    fn test_apply_to_url_with_existing_params() {
        let url = "http://p1.music.126.net/test.jpg?foo=bar";
        assert_eq!(
            ImageSize::Small.apply_to_url(url),
            "http://p1.music.126.net/test.jpg?foo=bar&param=200x200"
        );
    }

    #[test]
    fn test_recommend_for_width() {
        // 卡片宽度 160px
        assert_eq!(ImageSize::recommend_for_width(160.0), ImageSize::Small);
        // 列表项宽度 50px
        assert_eq!(ImageSize::recommend_for_width(50.0), ImageSize::Thumbnail);
    }
}
