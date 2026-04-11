#[derive(Debug, Clone, PartialEq)]
pub enum AppRoute {
    Home,
    PlaylistDetail(u64), // 携带歌单 ID
}