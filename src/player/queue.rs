use std::{collections::HashMap, sync::Arc};
use rand::seq::SliceRandom;
use crate::api::{Playlist, Song};
use super::messages::PlayMode;

const PRELOAD_SIZE: usize = 50;

pub(crate) enum QueueItem {
    Full(Song),
    Id(u64),
    Loading(u64),
}

pub(crate) struct QueueManager {
    items: Vec<QueueItem>,
    pub current_index: Option<usize>,
    pub current_playlist: Option<Playlist>,
    play_mode: PlayMode,
    loop_enabled: bool,
    play_order: Vec<usize>,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current_index: None,
            current_playlist: None,
            play_mode: PlayMode::Sequential,
            loop_enabled: true,
            play_order: Vec::new(),
        }
    }

    pub fn load(&mut self, track_ids: Arc<Vec<u64>>, tracks: Arc<Vec<Song>>, playlist: Playlist, start_index: usize) {
        let mut map: HashMap<u64, Song> = tracks.iter().map(|s| (s.id, s.clone())).collect();

        self.items = track_ids
            .iter()
            .map(|&id| map.remove(&id).map(QueueItem::Full).unwrap_or(QueueItem::Id(id)))
            .collect();

        self.current_index = Some(start_index);
        self.current_playlist = Some(playlist);
        self.rebuild_play_order();
    }

    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|i| self.items.get(i))
    }

    pub fn advance(&mut self, auto: bool) -> bool {
        let ci = match self.current_index {
            Some(i) => i,
            None => return false,
        };
        if self.items.is_empty() || self.play_order.is_empty() {
            return false;
        }

        if auto && matches!(self.play_mode, PlayMode::SingleLoop) {
            return true;
        }

        if let Some(pos) = self.play_order.iter().position(|&i| i == ci) {
            if pos + 1 < self.play_order.len() {
                self.current_index = Some(self.play_order[pos + 1]);
                true
            } else if self.loop_enabled {
                self.current_index = Some(self.play_order[0]);
                true
            } else if matches!(self.items.get(ci), Some(QueueItem::Id(_)) | Some(QueueItem::Loading(_))) {
                self.current_index = Some(self.play_order[pos]);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn go_back(&mut self) -> bool {
        let ci = match self.current_index {
            Some(i) => i,
            None => return false,
        };
        if self.items.is_empty() || self.play_order.is_empty() {
            return false;
        }

        if let Some(pos) = self.play_order.iter().position(|&i| i == ci) {
            if pos > 0 {
                self.current_index = Some(self.play_order[pos - 1]);
                true
            } else if self.loop_enabled {
                let last = *self.play_order.last().unwrap();
                self.current_index = Some(last);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn apply_fetched(&mut self, songs: Vec<Song>) -> bool {
        let current_id = self.current_waiting_id();
        let mut hit_current = false;

        for song in songs {
            if Some(song.id) == current_id {
                hit_current = true;
            }
            if let Some(pos) = self.items.iter().position(|item| {
                matches!(item, QueueItem::Loading(id) | QueueItem::Id(id) if *id == song.id)
            }) {
                self.items[pos] = QueueItem::Full(song);
            }
        }
        hit_current
    }

    pub fn take_preload_ids(&mut self) -> Vec<u64> {
        if self.play_mode == PlayMode::SingleLoop {
            return Vec::new();
        }

        let ci = match self.current_index {
            Some(i) => i,
            None => return Vec::new(),
        };

        if self.play_order.is_empty() {
            return Vec::new();
        }

        let pos = match self.play_order.iter().position(|&i| i == ci) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let start = pos + 1;
        let end = (start + PRELOAD_SIZE).min(self.play_order.len());
        let mut ids = Vec::new();

        for &idx in &self.play_order[start..end] {
            if let QueueItem::Id(id) = &self.items[idx] {
                ids.push(*id);
                self.items[idx] = QueueItem::Loading(*id);
            }
        }
        ids
    }

    fn current_waiting_id(&self) -> Option<u64> {
        self.current_index.and_then(|i| self.items.get(i)).and_then(|item| match item {
            QueueItem::Loading(id) | QueueItem::Id(id) => Some(*id),
            _ => None,
        })
    }

    pub fn find_by_id(&self, song_id: u64) -> Option<Song> {
        self.items.iter().find_map(|item| {
            if let QueueItem::Full(s) = item {
                if s.id == song_id { return Some(s.clone()); }
            }
            None
        })
    }

    pub fn remove(&mut self, index: usize) {
        eprintln!("QueueManager: 删除指定歌曲 remove index {}", index);
        if index < self.items.len() {
            self.items.remove(index);

            match self.play_mode {
                PlayMode::Shuffle => {
                    self.play_order.retain(|&i| i != index);
                    for i in &mut self.play_order {
                        if *i > index {
                            *i -= 1;
                        }
                    }
                }
                _ => {
                    self.play_order = (0..self.items.len()).collect();
                }
            }

            if let Some(current) = self.current_index {
                if index < current {
                    self.current_index = Some(current - 1);
                } else if index == current {
                    if current >= self.items.len() {
                        self.current_index = Some(self.items.len().saturating_sub(1));
                    }
                }
            }
        }
    }

    pub fn play(&mut self, index: usize) {
        if index < self.items.len() {
            eprintln!("QueueManager: 播放指定歌曲 play index {}", index);
            self.current_index = Some(index);
        }
    }

    pub fn set_play_mode(&mut self, mode: PlayMode) {
        if self.play_mode == mode {
            return;
        }
        self.play_mode = mode;
        self.rebuild_play_order();
    }

    pub fn set_loop_enabled(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
    }

    pub fn get_queue(&self) -> Arc<Vec<Song>> {
        Arc::new(self.items.iter().filter_map(|item| {
            if let QueueItem::Full(s) = item {
                Some(s.clone())
            } else {
                None
            }
        }).collect())
    }

    fn rebuild_play_order(&mut self) {
        self.play_order = (0..self.items.len()).collect();
        if self.play_mode == PlayMode::Shuffle {
            self.shuffle_around_current();
        }
    }

    fn shuffle_around_current(&mut self) {
        let ci = match self.current_index {
            Some(i) if i < self.items.len() => i,
            _ => {
                self.play_order.shuffle(&mut rand::thread_rng());
                return;
            }
        };

        let mut indices: Vec<usize> = (0..self.items.len()).collect();
        if let Some(cur_pos) = indices.iter().position(|&i| i == ci) {
            indices.remove(cur_pos);
            indices.shuffle(&mut rand::thread_rng());
            indices.insert(cur_pos, ci);
        }
        self.play_order = indices;
    }
}