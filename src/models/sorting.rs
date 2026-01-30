/// 排序字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    Default,
    Title,
    Artist,
    Album,
    Duration,
    Size,
    CreateTime,
}

impl SortField {
    pub fn display_name(&self) -> &'static str {
        match self {
            SortField::Default => "默认",
            SortField::Title => "歌名",
            SortField::Artist => "歌手",
            SortField::Album => "专辑",
            SortField::Duration => "时长",
            SortField::Size => "大小",
            SortField::CreateTime => "创建时间",
        }
    }

    pub fn all_fields() -> [SortField; 7] {
        [
            SortField::Default,
            SortField::Title,
            SortField::Artist,
            SortField::Album,
            SortField::Duration,
            SortField::Size,
            SortField::CreateTime,
        ]
    }
}

/// 排序顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Default,
    Asc,
    Desc,
}

impl SortOrder {
    pub fn toggle(&self) -> Self {
        match self {
            SortOrder::Default => SortOrder::Asc,
            SortOrder::Asc => SortOrder::Desc,
            SortOrder::Desc => SortOrder::Default,
        }
    }

    pub fn arrow_symbol(&self) -> &'static str {
        match self {
            SortOrder::Default => "",
            SortOrder::Asc => " ↑",
            SortOrder::Desc => " ↓",
        }
    }
}
