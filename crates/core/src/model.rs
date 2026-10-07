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
    /// A web page saved as one offline .html file (styles and images inside).
    Page,
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

/// An http(s) address. The only kind of picture link taken from outside (a page, the extension,
/// yt-dlp): a `file:` or `\\host\share` one would have Snag read local files, or hand the
/// Windows login to a network share.
pub fn is_web_link(url: &str) -> bool {
    let scheme = url.split_once("://").map(|(s, _)| s);
    scheme.is_some_and(|s| s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("https"))
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
    /// Videos: a preview image URL (http(s) only, see `is_web_link`).
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
    /// Look for a newer Snag on GitHub (at start, then daily) and offer it.
    pub check_updates: bool,
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
    /// The user's own free VirusTotal API key; empty = no safety checks (nothing is sent).
    pub virustotal_key: String,
    /// After-download rules, tried in order; the first that fits runs.
    pub rules: Vec<crate::rules::Rule>,
    /// Dark, light, or whatever Windows uses for apps.
    pub theme: ThemeMode,
    /// Mica (acrylic on Windows 10) behind the window, with slightly see-through panels.
    pub translucent: bool,
    /// The first-run tour was finished or skipped (or this install predates it).
    pub tour_done: bool,
    /// Draw the window with the graphics card instead of the CPU (more memory; takes effect at
    /// the next start).
    pub use_gpu: bool,
    /// The phone page's own code (in its link and QR code): the extension's code never travels
    /// over the home network. Settings from before it existed get one when loaded.
    pub phone_token: String,
}

/// The window's colours: Settings → Appearance → Theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    /// Follows Windows' "Choose your app mode".
    System,
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
            check_updates: true,
            subtitles: false,
            subtitle_langs: "en.*".into(),
            auto_retry: true,
            keep_sharing: false,
            phone_sharing: false,
            last_seen_version: String::new(),
            virustotal_key: String::new(),
            rules: Vec::new(),
            theme: ThemeMode::Dark,
            translucent: false,
            tour_done: false,
            use_gpu: false,
            phone_token: String::new(),
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
    /// VirusTotal's verdict on downloaded programs (see `safety`).
    pub safety: std::collections::BTreeMap<ItemId, crate::safety::Safety>,
    /// Downloaded per local day ("2026-10-07"), for the stats screen.
    pub daily: std::collections::BTreeMap<String, DayTotal>,
}

/// One day's downloading: bytes received and the time spent receiving them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DayTotal {
    pub bytes: u64,
    /// Milliseconds during which something was downloading.
    pub ms: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
            queues: vec![Queue { id: 0, name: "Main".into(), max_concurrent: usize::MAX, schedule: None }],
            settings: Settings::default(),
            watches: Vec::new(),
            safety: Default::default(),
            daily: Default::default(),
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

    /// Adds to `day`'s total (a day with nothing downloaded gets no entry).
    pub fn count_download(&mut self, day: &str, bytes: u64, ms: u64) {
        if bytes == 0 && ms == 0 {
            return;
        }
        let total = self.daily.entry(day.to_string()).or_default();
        total.bytes += bytes;
        total.ms += ms;
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

    #[test]
    fn downloaded_bytes_add_up_per_day() {
        let mut s = AppState::default();
        s.count_download("2026-10-07", 100, 250);
        s.count_download("2026-10-07", 50, 250);
        s.count_download("2026-10-08", 7, 0);
        assert_eq!(s.daily["2026-10-07"], DayTotal { bytes: 150, ms: 500 });
        assert_eq!(s.daily["2026-10-08"], DayTotal { bytes: 7, ms: 0 });
        s.count_download("2026-10-09", 0, 0);
        assert!(!s.daily.contains_key("2026-10-09"), "nothing downloaded: no entry");
    }

    #[test]
    fn state_from_before_the_day_counter_loads() {
        let s: AppState = serde_json::from_str(r#"{"next_id": 4}"#).unwrap();
        assert_eq!(s.next_id, 4);
        assert!(s.daily.is_empty());
        let round: AppState = serde_json::from_str(&serde_json::to_string(&AppState { daily: [("2026-10-07".to_string(), DayTotal { bytes: 9, ms: 1 })].into(), ..s }).unwrap()).unwrap();
        assert_eq!(round.daily["2026-10-07"].bytes, 9);
    }

    #[test]
    fn appearance_and_tour_settings_default_for_old_files() {
        // A settings block written before these fields existed.
        let old: Settings = serde_json::from_str(r##"{"accent":"#0a84ff","clipboard_watch":false}"##).unwrap();
        assert_eq!(old.theme, ThemeMode::Dark, "existing users keep the dark look");
        assert!(!old.translucent);
        assert!(!old.tour_done);
        assert!(old.check_updates, "updates are looked for unless switched off");
        assert_eq!(old.accent, "#0a84ff");
        let s = Settings { theme: ThemeMode::System, translucent: true, tour_done: true, ..Settings::default() };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""theme":"system""#), "{json}");
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
    }
}
