/// 歌曲数据模型
#[derive(Debug, Clone)]
pub struct Song {
    pub id: u64,
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: u64, // 毫秒
    pub size: Option<u64>, // 字节（可选）
    pub copyright_id: u64,
    pub cover_url: String,
}

/// 歌手信息
#[derive(Debug, Clone)]
pub struct Artist {
    pub id: u64,
    pub name: String,
}

/// 专辑信息
#[derive(Debug, Clone)]
pub struct Album {
    pub id: u64,
    pub name: String,
}

/// 歌单详情（包含歌曲列表）
#[derive(Debug, Clone)]
pub struct PlaylistDetail {
    pub id: u64,
    pub name: String,
    pub cover_url: String,
    pub description: String,
    pub songs: Vec<Song>,
}

impl Song {
    /// 获取歌手名称字符串（多个歌手用 "/" 连接）
    pub fn artists_string(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<&str>>()
            .join(" / ")
    }

    /// 格式化时长（毫秒转 mm:ss）
    pub fn format_duration(&self) -> String {
        let total_seconds = self.duration / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// 格式化文件大小
    pub fn format_size(&self) -> Option<String> {
        self.size.map(|bytes| {
            if bytes < 1024 {
                format!("{} B", bytes)
            } else if bytes < 1024 * 1024 {
                format!("{:.1} KB", bytes as f64 / 1024.0)
            } else {
                format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artists_string() {
        let song = Song {
            id: 1,
            name: "Test Song".to_string(),
            artists: vec![
                Artist { id: 1, name: "Artist A".to_string() },
                Artist { id: 2, name: "Artist B".to_string() },
            ],
            album: Album { id: 1, name: "Test Album".to_string() },
            duration: 180000,
            size: None,
            copyright_id: 0,
            cover_url: String::new(),
        };

        assert_eq!(song.artists_string(), "Artist A / Artist B");
    }

    #[test]
    fn test_format_duration() {
        let song = Song {
            id: 1,
            name: "Test Song".to_string(),
            artists: vec![Artist { id: 1, name: "Artist".to_string() }],
            album: Album { id: 1, name: "Album".to_string() },
            duration: 185000, // 3:05
            size: None,
            copyright_id: 0,
            cover_url: String::new(),
        };

        assert_eq!(song.format_duration(), "03:05");
    }

    #[test]
    fn test_format_size() {
        let song = Song {
            id: 1,
            name: "Test Song".to_string(),
            artists: vec![Artist { id: 1, name: "Artist".to_string() }],
            album: Album { id: 1, name: "Album".to_string() },
            duration: 180000,
            size: Some(1024 * 1024 * 5), // 5 MB
            copyright_id: 0,
            cover_url: String::new(),
        };

        assert_eq!(song.format_size(), Some("5.0 MB".to_string()));
    }
}
