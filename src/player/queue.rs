use std::collections::HashMap;
use crate::api::Song;

const PRELOAD_SIZE: usize = 50;

pub(crate) enum QueueItem {
    Full(Song),
    Id(u64),
    Loading(u64),
}

pub(crate) struct QueueManager {
    items: Vec<QueueItem>,
    pub current_index: Option<usize>,
}

impl QueueManager {
    pub fn new() -> Self {
        Self { items: Vec::new(), current_index: None }
    }

    pub fn load(&mut self, full_ids: Vec<u64>, tracks: Vec<Song>, start_index: usize) {
        let mut map: HashMap<u64, Song> = tracks.into_iter().map(|s| (s.id, s)).collect();
        self.items = full_ids
            .into_iter()
            .map(|id| map.remove(&id).map(QueueItem::Full).unwrap_or(QueueItem::Id(id)))
            .collect();
        self.current_index = Some(start_index);
    }

    /// 当前曲目。返回 None 说明队列空或索引越界。
    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|i| self.items.get(i))
    }

    pub fn advance(&mut self) -> bool {
        if let Some(i) = self.current_index {
            if i + 1 < self.items.len() {
                self.current_index = Some(i + 1);
                return true;
            }
        }
        false
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(i) = self.current_index {
            if i > 0 {
                self.current_index = Some(i - 1);
                return true;
            }
        }
        false
    }

    /// 把 Full(song) 拼回队列，返回是否命中当前曲目。
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

    /// 返回需要预加载的 id 列表，并把对应项标记为 Loading。
    pub fn take_preload_ids(&mut self) -> Vec<u64> {
        let start = self.current_index.map(|i| i + 1).unwrap_or(0);
        let end = (start + PRELOAD_SIZE).min(self.items.len());
        let mut ids = Vec::new();
        for item in &mut self.items[start..end] {
            if let QueueItem::Id(id) = item {
                ids.push(*id);
                *item = QueueItem::Loading(*id);
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
}