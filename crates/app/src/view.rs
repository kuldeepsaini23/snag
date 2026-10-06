//! What the Figma screens show, computed from the model: filters, counts, groups, labels.

use crate::format;
use crate::state::Model;
use rdm_core::{Category, Item, Kind, MediaFormat, Now, Queue, QueueId, Status};

/// The toolbar's filter pills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    /// Running, queued, paused or failed (but not waiting for a schedule).
    Active,
    Done,
    /// Queued in a queue whose schedule doesn't allow it right now.
    Scheduled,
}

pub const FILTERS: [Filter; 4] = [Filter::All, Filter::Active, Filter::Done, Filter::Scheduled];

/// The sidebar's selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Library {
    #[default]
    All,
    Category(Category),
    Queue(QueueId),
}

/// The sidebar's categories, in order.
pub const CATEGORIES: [Category; 6] = [Category::Video, Category::Music, Category::Image, Category::Archive, Category::Document, Category::Program];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counts {
    pub all: usize,
    pub active: usize,
    pub done: usize,
    pub scheduled: usize,
    categories: Vec<(Category, usize)>,
    queues: Vec<(QueueId, usize)>,
}

impl Counts {
    pub fn category(&self, c: Category) -> usize {
        self.categories.iter().find(|(k, _)| *k == c).map_or(0, |(_, n)| *n)
    }

    pub fn queue(&self, q: QueueId) -> usize {
        self.queues.iter().find(|(k, _)| *k == q).map_or(0, |(_, n)| *n)
    }

    pub fn of(&self, f: Filter) -> usize {
        match f {
            Filter::All => self.all,
            Filter::Active => self.active,
            Filter::Done => self.done,
            Filter::Scheduled => self.scheduled,
        }
    }
}

fn bump<K: PartialEq>(list: &mut Vec<(K, usize)>, key: K) {
    match list.iter_mut().find(|(k, _)| *k == key) {
        Some((_, n)) => *n += 1,
        None => list.push((key, 1)),
    }
}

impl Model {
    fn passes(&self, item: &Item, filter: Filter) -> bool {
        let waiting = self.waiting_for_schedule(item);
        match filter {
            Filter::All => true,
            Filter::Done => item.status == Status::Done,
            Filter::Scheduled => waiting,
            Filter::Active => item.status != Status::Done && !waiting,
        }
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts { all: self.items.len(), ..Counts::default() };
        for i in &self.items {
            c.active += self.passes(i, Filter::Active) as usize;
            c.done += (i.status == Status::Done) as usize;
            c.scheduled += self.passes(i, Filter::Scheduled) as usize;
            bump(&mut c.categories, i.category);
            bump(&mut c.queues, i.queue);
        }
        c
    }

    fn shown(&self, item: &Item) -> bool {
        let in_library = match self.library {
            Library::All => true,
            Library::Category(c) => item.category == c,
            Library::Queue(q) => item.queue == q,
        };
        let needle = self.search.trim().to_lowercase();
        in_library && self.passes(item, self.filter) && (needle.is_empty() || item.name.to_lowercase().contains(&needle))
    }

    /// (downloading, recent): what the list shows, newest first.
    pub fn visible(&self) -> (Vec<&Item>, Vec<&Item>) {
        self.items.iter().rev().filter(|i| self.shown(i)).partition(|i| i.status != Status::Done)
    }

    /// The selected item, if the list currently shows it.
    pub fn inspected(&self) -> Option<&Item> {
        let id = self.selected?;
        self.items.iter().find(|i| i.id == id).filter(|i| self.shown(i))
    }
}

impl Model {
    /// Thumbnails to fetch at `now`: shown by an item or the picker, not cached, not on their way,
    /// and not waiting to be tried again after a failure.
    pub fn missing_thumbs(&self, now: std::time::Instant) -> Vec<String> {
        let picker = self.picker.iter().flat_map(|p| p.info.thumbnail.iter().cloned().chain(p.info.entries.iter().filter_map(|e| e.thumbnail.clone())));
        let waiting = |url: &String| self.thumb_failures.get(url).is_some_and(|(n, at)| thumb_backoff(*n).is_none() || now < *at);
        let mut out: Vec<String> = Vec::new();
        for url in self.items.iter().filter_map(thumb_url).chain(picker) {
            if !self.thumbs.contains_key(&url) && !self.thumb_pending.contains(&url) && !waiting(&url) && !out.contains(&url) {
                out.push(url);
            }
        }
        out
    }

    /// A thumbnail arrived (`Some`) or couldn't be fetched: a failure is tried again later.
    pub fn thumb_ready(&mut self, url: String, path: Option<std::path::PathBuf>, now: std::time::Instant) {
        self.thumb_pending.remove(&url);
        match path {
            Some(path) => {
                self.thumb_failures.remove(&url);
                self.thumbs.insert(url, path);
            }
            None => {
                let failures = self.thumb_failures.get(&url).map_or(0, |(n, _)| *n) + 1;
                let at = now + thumb_backoff(failures).unwrap_or_default();
                self.thumb_failures.insert(url, (failures, at));
            }
        }
    }

    /// When the next failed thumbnail is due again (None: nothing waits).
    pub fn next_thumb_retry(&self) -> Option<std::time::Instant> {
        self.thumb_failures.values().filter(|(n, _)| thumb_backoff(*n).is_some()).map(|(_, at)| *at).min()
    }
}

/// How long to wait before trying a thumbnail again after its `failures`-th failure; None once
/// it has failed too often (it's tried again on the next launch).
pub fn thumb_backoff(failures: u32) -> Option<std::time::Duration> {
    const WAITS: [u64; 4] = [10, 60, 300, 1800];
    WAITS.get(failures.checked_sub(1)? as usize).map(|s| std::time::Duration::from_secs(*s))
}

/// The 11-character video id of a YouTube link (watch, youtu.be, shorts, live, embed).
pub fn youtube_id(url: &str) -> Option<&str> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h).split(':').next().unwrap_or("").to_ascii_lowercase();
    let path = path.split('#').next().unwrap_or("");
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let id = if host == "youtu.be" {
        path.split('/').next()
    } else if host == "youtube.com" || host.ends_with(".youtube.com") {
        match path.split('/').collect::<Vec<_>>().as_slice() {
            ["watch"] => query.split('&').find_map(|kv| kv.strip_prefix("v=")),
            ["shorts" | "live" | "embed" | "v", id, ..] => Some(*id),
            _ => None,
        }
    } else {
        None
    };
    id.filter(|id| id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
}

/// The picture a row shows: the one recorded with the item, else YouTube's own for its video
/// (items added before thumbnails were recorded, and paths that don't record one).
pub fn thumb_url(item: &Item) -> Option<String> {
    if item.thumbnail.is_some() {
        return item.thumbnail.clone();
    }
    let id = youtube_id(&item.url).filter(|_| matches!(item.kind, Kind::Media(_)))?;
    Some(format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg"))
}

/// Where a thumbnail is cached, without its extension: a stable hash of its URL (FNV-1a),
/// so it survives restarts. The extension comes from the image itself (`image_ext`).
pub fn thumb_file(dir: &std::path::Path, url: &str) -> std::path::PathBuf {
    let hash = url.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3));
    dir.join(format!("{hash:016x}"))
}

/// The image format from its first bytes (the decoder goes by file extension).
pub fn image_ext(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("webp"),
        _ => None,
    }
}

/// "retrying in 30s" for a failed item waiting for its automatic retry.
pub fn retry_note(item: &Item, now_unix: i64) -> Option<String> {
    let at = item.retry_at?;
    Some(if at > now_unix { format!("retrying in {}s", at - now_unix) } else { "retrying now".into() })
}

/// A stopped download can take a fresh link (its old one expired).
pub fn can_refresh(item: &Item) -> bool {
    matches!(item.status, Status::Paused | Status::Failed(_))
}

impl Model {
    pub fn refresh_typed(&mut self, id: rdm_core::ItemId, text: String) {
        self.refresh = Some((id, text));
    }

    /// The link to switch `id` to, if one was typed for it and it is a web link.
    pub fn take_refresh(&mut self, id: rdm_core::ItemId) -> Option<String> {
        let (for_id, text) = self.refresh.clone()?;
        if for_id != id {
            return None;
        }
        let link = text.trim();
        if !(link.starts_with("http://") || link.starts_with("https://")) || link.contains(char::is_whitespace) {
            self.notice = Some("Paste a link that starts with http:// or https://".into());
            return None;
        }
        self.refresh = None;
        Some(link.to_string())
    }
}

/// "0:42", "42:18", "1:02:40".
pub fn duration_label(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

/// The line under the URL bar.
pub fn url_hint(m: &Model) -> &'static str {
    if m.probing {
        "Reading video info…"
    } else if m.settings.clipboard_watch {
        "Copy any link and Snag offers to download it. Paste one here and press Enter."
    } else {
        "Paste a link and press Enter."
    }
}

/// The tag in the URL bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkTag {
    Video,
    Playlist,
    /// A page of images (gallery-dl).
    Images,
    /// A magnet link or `.torrent`.
    Torrent,
    File,
}

impl LinkTag {
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "Video detected",
            Self::Playlist => "Playlist detected",
            Self::Images => "Images detected",
            Self::Torrent => "Torrent",
            Self::File => "File link",
        }
    }
}

pub fn link_tag(url: &str) -> Option<LinkTag> {
    let url = url.trim();
    if rdm_core::is_torrent_link(url) {
        return Some(LinkTag::Torrent);
    }
    let web = (url.starts_with("http://") || url.starts_with("https://")) && !url.contains(char::is_whitespace);
    if !web {
        return None;
    }
    if rdm_media::gallery::is_gallery_url(url) {
        Some(LinkTag::Images)
    } else if url.contains("list=") || url.contains("/playlist") {
        Some(LinkTag::Playlist)
    } else if rdm_media::is_media_url(url) {
        Some(LinkTag::Video)
    } else {
        Some(LinkTag::File)
    }
}

/// "1080p · MP4", "Audio · MP3", or the file extension ("ZIP"); empty if unknown.
pub fn kind_label(item: &Item) -> String {
    match &item.kind {
        Kind::Media(MediaFormat::Video { max_height }) => format!("{max_height}p · MP4"),
        Kind::Media(MediaFormat::AudioMp3) => "Audio · MP3".into(),
        Kind::Media(MediaFormat::Live { max_height }) => format!("Live · {max_height}p"),
        Kind::Gallery => "Images".into(),
        Kind::Torrent => "Torrent".into(),
        Kind::Http => item
            .name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_uppercase())
            .filter(|e| !e.is_empty() && e.len() <= 5)
            .unwrap_or_default(),
    }
}

/// The second line of a row.
pub fn row_meta(item: &Item) -> String {
    let size = match (item.downloaded, item.total) {
        (d, Some(t)) if item.status != Status::Done => format!("{} of {}", format::size(d), format::size(t)),
        (_, Some(t)) => format::size(t),
        (0, None) => String::new(),
        (d, None) => format::size(d),
    };
    let lead = match &item.status {
        Status::Failed(e) if e.is_empty() => return "Failed".into(),
        Status::Failed(e) => return e.clone(),
        Status::Running => None,
        Status::Queued => Some("Queued"),
        Status::Paused => Some("Paused"),
        Status::Done => Some("Completed"),
    };
    let parts: Vec<String> = [lead.map(str::to_string), Some(kind_label(item)), Some(size)].into_iter().flatten().filter(|p| !p.is_empty()).collect();
    if parts.is_empty() { format::status_label(&item.status) } else { parts.join(" · ") }
}

/// The inspector's main button for an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainAction {
    Pause,
    Resume,
    /// Only for a finished item whose file is gone.
    Redownload,
}

pub fn main_action(item: &Item) -> Option<MainAction> {
    match item.status {
        Status::Running | Status::Queued => Some(MainAction::Pause),
        Status::Paused | Status::Failed(_) => Some(MainAction::Resume),
        Status::Done if crate::state::file_missing(item) => Some(MainAction::Redownload),
        Status::Done => None,
    }
}

/// What the animations aim at right now.
pub fn motion_targets(m: &Model) -> crate::motion::Targets {
    crate::motion::Targets {
        tab: FILTERS.iter().position(|f| *f == m.filter).unwrap_or(0),
        sidebar: m.sidebar_open,
        inspector: m.inspected().is_some(),
        sheet: matches!(m.screen, crate::state::Screen::Picker | crate::state::Screen::Settings) || m.confirm_quit || m.pair_request.is_some() || m.info.is_some(),
        popover: m.speed_open || m.help_open,
        toast: m.screen == crate::state::Screen::Downloads && (m.toast.is_some() || m.notice.is_some()),
        search: m.search_open,
    }
}

/// Every row's progress (0 … 1) and whether it finished, for the bar easing and finish pulse.
pub fn row_targets(m: &Model) -> Vec<crate::motion::RowTarget> {
    m.items
        .iter()
        .map(|i| {
            let done = i.status == Status::Done;
            let progress = match i.total {
                _ if done => 1.0,
                Some(t) if t > 0 => (i.downloaded as f32 / t as f32).min(1.0),
                _ => 0.0,
            };
            crate::motion::RowTarget { id: i.id.0, progress, done }
        })
        .collect()
}

/// Cancel (stop, remove, throw away the partial data) is for downloads that haven't finished.
pub fn can_cancel(item: &Item) -> bool {
    item.status != Status::Done
}

/// A row's buttons show only while it is hovered or selected (quieter list).
pub fn row_actions_visible(id: rdm_core::ItemId, hovered: Option<rdm_core::ItemId>, selected: Option<rdm_core::ItemId>) -> bool {
    hovered == Some(id) || selected == Some(id)
}

/// "youtube.com" from "https://www.youtube.com/watch?…".
pub fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host.split(':').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

/// When the next scheduled queue starts ("23:00"), or None.
pub fn next_queue_start(queues: &[Queue], now: Now) -> Option<String> {
    let mut best: Option<(u32, u16)> = None;
    for s in queues.iter().filter_map(|q| q.schedule.as_ref()) {
        for d in 0..8u32 {
            let day = (now.weekday as u32 + d) % 7;
            if s.days[day as usize] && (d > 0 || s.start > now.minute) {
                let wait = d * 1440 + s.start as u32 - now.minute as u32;
                if best.is_none_or(|(w, _)| wait < w) {
                    best = Some((wait, s.start));
                }
                break;
            }
        }
    }
    best.map(|(_, start)| crate::queues::fmt_hhmm(start))
}

const SLIDER_MIN: f64 = 64.0 * 1024.0;
const SLIDER_MAX: f64 = 50.0 * 1024.0 * 1024.0;
const SLIDER_STEP: u64 = 64 * 1024;

/// The speed slider is logarithmic from 64 KB/s to 50 MB/s, in 64 KB/s steps.
pub fn slider_to_bps(v: f32) -> u64 {
    let v = (v as f64).clamp(0.0, 1.0);
    let bps = SLIDER_MIN * (SLIDER_MAX / SLIDER_MIN).powf(v);
    ((bps / SLIDER_STEP as f64).round() as u64).max(1) * SLIDER_STEP
}

pub fn bps_to_slider(bps: u64) -> f32 {
    let bps = (bps as f64).clamp(SLIDER_MIN, SLIDER_MAX);
    ((bps / SLIDER_MIN).ln() / (SLIDER_MAX / SLIDER_MIN).ln()) as f32
}

/// A segmented control's values: the usual ones plus the current value if it's unusual.
pub fn segment_choices(base: &[usize], current: usize) -> Vec<usize> {
    let mut list = base.to_vec();
    list.push(current);
    list.sort();
    list.dedup();
    list
}

/// At most `max` characters, with "…" at the end when cut.
pub fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut cut: String = s.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// At most `max` characters, keeping the end ("…ads\\RDM\\Videos"): for paths.
pub fn ellipsize_left(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let keep = max.saturating_sub(1);
    std::iter::once('…').chain(s.chars().skip(n - keep)).collect()
}

/// Where a new download of `category` is saved.
pub fn save_dir(settings: &rdm_core::Settings, category: Category) -> std::path::PathBuf {
    if settings.sort_into_folders { settings.download_dir.join(category.folder()) } else { settings.download_dir.clone() }
}

/// ("2.0", "MB/s") for the popover's big number; 0 = ("Unlimited", "").
pub fn speed_parts(bps: u64) -> (String, String) {
    if bps == 0 {
        return ("Unlimited".into(), String::new());
    }
    let s = format::speed(bps);
    match s.split_once(' ') {
        Some((n, unit)) => (n.to_string(), unit.to_string()),
        None => (s, String::new()),
    }
}

/// ("Today", "14:02") or ("Sep 30", "09:05") for a unix time, in local time.
pub fn when_label(unix: i64, now: chrono::DateTime<chrono::Local>) -> (String, String) {
    use chrono::TimeZone;
    let Some(t) = chrono::Local.timestamp_opt(unix, 0).single() else { return (String::new(), "--:--".into()) };
    let day = if t.date_naive() == now.date_naive() { "Today".to_string() } else { t.format("%b %-d").to_string() };
    (day, t.format("%H:%M").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{MediaTab, Model, Picker, Screen};
    use rdm_core::{Category, Event, Item, ItemId, Kind, MediaFormat, MediaInfo, Now, Queue, Schedule, Status};
    use rdm_media::{Entry, QualityOption};

    fn item(id: u64, name: &str, category: Category, status: Status) -> Item {
        Item {
            id: ItemId(id),
            url: format!("https://www.example.com/{name}"),
            name: name.into(),
            category,
            status,
            dest: None,
            downloaded: 0,
            total: None,
            speed_bps: 0,
            queue: 0,
            added: id as i64,
            kind: Kind::Http,
            referrer: None,
            work_dir: None,
            thumbnail: None,
            duration: None,
            retry_at: None,
        }
    }

    #[test]
    fn cancel_only_for_unfinished() {
        let with = |status: Status| item(1, "a.zip", Category::Archive, status);
        for status in [Status::Running, Status::Queued, Status::Paused, Status::Failed("403".into())] {
            assert!(can_cancel(&with(status.clone())), "{status:?}");
        }
        assert!(!can_cancel(&with(Status::Done)), "finished: Remove from list instead");
    }

    #[test]
    fn row_actions_show_on_hover_or_selected() {
        let (a, b) = (ItemId(1), ItemId(2));
        assert!(!row_actions_visible(a, None, None), "a quiet row shows no buttons");
        assert!(row_actions_visible(a, Some(a), None), "hovered");
        assert!(row_actions_visible(a, None, Some(a)), "selected");
        assert!(!row_actions_visible(a, Some(b), Some(b)), "another row is busy");
    }

    fn never() -> Schedule {
        Schedule { start: 23 * 60, stop: None, days: [false; 7] }
    }

    /// 1 running video, 2 done zip, 3 queued-but-scheduled pdf, 4 failed iso, 5 done mp3 in queue 1.
    fn model() -> Model {
        let mut m = Model::default();
        m.queues.push(Queue { id: 1, name: "Tonight".into(), max_concurrent: 1, schedule: Some(never()) });
        m.apply(Event::Added(item(1, "Rust Async.mp4", Category::Video, Status::Running)));
        m.apply(Event::Added(item(2, "notes.zip", Category::Archive, Status::Done)));
        m.apply(Event::Added(Item { queue: 1, ..item(3, "paper.pdf", Category::Document, Status::Queued) }));
        m.apply(Event::Added(item(4, "ubuntu.iso", Category::Archive, Status::Failed("HTTP 403".into()))));
        m.apply(Event::Added(Item { queue: 1, ..item(5, "Lofi.mp3", Category::Music, Status::Done) }));
        m
    }

    fn ids(items: &[&Item]) -> Vec<u64> {
        items.iter().map(|i| i.id.0).collect()
    }

    #[test]
    fn filter_counts() {
        let c = model().counts();
        assert_eq!((c.all, c.active, c.done, c.scheduled), (5, 2, 2, 1));
        assert_eq!(c.category(Category::Archive), 2);
        assert_eq!(c.category(Category::Program), 0);
        assert_eq!(c.queue(0), 3);
        assert_eq!(c.queue(1), 2);
    }

    #[test]
    fn visible_splits_and_filters() {
        let mut m = model();
        let (down, recent) = m.visible();
        assert_eq!(ids(&down), vec![4, 3, 1], "newest first");
        assert_eq!(ids(&recent), vec![5, 2]);
        m.filter = Filter::Active;
        assert_eq!(ids(&m.visible().0), vec![4, 1]);
        assert!(m.visible().1.is_empty());
        m.filter = Filter::Scheduled;
        assert_eq!(ids(&m.visible().0), vec![3]);
        m.filter = Filter::All;
        m.library = Library::Category(Category::Archive);
        assert_eq!((ids(&m.visible().0), ids(&m.visible().1)), (vec![4], vec![2]));
        m.library = Library::Queue(1);
        assert_eq!((ids(&m.visible().0), ids(&m.visible().1)), (vec![3], vec![5]));
        m.library = Library::All;
        m.search = "  RUST ".into();
        assert_eq!(ids(&m.visible().0), vec![1], "case-insensitive, trimmed");
        assert!(m.visible().1.is_empty());
    }

    #[test]
    fn inspector_item_respects_filters() {
        let mut m = model();
        m.selected = Some(ItemId(2));
        assert_eq!(m.inspected().map(|i| i.id.0), Some(2));
        m.filter = Filter::Active;
        assert_eq!(m.inspected(), None, "a hidden item isn't inspected");
        m.filter = Filter::All;
        m.apply(Event::Removed(ItemId(2)));
        assert_eq!(m.inspected(), None);
    }

    #[test]
    fn link_tag_cases() {
        assert_eq!(link_tag("https://www.youtube.com/watch?v=abc"), Some(LinkTag::Video));
        assert_eq!(link_tag("https://www.youtube.com/playlist?list=PL1"), Some(LinkTag::Playlist));
        assert_eq!(link_tag("https://www.youtube.com/watch?v=a&list=PL1"), Some(LinkTag::Playlist));
        assert_eq!(link_tag(" https://files.example.com/file.zip "), Some(LinkTag::File));
        assert_eq!(link_tag(""), None);
        assert_eq!(link_tag("ftp://x/y"), None);
        assert_eq!(link_tag("hello"), None);
        assert_eq!(LinkTag::Video.label(), "Video detected");
        assert_eq!(link_tag("https://www.pinterest.com/pin/1/"), Some(LinkTag::Images));
        assert_eq!(link_tag("https://www.reddit.com/gallery/abc"), Some(LinkTag::Images), "before the video check");
        assert_eq!(LinkTag::Images.label(), "Images detected");
        assert_eq!(link_tag("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x"), Some(LinkTag::Torrent));
        assert_eq!(link_tag("https://site.org/ubuntu.iso.torrent"), Some(LinkTag::Torrent));
        assert_eq!(LinkTag::Torrent.label(), "Torrent");
    }

    #[test]
    fn row_meta_never_empty() {
        let mut running = item(1, "a.mp4", Category::Video, Status::Running);
        running.kind = Kind::Media(MediaFormat::Video { max_height: 1080 });
        running.downloaded = 642 * 1024 * 1024;
        running.total = Some(1229 * 1024 * 1024);
        assert_eq!(row_meta(&running), "1080p · MP4 · 642 MB of 1.2 GB");
        let mut paused = item(2, "u.iso", Category::Archive, Status::Paused);
        paused.downloaded = 2048;
        assert_eq!(row_meta(&paused), "Paused · ISO · 2.0 KB");
        let mut done = item(3, "n.pdf", Category::Document, Status::Done);
        done.total = Some(12 * 1024 * 1024);
        assert_eq!(row_meta(&done), "Completed · PDF · 12 MB");
        let mp3 = Item { kind: Kind::Media(MediaFormat::AudioMp3), ..item(4, "x", Category::Music, Status::Queued) };
        assert_eq!(row_meta(&mp3), "Queued · Audio · MP3");
        let live = Item { kind: Kind::Media(MediaFormat::Live { max_height: 720 }), ..item(8, "x", Category::Video, Status::Queued) };
        assert_eq!(row_meta(&live), "Queued · Live · 720p");
        let mut gallery = Item { kind: Kind::Gallery, ..item(7, "pinterest.com · 1", Category::Image, Status::Done) };
        gallery.total = Some(350 * 1024);
        assert_eq!(row_meta(&gallery), "Completed · Images · 350 KB");
        assert_eq!(row_meta(&item(5, "noext", Category::Other, Status::Failed(String::new()))), "Failed");
        assert_eq!(row_meta(&item(6, "f", Category::Other, Status::Failed("HTTP 403".into()))), "HTTP 403");
    }

    #[test]
    fn host_strips_scheme_www_port_and_user() {
        assert_eq!(host("https://www.youtube.com/watch?v=1"), "youtube.com");
        assert_eq!(host("http://user@files.example.org:8080/a"), "files.example.org");
        assert_eq!(host("nonsense"), "nonsense");
    }

    #[test]
    fn next_queue_start_picks_soonest() {
        let at = |start: u16, days: [bool; 7]| Queue { id: 1, name: "q".into(), max_concurrent: 1, schedule: Some(Schedule { start, stop: None, days }) };
        let every = [true; 7];
        let now = Now::at(0, 22, 0); // Monday 22:00
        assert_eq!(next_queue_start(&[at(23 * 60, every)], now).as_deref(), Some("23:00"));
        assert_eq!(next_queue_start(&[at(23 * 60, every), at(22 * 60 + 30, every)], now).as_deref(), Some("22:30"));
        let mut tuesday_only = [false; 7];
        tuesday_only[1] = true;
        assert_eq!(next_queue_start(&[at(7 * 60, tuesday_only), at(23 * 60, every)], now).as_deref(), Some("23:00"));
        assert_eq!(next_queue_start(&[at(21 * 60, [false; 7])], now), None, "no days: never starts");
        assert_eq!(next_queue_start(&[Queue { schedule: None, ..at(0, every) }], now), None);
    }

    #[test]
    fn slider_round_trip() {
        const KB: u64 = 1024;
        for bps in [64 * KB, 512 * KB, 1024 * KB, 2048 * KB, 5120 * KB, 50 * 1024 * KB] {
            assert_eq!(slider_to_bps(bps_to_slider(bps)), bps, "{bps}");
        }
        assert_eq!(slider_to_bps(0.0), 64 * KB);
        assert_eq!(slider_to_bps(1.0), 50 * 1024 * KB);
        assert_eq!(bps_to_slider(0), 0.0);
        assert_eq!(bps_to_slider(u64::MAX), 1.0);
        assert_eq!(slider_to_bps(0.5) % (64 * KB), 0, "rounded to 64 KB steps");
    }

    #[test]
    fn slider_preview_does_not_apply() {
        let mut m = model();
        m.preview_speed(0.5);
        assert_eq!(m.settings.speed_limit_bps, 0, "dragging changes nothing yet");
        assert_eq!(m.speed_preview, Some(slider_to_bps(0.5)));
        assert_eq!(m.shown_limit(), slider_to_bps(0.5));
        assert_eq!(m.release_speed(), Some(slider_to_bps(0.5)));
        assert_eq!(m.speed_preview, None);
        assert_eq!(m.release_speed(), None, "a release without a drag applies nothing");
    }

    #[test]
    fn segment_choices_include_current() {
        assert_eq!(segment_choices(&[1, 2, 4, 8, 16], 8), vec![1, 2, 4, 8, 16]);
        assert_eq!(segment_choices(&[1, 2, 4, 8, 16], 6), vec![1, 2, 4, 6, 8, 16]);
    }

    fn options() -> Vec<QualityOption> {
        let video = |h: u32| QualityOption { label: format!("{h}p"), format: MediaFormat::Video { max_height: h }, approx_size: None };
        vec![video(1080), video(720), QualityOption { label: "MP3".into(), format: MediaFormat::AudioMp3, approx_size: None }]
    }

    fn playlist(n: usize) -> MediaInfo {
        let entries = (1..=n).map(|i| Entry { url: format!("https://y/{i}"), title: format!("Video {i}"), thumbnail: None, duration: None }).collect();
        MediaInfo { title: "List".into(), duration: None, options: options(), entries, thumbnail: None, live: false }
    }

    #[test]
    fn picker_tabs_split_options() {
        let mut p = Picker::new("https://y/x".into(), MediaInfo { entries: vec![], ..playlist(0) }, 1, 0);
        assert_eq!(p.tab, MediaTab::Video);
        assert_eq!(p.visible_options().iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![0, 1]);
        assert!(p.has_both_tabs());
        p.set_tab(MediaTab::Audio);
        assert_eq!(p.choice, 2, "switching tab picks that tab's first option");
        assert_eq!(p.visible_options().iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![2]);
        let audio_first = Picker::new("u".into(), playlist(0), 2, 0);
        assert_eq!(audio_first.tab, MediaTab::Audio, "opens on the tab of the preferred choice");
    }

    #[test]
    fn playlist_selection_filters_requests() {
        let mut p = Picker::new("https://y/list".into(), playlist(3), 0, 1);
        assert_eq!(p.queue, 1);
        assert_eq!(p.selected_count(), 3, "everything is selected at first");
        p.toggle_entry(1);
        let urls: Vec<String> = p.requests().into_iter().map(|r| r.0).collect();
        assert_eq!(urls, vec!["https://y/1", "https://y/3"]);
        p.select_all(true);
        assert_eq!(p.requests().len(), 3);
        p.toggle_entry(99); // out of range: ignored
        assert_eq!(p.selected_count(), 3);
    }

    #[test]
    fn playlist_none_selected_adds_nothing() {
        let mut p = Picker::new("https://y/list".into(), playlist(2), 0, 0);
        p.select_all(false);
        assert_eq!(p.selected_count(), 0);
        assert!(p.requests().is_empty());
        let single = Picker::new("https://y/x".into(), playlist(0), 0, 0);
        assert_eq!(single.requests().len(), 1, "a single video is always one request");
    }

    #[test]
    fn draft_accent_falls_back() {
        let mut m = model();
        m.settings.accent = "#0a84ff".into();
        m.open_settings(crate::state::SettingsTab::Appearance);
        m.draft.accent = "#ff".into();
        assert_eq!(m.accent_hex(), "#0a84ff", "invalid draft: keep the saved accent");
        m.draft.accent = "#32d74b".into();
        assert_eq!(m.accent_hex(), "#32d74b", "valid draft previews live");
        m.screen = Screen::Downloads;
        assert_eq!(m.accent_hex(), "#0a84ff", "outside Settings the saved accent rules");
        m.draft.accent = "orange".into();
        assert!(m.draft.to_settings(&m.settings).is_err());
    }

    #[test]
    fn clipboard_link_becomes_toast() {
        let mut m = Model::default();
        m.clipboard_seen(Some("https://old.example/a.zip".into()));
        assert_eq!(m.toast, None, "the first read only records what's there");
        m.clipboard_seen(Some("https://x.example/b.zip".into()));
        assert_eq!(m.toast.as_deref(), Some("https://x.example/b.zip"));
        assert_eq!(m.url, "", "the URL bar is left alone");
        m.toast = None;
        m.clipboard_seen(Some("https://x.example/b.zip".into()));
        assert_eq!(m.toast, None, "the same link isn't suggested twice");
    }

    #[test]
    fn closing_settings_saves_or_stays_open() {
        let mut m = model();
        m.queues[1].schedule = Some(Schedule { start: 23 * 60, stop: None, days: [true; 7] }); // a valid saved queue
        m.open_settings(crate::state::SettingsTab::Connections);
        m.draft.connections = "4".into();
        let (settings, queues) = m.close_settings().expect("valid");
        assert_eq!(settings.connections, 4);
        assert_eq!(queues, m.queues);
        assert_eq!(m.screen, Screen::Downloads);
        m.open_settings(crate::state::SettingsTab::Speed);
        m.queue_drafts[1].start = "25:00".into();
        assert!(m.close_settings().is_err());
        assert_eq!(m.screen, Screen::Settings, "stays open to fix it");
        assert!(m.settings_error.is_some());
    }

    #[test]
    fn ellipsize_cuts_long_text_on_char_boundaries() {
        assert_eq!(ellipsize("short.zip", 20), "short.zip");
        let long = "😳".repeat(300);
        let cut = ellipsize(&long, 40);
        assert_eq!(cut.chars().count(), 40);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn ellipsize_left_keeps_the_end_of_paths() {
        assert_eq!(ellipsize_left(r"C:\dl", 20), r"C:\dl");
        let cut = ellipsize_left(r"C:\Users\someone\Downloads\RDM\Videos", 16);
        assert_eq!(cut.chars().count(), 16);
        assert_eq!(cut, r"…oads\RDM\Videos");
    }

    #[test]
    fn save_dir_follows_category_sorting() {
        let mut s = rdm_core::Settings { download_dir: std::path::PathBuf::from(r"C:\dl"), ..Default::default() };
        s.sort_into_folders = true;
        assert_eq!(save_dir(&s, Category::Video), std::path::PathBuf::from(r"C:\dl\Videos"));
        s.sort_into_folders = false;
        assert_eq!(save_dir(&s, Category::Music), std::path::PathBuf::from(r"C:\dl"));
    }

    #[test]
    fn speed_parts_split_number_and_unit() {
        assert_eq!(speed_parts(2 * 1024 * 1024), ("2.0".to_string(), "MB/s".to_string()));
        assert_eq!(speed_parts(512 * 1024), ("512.0".to_string(), "KB/s".to_string()));
        assert_eq!(speed_parts(0), ("Unlimited".to_string(), String::new()));
    }

    #[test]
    fn when_label_today_or_date() {
        use chrono::TimeZone;
        let now = chrono::Local.with_ymd_and_hms(2026, 10, 7, 18, 0, 0).unwrap();
        let today = chrono::Local.with_ymd_and_hms(2026, 10, 7, 14, 2, 0).unwrap().timestamp();
        let earlier = chrono::Local.with_ymd_and_hms(2026, 9, 30, 9, 5, 0).unwrap().timestamp();
        assert_eq!(when_label(today, now), ("Today".to_string(), "14:02".to_string()));
        assert_eq!(when_label(earlier, now), ("Sep 30".to_string(), "09:05".to_string()));
        assert_eq!(when_label(0, now).1.len(), 5, "any timestamp gives a time");
    }

    #[test]
    fn switching_settings_tab_keeps_edits() {
        let mut m = model();
        m.queues[1].schedule = Some(Schedule { start: 23 * 60, stop: None, days: [true; 7] });
        m.open_settings(crate::state::SettingsTab::General);
        m.draft.download_dir = r"D:\elsewhere".into();
        m.draft.accent = "#0a84ff".into();
        m.queue_drafts[0].name = "Everything".into();
        m.switch_settings_tab(crate::state::SettingsTab::Speed);
        assert_eq!(m.settings_tab, crate::state::SettingsTab::Speed);
        assert_eq!(m.draft.download_dir, r"D:\elsewhere", "edits on other tabs survive a tab switch");
        assert_eq!(m.draft.accent, "#0a84ff");
        assert_eq!(m.queue_drafts[0].name, "Everything");
        let (settings, queues) = m.close_settings().expect("valid");
        assert_eq!(settings.download_dir, std::path::PathBuf::from(r"D:\elsewhere"));
        assert_eq!(queues[0].name, "Everything");
    }

    #[test]
    fn settings_echo_keeps_open_sheet_draft() {
        let mut m = model();
        m.queues[1].schedule = Some(Schedule { start: 23 * 60, stop: None, days: [true; 7] });
        m.open_settings(crate::state::SettingsTab::Extension);
        m.draft.download_dir = r"D:\elsewhere".into();
        let regenerated = rdm_core::Settings { extension_token: "new-token".into(), ..m.settings.clone() };
        m.apply(Event::Settings(regenerated));
        assert_eq!(m.settings.extension_token, "new-token");
        assert_eq!(m.draft.download_dir, r"D:\elsewhere", "an echo while the sheet is open keeps the typing");
        let (saved, _) = m.close_settings().expect("valid");
        assert_eq!(saved.extension_token, "new-token", "saving doesn't undo the regenerated token");
        // Closed: an echo refreshes the draft as before.
        m.apply(Event::Settings(rdm_core::Settings { connections: 2, ..m.settings.clone() }));
        assert_eq!(m.draft.connections, "2");
    }

    #[test]
    fn finished_item_with_its_file_offers_no_redownload() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.zip");
        std::fs::write(&file, b"x").unwrap();
        let mut done = item(1, "a.zip", Category::Archive, Status::Done);
        done.dest = Some(file);
        assert_eq!(main_action(&done), None, "a finished file that's still there has nothing to redo");
        done.dest = Some(dir.path().join("gone.zip"));
        assert_eq!(main_action(&done), Some(MainAction::Redownload));
        assert_eq!(main_action(&item(2, "b", Category::Other, Status::Running)), Some(MainAction::Pause));
        assert_eq!(main_action(&item(3, "c", Category::Other, Status::Queued)), Some(MainAction::Pause));
        assert_eq!(main_action(&item(4, "d", Category::Other, Status::Paused)), Some(MainAction::Resume));
        assert_eq!(main_action(&item(5, "e", Category::Other, Status::Failed("x".into()))), Some(MainAction::Resume));
    }

    #[test]
    fn quit_asks_only_when_something_runs() {
        let mut idle = Model::default();
        idle.apply(Event::Added(item(1, "a.zip", Category::Archive, Status::Done)));
        idle.apply(Event::Added(item(2, "b.zip", Category::Archive, Status::Paused)));
        assert!(idle.request_quit(), "nothing running: quit straight away");
        assert!(!idle.confirm_quit);

        let mut busy = model();
        assert_eq!(busy.busy_count(), 2, "one running, one queued");
        assert!(!busy.request_quit(), "something is downloading: ask first");
        assert!(busy.confirm_quit);
    }

    #[test]
    fn keep_downloading_cancels_quit() {
        let mut m = model();
        assert!(!m.request_quit());
        m.keep_downloading();
        assert!(!m.confirm_quit);
    }

    #[test]
    fn pause_and_resume_all_pick_the_right_items() {
        let m = model();
        let mut paused = m.pause_all_ids();
        paused.sort();
        assert_eq!(paused, vec![ItemId(1), ItemId(3)], "running and queued");
        assert_eq!(m.resume_all_ids(), vec![ItemId(4)], "paused and failed");
    }

    fn finish(m: &mut Model, id: u64, status: Status) {
        let mut it = m.items.iter().find(|i| i.id == ItemId(id)).cloned().unwrap();
        it.status = status;
        m.apply(Event::Updated(it));
    }

    #[test]
    fn notification_only_on_transition() {
        let mut m = model();
        finish(&mut m, 1, Status::Done);
        let notes = m.take_notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Download finished");
        assert_eq!(notes[0].body, "Rust Async.mp4");
        finish(&mut m, 1, Status::Done);
        assert!(m.take_notes().is_empty(), "an update to an already finished item says nothing");
        finish(&mut m, 3, Status::Failed("HTTP 404".into()));
        let failed = m.take_notes();
        assert_eq!(failed[0].title, "Download failed");
        assert_eq!(failed[0].body, "paper.pdf: HTTP 404");
        m.settings.notify = false;
        finish(&mut m, 4, Status::Done);
        assert!(m.take_notes().is_empty(), "notifications can be turned off");
    }

    #[test]
    fn notifications_batch_bursts() {
        let mut m = Model::default();
        for id in 1..=5 {
            m.apply(Event::Added(item(id, &format!("clip {id}.mp4"), Category::Video, Status::Running)));
        }
        for id in 1..=5 {
            finish(&mut m, id, Status::Done);
        }
        let notes = m.take_notes();
        assert_eq!(notes.len(), 1, "a finished playlist is one notification, not five");
        assert_eq!(notes[0].title, "5 downloads finished");
        assert!(m.take_notes().is_empty());
        finish(&mut m, 1, Status::Running);
        finish(&mut m, 1, Status::Done);
        finish(&mut m, 2, Status::Running);
        finish(&mut m, 2, Status::Done);
        assert_eq!(m.take_notes().len(), 2, "a couple are shown one by one");
    }

    #[test]
    fn picked_playlist_meta_follows_selection() {
        let mut info = playlist(3);
        for (n, e) in info.entries.iter_mut().enumerate() {
            e.thumbnail = Some(format!("https://i/{n}.jpg"));
            e.duration = Some(n as f64);
        }
        let mut p = Picker::new("https://y/list".into(), info, 0, 0);
        p.toggle_entry(1);
        let picked = p.requests_with_meta();
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[1].0 .0, "https://y/3");
        assert_eq!(picked[1].1, (Some("https://i/2.jpg".to_string()), Some(2.0)), "meta stays with its video");
    }

    #[test]
    fn missing_thumbs_lists_each_url_once() {
        let mut m = model();
        let with = |id: u64, url: &str| Item { thumbnail: Some(url.into()), ..item(id, "v.mp4", Category::Video, Status::Done) };
        m.items = vec![with(1, "https://i/a.jpg"), with(2, "https://i/a.jpg"), with(3, "https://i/b.jpg"), item(4, "f.zip", Category::Archive, Status::Done)];
        let now = std::time::Instant::now();
        assert_eq!(m.missing_thumbs(now), vec!["https://i/a.jpg".to_string(), "https://i/b.jpg".to_string()]);
        m.thumb_pending.insert("https://i/a.jpg".into());
        m.thumbs.insert("https://i/b.jpg".into(), std::path::PathBuf::from("b"));
        assert!(m.missing_thumbs(now).is_empty(), "being fetched or already here");
        m.picker = Some(Picker::new("u".into(), MediaInfo { thumbnail: Some("https://i/p.jpg".into()), ..playlist(0) }, 0, 0));
        assert_eq!(m.missing_thumbs(now), vec!["https://i/p.jpg".to_string()], "the picker's preview too");
    }

    #[test]
    fn youtube_thumbnail_from_id() {
        for url in [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://m.youtube.com/watch?feature=share&v=dQw4w9WgXcQ&t=42",
            "https://youtu.be/dQw4w9WgXcQ?si=abc",
            "https://www.youtube.com/shorts/dQw4w9WgXcQ",
            "https://www.youtube.com/live/dQw4w9WgXcQ?feature=x",
            "https://www.youtube.com/embed/dQw4w9WgXcQ",
            "https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RD1",
            "http://youtube.com/watch?v=dQw4w9WgXcQ#t=1",
        ] {
            assert_eq!(youtube_id(url), Some("dQw4w9WgXcQ"), "{url}");
        }
        for url in [
            "https://www.youtube.com/@channel/videos",
            "https://www.youtube.com/playlist?list=PL1",
            "https://youtu.be/short",
            "https://notyoutube.com/watch?v=dQw4w9WgXcQ",
            "https://vimeo.com/824123456",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ!",
        ] {
            assert_eq!(youtube_id(url), None, "{url}");
        }
        let video = Item { kind: Kind::Media(MediaFormat::Video { max_height: 1080 }), url: "https://youtu.be/dQw4w9WgXcQ".into(), ..item(1, "v.mp4", Category::Video, Status::Done) };
        assert_eq!(thumb_url(&video).as_deref(), Some("https://i.ytimg.com/vi/dQw4w9WgXcQ/mqdefault.jpg"), "older items without a recorded thumbnail");
        let recorded = Item { thumbnail: Some("https://i/own.jpg".into()), ..video.clone() };
        assert_eq!(thumb_url(&recorded).as_deref(), Some("https://i/own.jpg"), "a recorded one wins");
        let file = Item { url: "https://youtu.be/dQw4w9WgXcQ".into(), ..item(2, "a.zip", Category::Archive, Status::Done) };
        assert_eq!(thumb_url(&file), None, "only videos and music get a picture");
        let m = Model { items: vec![video], ..Model::default() };
        assert_eq!(m.missing_thumbs(std::time::Instant::now()), vec!["https://i.ytimg.com/vi/dQw4w9WgXcQ/mqdefault.jpg".to_string()], "and it's fetched");
    }

    #[test]
    fn thumb_retries_with_backoff() {
        use std::time::Duration;
        let t0 = std::time::Instant::now();
        let url = "https://i/a.jpg".to_string();
        let mut m = Model { items: vec![Item { thumbnail: Some(url.clone()), ..item(1, "v.mp4", Category::Video, Status::Done) }], ..Model::default() };
        assert_eq!(m.missing_thumbs(t0), vec![url.clone()]);
        m.thumb_pending.insert(url.clone());
        assert!(m.missing_thumbs(t0).is_empty(), "on its way");
        // It failed: not pending any more, but not asked for again straight away either.
        m.thumb_ready(url.clone(), None, t0);
        assert!(!m.thumb_pending.contains(&url));
        assert!(m.missing_thumbs(t0).is_empty());
        let first = thumb_backoff(1).expect("a first retry");
        assert_eq!(m.next_thumb_retry(), Some(t0 + first));
        assert_eq!(m.missing_thumbs(t0 + first), vec![url.clone()], "tried again after the wait");
        // Each failure waits longer, and after a few it gives up (until the next launch).
        let waits: Vec<Duration> = (1..).map_while(thumb_backoff).collect();
        assert!(waits.windows(2).all(|w| w[1] > w[0]), "{waits:?}");
        assert!((2..=6).contains(&waits.len()), "bounded: {waits:?}");
        let mut t = t0;
        for _ in 1..=waits.len() {
            t += Duration::from_secs(3600);
            m.thumb_ready(url.clone(), None, t);
        }
        assert!(m.missing_thumbs(t + Duration::from_secs(86_400)).is_empty(), "gave up");
        assert_eq!(m.next_thumb_retry(), None, "nothing to wake up for");
        // A later success (another launch's cache) is taken.
        m.thumb_ready(url.clone(), Some("a.jpg".into()), t);
        assert_eq!(m.thumbs.get(&url), Some(&std::path::PathBuf::from("a.jpg")));
    }

    #[test]
    fn thumb_file_is_stable_per_url() {
        let dir = std::path::Path::new(r"C:\data\thumbs");
        assert_eq!(thumb_file(dir, "https://i/a.jpg"), thumb_file(dir, "https://i/a.jpg"));
        assert_ne!(thumb_file(dir, "https://i/a.jpg"), thumb_file(dir, "https://i/b.jpg"));
        assert!(thumb_file(dir, "https://i/a.jpg").starts_with(dir));
    }

    #[test]
    fn image_kind_from_first_bytes() {
        assert_eq!(image_ext(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("jpg"));
        assert_eq!(image_ext(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(image_ext(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(image_ext(b"<html>not an image</html>"), None);
        assert_eq!(image_ext(&[]), None);
    }

    #[test]
    fn duration_labels() {
        assert_eq!(duration_label(42.0), "0:42");
        assert_eq!(duration_label(2538.0), "42:18");
        assert_eq!(duration_label(3760.4), "1:02:40");
    }

    #[test]
    fn refresh_link_only_for_stopped_items_and_web_links() {
        assert!(can_refresh(&item(1, "a", Category::Other, Status::Failed("HTTP 403".into()))));
        assert!(can_refresh(&item(2, "a", Category::Other, Status::Paused)));
        assert!(!can_refresh(&item(3, "a", Category::Other, Status::Running)));
        assert!(!can_refresh(&item(4, "a", Category::Other, Status::Done)));
        let mut m = model();
        m.refresh_typed(ItemId(4), "  https://new.example/f.iso ".into());
        assert_eq!(m.take_refresh(ItemId(4)), Some("https://new.example/f.iso".to_string()));
        assert_eq!(m.take_refresh(ItemId(4)), None, "used once");
        m.refresh_typed(ItemId(4), "not a link".into());
        assert_eq!(m.take_refresh(ItemId(4)), None);
        assert!(m.notice.is_some(), "says why");
        m.refresh_typed(ItemId(4), "https://x/y".into());
        assert_eq!(m.take_refresh(ItemId(3)), None, "typed for another item");
    }

    #[test]
    fn closing_the_popover_applies_a_keyboard_change() {
        let mut m = model();
        m.speed_open = true;
        m.preview_speed(0.5);
        assert_eq!(m.close_speed(), Some(slider_to_bps(0.5)), "arrow keys / wheel changed it without a release");
        assert!(!m.speed_open);
        m.speed_open = true;
        assert_eq!(m.close_speed(), None, "nothing changed: nothing to apply");
    }

    #[test]
    fn settings_errors_are_kept_apart_from_notices() {
        let mut m = model();
        m.queues[1].schedule = Some(Schedule { start: 23 * 60, stop: None, days: [true; 7] });
        m.open_settings(crate::state::SettingsTab::Speed);
        m.queue_drafts[1].name = "  ".into();
        assert!(m.close_settings().is_err());
        assert!(m.settings_error.is_some(), "shown with a warning icon");
        m.queue_drafts[1].name = "Night".into();
        let (saved, _) = m.close_settings().expect("valid now");
        assert_eq!(m.settings_error, None);
        assert_eq!(m.settings, saved, "applied at once: no flash of the old accent");
    }

    #[test]
    fn deleted_queue_in_the_sidebar_falls_back_to_all() {
        let mut m = model();
        m.library = Library::Queue(1);
        let main = m.queues[0].clone();
        m.apply(Event::Queues(vec![main]));
        assert_eq!(m.library, Library::All);
    }

    #[test]
    fn hint_matches_clipboard_watch() {
        let mut m = Model::default();
        m.settings.clipboard_watch = true;
        assert!(url_hint(&m).contains("Copy any link"));
        m.settings.clipboard_watch = false;
        assert!(!url_hint(&m).contains("Copy any link"));
    }

    #[test]
    fn video_from_browser_waits_while_settings_are_open() {
        let mut m = model();
        m.queues[1].schedule = Some(Schedule { start: 23 * 60, stop: None, days: [true; 7] });
        m.open_settings(crate::state::SettingsTab::General);
        m.draft.download_dir = r"D:\x".into();
        m.apply(Event::PickMedia { url: "https://youtu.be/x".into(), info: playlist(0), choice: 0 });
        assert_eq!(m.screen, Screen::Settings, "the sheet with unsaved edits stays");
        assert_eq!(m.draft.download_dir, r"D:\x");
        m.close_settings().expect("valid");
        assert_eq!(m.screen, Screen::Picker, "then the quality choice shows");
    }

    #[test]
    fn no_toast_while_a_retry_is_pending() {
        let mut m = model();
        let mut it = m.items.iter().find(|i| i.id == ItemId(1)).cloned().unwrap();
        it.status = Status::Failed("HTTP Error 503".into());
        it.retry_at = Some(100);
        m.apply(Event::Updated(it.clone()));
        assert!(m.take_notes_at(std::time::Instant::now()).is_empty(), "it will try again: no 'failed' yet");
        it.retry_at = None;
        m.apply(Event::Updated(it));
        let notes = m.take_notes_at(std::time::Instant::now());
        assert_eq!(notes.len(), 1, "the final failure is told");
        assert_eq!(notes[0].title, "Download failed");
    }

    #[test]
    fn pause_all_includes_pending_retries() {
        let mut m = model();
        let mut it = m.items.iter().find(|i| i.id == ItemId(4)).cloned().unwrap();
        it.retry_at = Some(100);
        m.apply(Event::Updated(it));
        let mut ids = m.pause_all_ids();
        ids.sort();
        assert_eq!(ids, vec![ItemId(1), ItemId(3), ItemId(4)]);
        assert!(!m.resume_all_ids().contains(&ItemId(4)), "it resumes by itself");
    }

    #[test]
    fn retry_countdown_text() {
        let mut it = item(1, "a.zip", Category::Archive, Status::Failed("HTTP Error 503".into()));
        assert_eq!(retry_note(&it, 1000), None);
        it.retry_at = Some(1030);
        assert_eq!(retry_note(&it, 1000).as_deref(), Some("retrying in 30s"));
        assert_eq!(retry_note(&it, 1031).as_deref(), Some("retrying now"));
    }

    #[test]
    fn a_playlist_finishing_one_by_one_is_one_toast() {
        let start = std::time::Instant::now();
        let mut m = Model::default();
        for id in 1..=5 {
            m.apply(Event::Added(item(id, &format!("clip {id}.mp4"), Category::Video, Status::Queued)));
        }
        for id in 1..=4 {
            finish(&mut m, id, Status::Running);
            finish(&mut m, id, Status::Done);
            assert!(m.take_notes_at(start + std::time::Duration::from_secs(id * 10)).is_empty(), "more are coming: wait");
        }
        finish(&mut m, 5, Status::Running);
        finish(&mut m, 5, Status::Done);
        let notes = m.take_notes_at(start + std::time::Duration::from_secs(50));
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "5 downloads finished");
        // A long batch still reports after a while, so nothing waits forever.
        m.apply(Event::Added(item(9, "big.iso", Category::Archive, Status::Running)));
        m.apply(Event::Added(item(10, "small.zip", Category::Archive, Status::Running)));
        finish(&mut m, 10, Status::Done);
        let t = start + std::time::Duration::from_secs(60);
        assert!(m.take_notes_at(t).is_empty());
        assert_eq!(m.take_notes_at(t + std::time::Duration::from_secs(121)).len(), 1, "after two minutes it's told anyway");
    }

    #[test]
    fn watches_reach_the_model() {
        let mut m = Model::default();
        let w = rdm_core::watch::Watch {
            id: 1,
            url: "https://www.youtube.com/@c/videos".into(),
            name: "C".into(),
            format: MediaFormat::Video { max_height: 720 },
            max_minutes: None,
            every_hours: 6,
            queue: 0,
            last_check: 0,
            seen: vec![],
            primed: false,
        };
        m.apply(Event::Watches(vec![w.clone()]));
        assert_eq!(m.watches, vec![w.clone()]);
        let st = rdm_core::AppState { watches: vec![w.clone()], ..Default::default() };
        let mut fresh = Model::default();
        fresh.load(st);
        assert_eq!(fresh.watches.len(), 1, "a snapshot brings them too");
    }
}
