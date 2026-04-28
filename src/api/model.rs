
use serde::{Deserialize, Serialize};
use strum::Display;


#[derive(Debug, Clone, Default, PartialEq)]
pub struct Playlist{
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub creator_name: String,
    pub creator_id: u64,
    pub description: String,
    pub play_count: u64,

}

impl Playlist {
    // 每日推荐：参数只有歌，因为其他信息都是写死的
    pub fn from_daily_recommend(songs: Vec<Song>) -> Self {
        Self {
            id: 0,
            name: "每日推荐".into(),
            cover_url: songs.first().map(|s| s.cover_url.clone()).unwrap_or_default(),
            creator_name: "网易云音乐".into(),
            creator_id: 0,
            description: "根据你的音乐口味生成, 每日6:00更新".into(),
            play_count: 0,
        }
    }

    // 歌手热门：除了歌，还需要传入动态的歌手信息
    pub fn from_artist_hot_songs(cover: String, artist_name: String, artist_id: u64) -> Self {
        Self {
            id: 0, // 或者生成一个特定的伪 ID
            name: format!("{} - 热门歌曲", artist_name.clone()),
            cover_url: cover,
            creator_name: artist_name.clone(),
            creator_id: artist_id,
            description: format!("{}最火的歌曲", artist_name),
            play_count: 0,
        }
    }
}


#[derive(Debug, Clone)]
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



#[derive(Debug, Clone, PartialEq, Default)]
pub struct Song {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Artist {
    pub id: u64,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Default)]
pub struct ArtistDetail {   
    pub id: u64,
    pub name: String,
    pub trans_name: String,
    pub avatar: String,
    pub description: String,
    pub identify_desc: String,
    pub alias_text: String,
    pub signature: String,
    pub brief_desc: String,

    pub music_size: u64,
    pub album_size: u64,
    pub mv_size: u64,
}

#[derive(Debug, Clone,PartialEq, Default)]
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
pub struct LyricDetail {
    pub lyric: Option<String>,
    pub tlyric: Option<String>,
    pub is_pure_music: bool,
    pub yrc: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct UserDetails {
    pub id: u64,
    pub name: String,
    pub avatar_url: String,
    pub follows: String,
    pub followeds: String,
    pub vip_type: String,
    pub level: String,

}


#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub avatar_url: String
}

#[derive(Debug, Clone, Default)]
pub struct Mv{
    pub id: u64,
    pub name: String,
    pub cover: String,
    pub duration: u64,
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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCounts {
    #[serde(rename = "programCount")]
    pub program_count: u32,
    
    #[serde(rename = "djRadioCount")]
    pub dj_radio_count: u32,
    
    #[serde(rename = "mvCount")]
    pub mv_count: u32,
    
    #[serde(rename = "artistCount")]
    pub artist_count: u32,
    
    #[serde(rename = "newProgramCount")]
    pub new_program_count: u32,
    
    #[serde(rename = "createDjRadioCount")]
    pub create_dj_radio_count: u32,
    
    #[serde(rename = "createdPlaylistCount")]
    pub created_playlist_count: u32,
    
    #[serde(rename = "subPlaylistCount")]
    pub sub_playlist_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Comment{
    pub id: u64,
    pub content: String,
    pub user: UserInfo,
    pub liked_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MusicComment{
    pub song_id: u64,
    pub hot_comments: Vec<Comment>,
    pub comments: Vec<Comment>,
}
