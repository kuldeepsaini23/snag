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
    /// A page of images, saved by gallery-dl into a folder of its own.
    Gallery,
    /// A magnet link or `.torrent` (librqbit).
    Torrent,
}

/// "pinterest.com · wallpapers · 3fa2": a gallery's name and folder. The most telling part of
/// the link plus a short hash of it, so two galleries never share a folder; safe as a Windows
/// file name and short enough for long download paths.
pub fn gallery_name(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let rest = rest.split(['?', '#']).next().unwrap_or(rest).trim_end_matches('/');
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    let host = parts.next().unwrap_or("gallery");
    let host = host.strip_prefix("www.").unwrap_or(host);
    // Skip generic tails like "photo/1": the post id or board name says more.
    const GENERIC: [&str; 9] = ["photo", "photos", "video", "status", "p", "pin", "gallery", "a", "post"];
    let segments: Vec<&str> = parts.collect();
    let telling = segments.iter().rev().find(|s| !GENERIC.contains(s) && !(s.len() <= 2 && s.chars().all(|c| c.is_ascii_digit())));
    let hash = rest.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3));
    let label: String = match telling {
        Some(t) => format!("{host} · {t}"),
        None => host.to_string(),
    };
    let label: String = label.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).take(70).collect();
    format!("{} · {:04x}", label.trim_end_matches(['.', ' ']), hash & 0xffff)
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
    /// Videos: a preview image URL.
    #[serde(default)]
    pub thumbnail: Option<String>,
    /// Videos: length in seconds.
    #[serde(default)]
    pub duration: Option<f64>,
    /// Failed for a temporary reason: tries again at this time (unix seconds). Not saved:
    /// after a restart the item is simply failed.
    #[serde(skip)]
    pub retry_at: Option<i64>,
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
    /// Windows notifications when downloads finish or fail.
    pub notify: bool,
    /// Videos: fetch subtitles and embed them in the file.
    pub subtitles: bool,
    /// Subtitle languages for yt-dlp (`en.*`, `en.*,hi`, `all`).
    pub subtitle_langs: String,
    /// Retry downloads that failed for a temporary reason (timeouts, busy servers).
    pub auto_retry: bool,
    /// Torrents: keep sharing with others after the download finishes.
    pub keep_sharing: bool,
    /// "Send from your phone": a page on the home network (off by default).
    pub phone_sharing: bool,
    /// The app version whose "What's new" was last seen ("" = before this was recorded).
    pub last_seen_version: String,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        Self {
            download_dir: home.join("Downloads").join("Snag"),
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
            notify: true,
            subtitles: false,
            subtitle_langs: "en.*".into(),
            auto_retry: true,
            keep_sharing: false,
            phone_sharing: false,
            last_seen_version: String::new(),
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
    /// Watched channels and playlists.
    pub watches: Vec<crate::watch::Watch>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
            queues: vec![Queue { id: 0, name: "Main".into(), max_concurrent: usize::MAX, schedule: None }],
            settings: Settings::default(),
            watches: Vec::new(),
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
