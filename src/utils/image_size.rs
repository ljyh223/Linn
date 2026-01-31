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

    pub fn to_rank(&self) -> u32 {
        match self {
            ImageSize::Thumbnail => 0,
            ImageSize::Small => 1,
            ImageSize::Medium => 2,
            ImageSize::Large => 3,
            ImageSize::Original => 4,
        }
    }

    pub fn from_param(param: &str) -> Self {
        match param {
            "100x100" => ImageSize::Thumbnail,
            "200x200" => ImageSize::Small,
            "400x400" => ImageSize::Medium,
            "800x800" => ImageSize::Large,
            _ => ImageSize::Original,
        }
    }
}
