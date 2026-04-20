use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::APP_NAME;

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
pub struct LyricDetail {
    pub lyric: Option<String>,
    pub tlyric: Option<String>,
    pub is_pure_music: bool,
    pub yrc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub avatar_url: String,
}

impl UserInfo {
    fn get_file_path() -> PathBuf {
        // 通常存放在系统的配置目录下，比如 Linux 的 ~/.config/yourapp/
        let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.join(APP_NAME).join("user.json")
    }

    /// 启动时调用：从磁盘读取，如果没有则返回 None
    pub fn load_from_disk() -> Option<Self> {
        let path = Self::get_file_path();
        if path.exists() {
            let content = fs::read_to_string(path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// 登录成功或修改信息时调用：写入磁盘
    pub fn save_to_disk(&self) {
        let path = Self::get_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok(); // 确保目录存在
        }
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json).ok();
    }

    /// 退出登录时调用：清空磁盘缓存
    pub fn clear_disk() {
        let path = Self::get_file_path();
        fs::remove_file(path).ok();
    }
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