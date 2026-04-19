// model.rs

#[derive(Debug, Clone)]
pub struct LyricChar {
    pub ch: String,
    pub start: u64,
    pub duration: u64,
}

/// 歌词行的内容类型
#[derive(Debug, Clone)]
pub enum LyricLineKind {
    /// 逐字模式（来自 yrc）
    Verbatim(Vec<LyricChar>),
    /// 整行模式（来自普通 lrc，无逐字信息）
    Plain,
}

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub start: u64,
    pub duration: u64,
    /// 完整文本，两种模式都有，渲染和配对都用这个
    pub text: String,
    pub kind: LyricLineKind,
    /// 配对后注入的翻译文本
    pub translation: Option<String>,
}

/// API 返回的原始歌词数据（未解析）
#[derive(Debug, Clone)]
pub struct Lyric {
    pub lyric: Option<String>,
    pub tlyric: Option<String>,
    pub is_pure_music: bool,
    pub yrc: Option<String>,
}