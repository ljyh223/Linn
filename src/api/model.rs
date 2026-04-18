use strum::Display;

#[derive(Debug)]
pub struct Playlist{
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: u64,
    pub description: String,
    pub play_count: u64,
}

#[derive(Debug)]
pub struct PlaylistDetail {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: u64,
    pub description: String,
    pub play_count: u64,
    pub tracks: Vec<Song>,
    pub track_ids: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct Song {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: u64,
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
}

#[derive(Debug, Clone)]
pub struct AlbumDetail {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub description: String,
    pub artists: Vec<Artist>,
    pub tracks: Vec<Song>,
}

#[derive(Debug, Clone)]
pub struct Lyric {
    pub lyric: Option<String>,
    pub tlyric: Option<String>,
    pub is_pure_music: bool,
    pub yrc: Option<String>,
    
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

