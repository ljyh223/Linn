/// 歌单数据模型
#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub creator: String,
}
