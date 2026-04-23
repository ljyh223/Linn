use std::{fs, path::PathBuf};

use crate::{APP_NAME, api::{AlbumDetail, Playlist, PlaylistDetail, Song, UserInfo}, ui::model::DetailView};


impl From<PlaylistDetail> for Playlist {
    fn from(detail: PlaylistDetail) -> Self {
        Playlist {
            id: detail.id,
            name: detail.name.clone(),
            cover_url: detail.cover_url.clone(),
            creator_name: detail.creator_name.clone(),
            creator_id: detail.creator_id,
            description: detail.description.clone(),
            play_count: detail.play_count,
        }
    }
}

impl From<PlaylistDetail> for DetailView {
    fn from(mut p: PlaylistDetail) -> Self {
        Self {
            id: p.id,
            cover_url: p.cover_url,
            name: p.name,
            creator: Some(p.creator_name),
            description: Some(p.description),
            tracks: std::mem::take(&mut p.tracks),
            track_ids: std::mem::take(&mut p.track_ids),
        }
    }
}

impl From<AlbumDetail> for DetailView {
    fn from(mut a: AlbumDetail) -> Self {
        let track_ids = a.tracks.iter().map(|t| t.id).collect();
        Self {
            id: a.id,
            cover_url: a.cover_url,
            name: a.name,
            creator: Some(a.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")),
            description: Some(a.description),
            tracks: std::mem::take(&mut a.tracks),
            track_ids,
        }
    }
}

impl From<Vec<Song>> for DetailView {
    fn from(songs: Vec<Song>) -> Self {
        let track_ids = songs.iter().map(|s| s.id).collect();
        Self {
            id: 0,
            cover_url: songs.first().map(|s| s.cover_url.clone()).unwrap_or_default(),
            name: "每日推荐".into(),
            creator: Some("网易云音乐".into()),
            description: Some("根据你的音乐口味生成, 每日6:00更新".into()),
            tracks: songs,
            track_ids,
        }
    }
}

impl From<DetailView> for Playlist {
    fn from(value: DetailView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            cover_url: value.cover_url,
            creator_name: value.creator.unwrap_or_default(),
            creator_id: 0,
            description: value.description.unwrap_or_default(),
            play_count: 0,
        }
    }
}


impl From<AlbumDetail> for Playlist {
    fn from(value: AlbumDetail) -> Self {
        Self {
            id: value.id,
            name: value.name,
            cover_url: value.cover_url,
            creator_name: value.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
            creator_id: 0,
            description: value.description,
            play_count: 0,
        }
    }
}

// impl From<Vec<Song>> for Playlist {
//     fn from(value: Vec<Song>) -> Self {
//         Self {
//             id: 0,
//             name: "每日推荐".into(),
//             cover_url: value.first().map(|s| s.cover_url.clone()).unwrap_or_default(),
//             creator_name: "网易云音乐".into(),
//             creator_id: 0,
//             description: "根据你的音乐口味生成, 每日6:00更新".into(),
//             play_count: 0,
//         }
//     }
// }



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