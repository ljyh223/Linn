use std::collections::HashSet;
use std::path::PathBuf;

use rusqlite::{Connection, params};

use crate::APP_NAME;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectType {
    Playlist,
    Album,
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
             PRAGMA synchronous=NORMAL;"
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS collected (
                item_id INTEGER NOT NULL,
                item_type TEXT NOT NULL,
                PRIMARY KEY (item_id, item_type)
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
}