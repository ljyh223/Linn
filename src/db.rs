use std::collections::HashSet;
use std::path::PathBuf;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::player::messages::PlayMode;
use crate::{APP_NAME, api::Song};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectType {
    Playlist,
    Album,
}

/// 上次播放会话的快照，用于启动时恢复。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionState {
    pub track_ids: Vec<u64>,
    pub current_index: usize,
    pub playlist_id: u64,
    pub playlist_name: String,
    pub playlist_cover_url: String,
    pub playlist_creator_name: String,
    /// 启动时优先展示的当前歌曲快照；完整队列仍会在后台重新校验。
    #[serde(default)]
    pub current_song: Option<Song>,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    fn db_path() -> PathBuf {
        let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.join(APP_NAME).join("linn.db")
    }

    pub fn open() -> anyhow::Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS collected (
                item_id INTEGER NOT NULL,
                item_type TEXT NOT NULL,
                PRIMARY KEY (item_id, item_type)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS player_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn is_collected(&self, item_id: u64, item_type: CollectType) -> bool {
        let type_str = match item_type {
            CollectType::Playlist => "playlist",
            CollectType::Album => "album",
        };
        self.conn
            .query_row(
                "SELECT 1 FROM collected WHERE item_id = ?1 AND item_type = ?2",
                params![item_id as i64, type_str],
                |_| Ok(true),
            )
            .unwrap_or(false)
    }

    pub fn set_collected(&self, item_id: u64, item_type: CollectType, collected: bool) {
        let type_str = match item_type {
            CollectType::Playlist => "playlist",
            CollectType::Album => "album",
        };
        if collected {
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO collected (item_id, item_type) VALUES (?1, ?2)",
                params![item_id as i64, type_str],
            );
        } else {
            let _ = self.conn.execute(
                "DELETE FROM collected WHERE item_id = ?1 AND item_type = ?2",
                params![item_id as i64, type_str],
            );
        }
    }

    pub fn get_all_collected(&self, item_type: CollectType) -> HashSet<u64> {
        let type_str = match item_type {
            CollectType::Playlist => "playlist",
            CollectType::Album => "album",
        };
        let mut stmt = self
            .conn
            .prepare("SELECT item_id FROM collected WHERE item_type = ?1")
            .unwrap();
        let rows = stmt.query_map(params![type_str], |row| row.get::<_, i64>(0));
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).map(|id| id as u64).collect(),
            Err(_) => HashSet::new(),
        }
    }

    pub fn sync_collected(&self, item_type: CollectType, ids: &[u64]) {
        let type_str = match item_type {
            CollectType::Playlist => "playlist",
            CollectType::Album => "album",
        };
        let tx = self.conn.unchecked_transaction().unwrap();
        tx.execute(
            "DELETE FROM collected WHERE item_type = ?1",
            params![type_str],
        )
        .unwrap();
        for &id in ids {
            tx.execute(
                "INSERT OR IGNORE INTO collected (item_id, item_type) VALUES (?1, ?2)",
                params![id as i64, type_str],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    fn get_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM player_settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .ok()
    }

    fn set_setting(&self, key: &str, value: &str) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO player_settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        );
    }

    pub fn get_play_mode(&self) -> PlayMode {
        match self.get_setting("play_mode").as_deref() {
            Some("single_loop") => PlayMode::SingleLoop,
            Some("shuffle") => PlayMode::Shuffle,
            _ => PlayMode::Sequential,
        }
    }

    pub fn set_play_mode(&self, mode: PlayMode) {
        let value = match mode {
            PlayMode::Sequential => "sequential",
            PlayMode::SingleLoop => "single_loop",
            PlayMode::Shuffle => "shuffle",
        };
        self.set_setting("play_mode", value);
    }

    pub fn get_loop_enabled(&self) -> bool {
        self.get_setting("loop_enabled").as_deref() != Some("false")
    }

    pub fn set_loop_enabled(&self, enabled: bool) {
        self.set_setting("loop_enabled", if enabled { "true" } else { "false" });
    }

    pub fn save_session(&self, session: &SessionState) {
        if let Ok(json) = serde_json::to_string(session) {
            self.set_setting("last_session", &json);
        }
    }

    pub fn load_session(&self) -> Option<SessionState> {
        serde_json::from_str::<SessionState>(self.get_setting("last_session")?.as_str()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::SessionState;
    use crate::api::{Album, Artist, Song};

    #[test]
    fn session_song_snapshot_round_trips() {
        let session = SessionState {
            track_ids: vec![1, 2],
            current_index: 1,
            current_song: Some(Song {
                id: 2,
                name: "恢复测试".into(),
                cover_url: "https://example.test/cover.jpg".into(),
                artists: vec![Artist {
                    id: 3,
                    name: "歌手".into(),
                    avatar: None,
                }],
                album: Album {
                    id: 4,
                    name: "专辑".into(),
                    cover_url: String::new(),
                },
                duration: 123_000,
            }),
            ..Default::default()
        };

        let decoded: SessionState =
            serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();

        assert_eq!(decoded.current_song, session.current_song);
    }

    #[test]
    fn legacy_session_without_song_snapshot_still_loads() {
        let decoded: SessionState = serde_json::from_str(
            r#"{"track_ids":[1],"current_index":0,"playlist_id":0,"playlist_name":"","playlist_cover_url":"","playlist_creator_name":""}"#,
        )
        .unwrap();

        assert_eq!(decoded.track_ids, vec![1]);
        assert!(decoded.current_song.is_none());
    }
}
