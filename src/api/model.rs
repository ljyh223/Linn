use strum::Display;

#[derive(Debug)]
pub struct Playlist{
    pub id: i64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: i64,
    pub description: String,
    pub play_count: i64,
}

#[derive(Debug)]
pub struct PlaylistDetail {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: i64,
    pub description: String,
    pub play_count: i64,
    pub tracks: Vec<Song>,
}

#[derive(Debug)]
pub struct Song {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: i64,
}

#[derive(Debug)]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
}

#[derive(Debug)]
pub struct Album {
    pub id: i64,
    pub name: String,
    pub cover_url: String,
}

#[derive(Display, Clone, PartialEq)]
pub enum SoundQuality {
    // 播放音质等级, 分为 standard => 标准,higher => 较高, exhigh=>极高, lossless=>无损, hires=>Hi-Res, jyeffect => 高清环绕声, sky => 沉浸环绕声, dolby => 杜比全景声, jymaster => 超清母带
    #[strum(serialize = "standard")]
    Standard,
    #[strum(serialize = "higher")]
    Higher,
    #[strum(serialize = "exhigh")]
    ExHigh,
    #[strum(serialize = "lossless")]
    Lossless,
    #[strum(serialize = "hires")]
    HiRes,
    #[strum(serialize = "jyeffect")]
    Jyeffect,
    #[strum(serialize = "sky")]
    Sky,
    #[strum(serialize = "dolby")]
    Dolby,
    #[strum(serialize = "jymaster")]
    Jymaster,
}

