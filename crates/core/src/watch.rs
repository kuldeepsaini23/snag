//! Watched channels and playlists: new uploads are downloaded by themselves.

use crate::model::QueueId;
use rdm_media::{Entry, MediaFormat};
use serde::{Deserialize, Serialize};

/// Only the newest uploads count (listings come newest first); older ones are never downloaded.
pub const WINDOW: usize = 50;
/// New downloads per check at most (a burst of uploads doesn't fill the disk).
pub const MAX_NEW: usize = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Watch {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub format: MediaFormat,
    /// Only videos up to this long (minutes); None = any length.
    pub max_minutes: Option<u32>,
    pub every_hours: u32,
    pub queue: QueueId,
    /// Unix seconds of the last check (0 = never).
    pub last_check: i64,
    /// Entry links already seen (downloaded or there before the watch started).
    pub seen: Vec<String>,
    /// The first check happened (it only records what's there).
    pub primed: bool,
    /// How many entries the last listing had (a playlist that grew at the end has new ones there).
    #[serde(default)]
    pub last_len: usize,
}

impl Watch {
    pub fn due(&self, now: i64) -> bool {
        self.last_check == 0 || now - self.last_check >= self.every_hours.max(1) as i64 * 3600
    }

    /// Takes a fresh listing: returns the entries to download now — never more than `MAX_NEW`,
    /// only from the `WINDOW` entries at either end — and remembers those ends.
    pub fn take_new(&mut self, entries: &[Entry], now: i64) -> Vec<Entry> {
        self.last_check = now;
        // Channels list newest first: new uploads are at the top. A playlist that adds at the
        // end shows nothing new at the top but grows: then its last entries are the new ones.
        // The middle (the back catalogue) never counts.
        let grew = entries.len().saturating_sub(self.last_len);
        let head = &entries[..entries.len().min(WINDOW)];
        let mut window: Vec<&Entry> = head.iter().collect();
        if self.primed && grew > 0 && head.iter().all(|e| self.seen.contains(&e.url)) {
            window.extend(&entries[entries.len() - grew.min(WINDOW)..]);
        }
        self.last_len = entries.len();
        let fresh: Vec<&Entry> = window.iter().copied().filter(|e| !self.seen.contains(&e.url)).collect();
        // The top of the list (and what was just taken from the end) is what can't be new again.
        self.seen = window.iter().map(|e| e.url.clone()).collect();
        if !self.primed {
            // Subscribing doesn't download the whole back catalogue.
            self.primed = true;
            return Vec::new();
        }
        let short_enough = |e: &Entry| match (self.max_minutes, e.duration) {
            (Some(max), Some(secs)) => secs <= max as f64 * 60.0,
            _ => true,
        };
        fresh.into_iter().filter(|e| short_enough(e)).take(MAX_NEW).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u32, minutes: Option<f64>) -> Entry {
        Entry { url: format!("https://y/{n}"), title: format!("V{n}"), thumbnail: None, duration: minutes.map(|m| m * 60.0) }
    }

    fn watch() -> Watch {
        Watch {
            id: 1,
            url: "https://www.youtube.com/@channel/videos".into(),
            name: "Channel".into(),
            format: MediaFormat::Video { max_height: 1080 },
            max_minutes: None,
            every_hours: 6,
            queue: 0,
            last_check: 0,
            seen: Vec::new(),
            primed: false,
            last_len: 0,
        }
    }

    #[test]
    fn a_big_channel_never_floods_the_list() {
        // Newest first, like yt-dlp lists a channel: 3000 uploads.
        let listing = |newest: u32| (0..3000).map(|k| entry(newest - k, None)).collect::<Vec<_>>();
        let mut w = watch();
        assert!(w.take_new(&listing(3000), 1).is_empty(), "subscribing downloads nothing");
        // Five new uploads since: exactly those five, not the back catalogue.
        let new: Vec<String> = w.take_new(&listing(3005), 2).iter().map(|e| e.url.clone()).collect();
        assert_eq!(new, (3001..=3005).rev().map(|n| format!("https://y/{n}")).collect::<Vec<_>>());
        // A burst of 30 new uploads: the newest 20 at most per check.
        assert_eq!(w.take_new(&listing(3035), 3).len(), MAX_NEW);
        assert!(w.seen.len() <= WINDOW, "remembers only the top of the list");
        // A playlist that adds at the end (oldest first) is caught too.
        let mut p = watch();
        let playlist = |count: u32| (1..=count).map(|n| entry(n, None)).collect::<Vec<_>>();
        assert!(p.take_new(&playlist(200), 1).is_empty());
        assert_eq!(p.take_new(&playlist(202), 2).len(), 2);
    }

    #[test]
    fn the_first_check_only_records() {
        let mut w = watch();
        assert!(w.take_new(&[entry(1, None), entry(2, None)], 100).is_empty());
        assert!(w.primed);
        assert_eq!(w.seen.len(), 2);
        assert_eq!(w.last_check, 100);
    }

    #[test]
    fn later_checks_add_only_new_uploads() {
        let mut w = watch();
        w.take_new(&[entry(1, None), entry(2, None)], 100);
        let new = w.take_new(&[entry(3, None), entry(1, None), entry(2, None)], 200);
        assert_eq!(new.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), vec!["https://y/3"]);
        assert!(w.take_new(&[entry(3, None), entry(1, None)], 300).is_empty(), "never twice");
    }

    #[test]
    fn length_limit_skips_long_videos() {
        let mut w = Watch { max_minutes: Some(20), ..watch() };
        w.take_new(&[], 1);
        let new = w.take_new(&[entry(1, Some(12.0)), entry(2, Some(95.0)), entry(3, None)], 2);
        assert_eq!(new.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), vec!["https://y/1", "https://y/3"], "unknown length is kept");
        assert!(w.seen.contains(&"https://y/2".to_string()), "skipped ones aren't offered again");
    }

    #[test]
    fn due_every_n_hours() {
        let mut w = watch();
        assert!(w.due(5), "never checked");
        w.last_check = 1000;
        assert!(!w.due(1000 + 5 * 3600));
        assert!(w.due(1000 + 6 * 3600));
    }
}
