use crate::category::Category;
use crate::schedule::Schedule;
use rdm_media::MediaFormat;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemId(pub u64);

/// 0 is the always-active "Main" queue.
pub type QueueId = u32;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Queued,
    Running,
    Paused,
    Done,
    Failed(String),
}

/// How an item is downloaded.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// A direct file link, fetched by the segmented engine.
    #[default]
    Http,
    /// A video/audio page, fetched by yt-dlp in this format.
    Media(MediaFormat),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub url: String,
    pub name: String,
    pub category: Category,
    pub status: Status,
    pub dest: Option<PathBuf>,
    pub downloaded: u64,
    pub total: Option<u64>,
    #[serde(skip)]
    pub speed_bps: u64,
    pub queue: QueueId,
    /// Unix seconds.
    pub added: i64,
    #[serde(default)]
    pub kind: Kind,
    /// Page the download was started from (sent as Referer).
    #[serde(default)]
    pub referrer: Option<String>,
    /// Video downloads: yt-dlp's partial files live here until the final file is moved out.
    #[serde(default)]
    pub work_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Queue {
    pub id: QueueId,
    pub name: String,
    pub max_concurrent: usize,
    pub schedule: Option<Schedule>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub download_dir: PathBuf,
    pub sort_into_folders: bool,
    pub connections: usize,
    pub max_concurrent: usize,
    /// 0 = unlimited.
    pub speed_limit_bps: u64,
    pub accent: String,
    pub clipboard_watch: bool,
    pub start_immediately: bool,
    /// Shared secret the browser extension sends with every request.
    pub extension_token: String,
    /// Show the quality picker for every video; otherwise use `preferred_quality` straight away.
    pub ask_quality: bool,
    /// Pre-selected quality; `None` = the best available.
    pub preferred_quality: Option<MediaFormat>,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        Self {
            download_dir: home.join("Downloads").join("RDM"),
            sort_into_folders: true,
            connections: 8,
            max_concurrent: 3,
            speed_limit_bps: 0,
            accent: "#ff9f0a".into(),
            clipboard_watch: true,
            start_immediately: true,
            extension_token: String::new(),
            ask_quality: true,
            preferred_quality: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub next_id: u64,
    pub items: Vec<Item>,
    pub queues: Vec<Queue>,
    pub settings: Settings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
            queues: vec![Queue { id: 0, name: "Main".into(), max_concurrent: usize::MAX, schedule: None }],
            settings: Settings::default(),
        }
    }
}

/// A fresh pairing token: 32 hex characters.
pub fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    // RandomState is seeded from the OS per instance: two independent 64-bit halves.
    (0..2u64)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(i);
            h.write_u128(nanos);
            format!("{:016x}", h.finish())
        })
        .collect()
}

impl AppState {
    pub fn item(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    pub fn queue(&self, id: QueueId) -> Option<&Queue> {
        self.queues.iter().find(|q| q.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_32_hex_and_differ() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
