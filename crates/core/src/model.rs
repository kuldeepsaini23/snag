use crate::category::Category;
use crate::schedule::Schedule;
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
