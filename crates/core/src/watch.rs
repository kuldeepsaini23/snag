//! Watched channels and playlists: new uploads are downloaded by themselves.

use crate::model::QueueId;
use rdm_media::{Entry, MediaFormat};
use serde::{Deserialize, Serialize};

/// Links remembered per watch (oldest forgotten first): plenty for any channel page.
const SEEN_MAX: usize = 500;

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
}

impl Watch {
    pub fn due(&self, now: i64) -> bool {
        self.last_check == 0 || now - self.last_check >= self.every_hours.max(1) as i64 * 3600
    }

    /// Takes a fresh listing: returns the entries to download now and remembers everything.
    pub fn take_new(&mut self, entries: &[Entry], now: i64) -> Vec<Entry> {
        self.last_check = now;
        let fresh: Vec<&Entry> = entries.iter().filter(|e| !self.seen.contains(&e.url)).collect();
        self.seen.extend(fresh.iter().map(|e| e.url.clone()));
        if self.seen.len() > SEEN_MAX {
            let extra = self.seen.len() - SEEN_MAX;
            self.seen.drain(..extra);
        }
        if !self.primed {
            // Subscribing doesn't download the whole back catalogue.
            self.primed = true;
            return Vec::new();
        }
        let short_enough = |e: &Entry| match (self.max_minutes, e.duration) {
            (Some(max), Some(secs)) => secs <= max as f64 * 60.0,
            _ => true,
        };
        fresh.into_iter().filter(|e| short_enough(e)).cloned().collect()
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
        }
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

    #[test]
    fn seen_list_is_bounded() {
        let mut w = watch();
        let many: Vec<Entry> = (0..700).map(|n| entry(n, None)).collect();
        w.take_new(&many, 1);
        assert_eq!(w.seen.len(), SEEN_MAX);
        assert_eq!(w.seen.last().map(String::as_str), Some("https://y/699"), "the newest are kept");
    }
}
