//! The stats screen's numbers: data per day, share by category, top sites, totals.
//! Bytes per day come from the manager's day counter; days from before that counter existed
//! fall back to the finished items added that day. Pure: the screen only draws these.

use chrono::{Datelike, NaiveDate, TimeZone};
use rdm_core::{Category, DayTotal, Item, Kind, Status};
use std::collections::BTreeMap;

/// The screen's range switch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Range {
    #[default]
    Week,
    Month,
    All,
}

impl Range {
    pub const ALL: [Range; 3] = [Range::Week, Range::Month, Range::All];

    pub fn label(self) -> &'static str {
        match self {
            Range::Week => "7 days",
            Range::Month => "30 days",
            Range::All => "All",
        }
    }
}

/// "All" longer than this many days shows a bar per month instead of per day.
const MAX_DAY_BARS: i64 = 60;

/// One bar of the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    /// "Mon", "Oct 7", or "Oct" for a month.
    pub label: String,
    pub bytes: u64,
}

/// A site in the top list.
#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    pub host: String,
    pub bytes: u64,
    pub files: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    /// Oldest first; the last bar is today (or this month).
    pub bars: Vec<Bar>,
    /// The bars are months.
    pub monthly: bool,
    /// Finished files' bytes per category, largest first (no empty ones).
    pub categories: Vec<(Category, u64)>,
    /// The 5 sites most downloaded from, by bytes.
    pub sites: Vec<Site>,
    /// Everything downloaded in the range.
    pub total: u64,
    /// Files finished in the range.
    pub files: usize,
    /// Bytes per second while something was downloading (None: no time recorded).
    pub avg_bps: Option<u64>,
}

/// The local day an item was added on.
pub fn item_day(item: &Item) -> Option<NaiveDate> {
    chrono::Local.timestamp_opt(item.added, 0).single().map(|t| t.date_naive())
}

fn day_key(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// A finished item's size.
fn size(item: &Item) -> u64 {
    item.total.unwrap_or(item.downloaded)
}

/// Where a download came from, as the top list names it.
fn site(item: &Item) -> String {
    if item.kind == Kind::Torrent { "Torrents".into() } else { crate::view::host(&item.url) }
}

/// The sidebar's name for a category.
pub fn category_label(c: Category) -> &'static str {
    match c {
        Category::Video => "Videos",
        Category::Music => "Music",
        Category::Image => "Images",
        Category::Archive => "Archives",
        Category::Document => "Documents",
        Category::Program => "Programs",
        Category::Other => "Other",
    }
}

impl crate::state::Model {
    /// The sidebar's "Stats": the screen replaces the list (popovers close).
    pub fn open_stats(&mut self) {
        self.stats_open = true;
        self.speed_open = false;
        self.help_open = false;
    }

    /// Back to the list (a library, a filter tab, Escape).
    pub fn close_stats(&mut self) {
        self.stats_open = false;
    }
}

pub fn compute(items: &[Item], daily: &BTreeMap<String, DayTotal>, range: Range, today: NaiveDate) -> Stats {
    let finished: Vec<(&Item, NaiveDate)> = items.iter().filter(|i| i.status == Status::Done).filter_map(|i| Some((i, item_day(i)?))).collect();
    let counted_from = daily.keys().next().and_then(|k| NaiveDate::parse_from_str(k, "%Y-%m-%d").ok());
    let first = match range {
        Range::Week => today - chrono::Days::new(6),
        Range::Month => today - chrono::Days::new(29),
        Range::All => finished.iter().map(|(_, d)| *d).chain(counted_from).min().unwrap_or(today).min(today),
    };

    // Bytes on one day: the counter's from the day it started, the finished items' before it.
    let day_bytes = |d: NaiveDate| -> u64 {
        if counted_from.is_some_and(|c| d >= c) {
            daily.get(&day_key(d)).map_or(0, |t| t.bytes)
        } else {
            finished.iter().filter(|(_, on)| *on == d).map(|(i, _)| size(i)).sum()
        }
    };
    let days: Vec<NaiveDate> = first.iter_days().take_while(|d| *d <= today).collect();
    let monthly = (today - first).num_days() >= MAX_DAY_BARS;
    let bars: Vec<Bar> = if monthly {
        let mut bars: Vec<(NaiveDate, Bar)> = Vec::new();
        for d in &days {
            let month = d.with_day(1).unwrap_or(*d);
            match bars.last_mut() {
                Some((m, bar)) if *m == month => bar.bytes += day_bytes(*d),
                _ => bars.push((month, Bar { label: d.format("%b").to_string(), bytes: day_bytes(*d) })),
            }
        }
        bars.into_iter().map(|(_, b)| b).collect()
    } else {
        let label = |d: &NaiveDate| if range == Range::Week { d.format("%a").to_string() } else { d.format("%b %-d").to_string() };
        days.iter().map(|d| Bar { label: label(d), bytes: day_bytes(*d) }).collect()
    };

    let in_range: Vec<&Item> = finished.iter().filter(|(_, d)| *d >= first && *d <= today).map(|(i, _)| *i).collect();
    let mut categories: Vec<(Category, u64)> = Vec::new();
    let mut sites: Vec<Site> = Vec::new();
    for i in &in_range {
        match categories.iter_mut().find(|(c, _)| *c == i.category) {
            Some((_, b)) => *b += size(i),
            None => categories.push((i.category, size(i))),
        }
        let host = site(i);
        match sites.iter_mut().find(|s| s.host == host) {
            Some(s) => (s.bytes, s.files) = (s.bytes + size(i), s.files + 1),
            None => sites.push(Site { host, bytes: size(i), files: 1 }),
        }
    }
    categories.retain(|(_, b)| *b > 0);
    categories.sort_by_key(|(_, bytes)| std::cmp::Reverse(*bytes));
    sites.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(b.files.cmp(&a.files)));
    sites.truncate(5);

    let timed: (u64, u64) = days.iter().filter_map(|d| daily.get(&day_key(*d))).fold((0, 0), |(b, ms), t| (b + t.bytes, ms + t.ms));
    Stats {
        total: bars.iter().map(|b| b.bytes).sum(),
        bars,
        monthly,
        categories,
        sites,
        files: in_range.len(),
        avg_bps: (timed.1 > 0).then(|| timed.0 * 1000 / timed.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    /// A finished item added at noon (local) on `on`.
    fn done(id: u64, url: &str, category: Category, bytes: u64, on: NaiveDate) -> Item {
        let added = chrono::Local.from_local_datetime(&on.and_hms_opt(12, 0, 0).unwrap()).single().unwrap().timestamp();
        let mut i: Item = serde_json::from_value(serde_json::json!({
            "id": id, "url": url, "name": "f", "category": "Other", "status": "Done",
            "dest": null, "downloaded": bytes, "total": bytes, "queue": 0, "added": added
        }))
        .unwrap();
        i.category = category;
        i
    }

    fn counter(days: &[(NaiveDate, u64, u64)]) -> BTreeMap<String, DayTotal> {
        days.iter().map(|(d, bytes, ms)| (day_key(*d), DayTotal { bytes: *bytes, ms: *ms })).collect()
    }

    const TODAY: (i32, u32, u32) = (2026, 10, 7); // a Wednesday

    fn today() -> NaiveDate {
        day(TODAY.0, TODAY.1, TODAY.2)
    }

    #[test]
    fn a_week_is_seven_daily_bars_ending_today() {
        let daily = counter(&[(day(2026, 10, 7), 300, 1000), (day(2026, 10, 2), 100, 1000), (day(2026, 9, 1), 999, 1000)]);
        let s = compute(&[], &daily, Range::Week, today());
        assert!(!s.monthly);
        let labels: Vec<&str> = s.bars.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"]);
        let bytes: Vec<u64> = s.bars.iter().map(|b| b.bytes).collect();
        assert_eq!(bytes, [0, 100, 0, 0, 0, 0, 300]);
        assert_eq!(s.total, 400, "September is outside the week");
        assert_eq!(s.avg_bps, Some(200), "400 bytes in 2 s");
    }

    #[test]
    fn days_before_the_counter_use_finished_items() {
        // The counter started on Oct 5; Oct 3's download only exists as an item.
        let daily = counter(&[(day(2026, 10, 5), 50, 0)]);
        let items = [done(1, "https://a.com/x", Category::Video, 70, day(2026, 10, 3)), done(2, "https://a.com/y", Category::Video, 40, day(2026, 10, 5))];
        let s = compute(&items, &daily, Range::Week, today());
        let bytes: Vec<u64> = s.bars.iter().map(|b| b.bytes).collect();
        assert_eq!(bytes, [0, 0, 70, 0, 50, 0, 0], "Oct 5 counts the counter, not the item again");
        assert_eq!(s.avg_bps, None, "no time recorded");
    }

    #[test]
    fn category_share_is_finished_files_largest_first() {
        let mut running = done(4, "https://a.com/r", Category::Archive, 5000, today());
        running.status = Status::Running;
        let items = [
            done(1, "https://a.com/1", Category::Video, 300, today()),
            done(2, "https://a.com/2", Category::Music, 100, today()),
            done(3, "https://a.com/3", Category::Video, 200, today()),
            running,
            done(5, "https://a.com/5", Category::Document, 900, day(2026, 8, 1)),
        ];
        let s = compute(&items, &BTreeMap::new(), Range::Week, today());
        assert_eq!(s.categories, vec![(Category::Video, 500), (Category::Music, 100)]);
        assert_eq!(s.files, 3);
    }

    #[test]
    fn top_sites_by_bytes_five_at_most() {
        let mut items: Vec<Item> = ["a", "b", "c", "d", "e", "f"]
            .iter()
            .enumerate()
            .map(|(k, h)| done(k as u64, &format!("https://www.{h}.com/f"), Category::Other, 10 * (k as u64 + 1), today()))
            .collect();
        items.push(done(10, "https://a.com/again", Category::Other, 100, today()));
        let mut torrent = done(11, "magnet:?xt=urn:btih:abc", Category::Other, 35, today());
        torrent.kind = Kind::Torrent;
        items.push(torrent);
        let s = compute(&items, &BTreeMap::new(), Range::Week, today());
        let top: Vec<(&str, u64, usize)> = s.sites.iter().map(|x| (x.host.as_str(), x.bytes, x.files)).collect();
        assert_eq!(top, [("a.com", 110, 2), ("f.com", 60, 1), ("e.com", 50, 1), ("d.com", 40, 1), ("Torrents", 35, 1)]);
    }

    #[test]
    fn the_range_decides_what_counts() {
        let items = [done(1, "https://a.com/1", Category::Video, 10, day(2026, 9, 20)), done(2, "https://a.com/2", Category::Video, 1, today())];
        let week = compute(&items, &BTreeMap::new(), Range::Week, today());
        assert_eq!((week.files, week.total), (1, 1));
        let month = compute(&items, &BTreeMap::new(), Range::Month, today());
        assert_eq!(month.bars.len(), 30);
        assert_eq!((month.files, month.total), (2, 11));
        assert_eq!(month.bars[0].label, "Sep 8");
        let all = compute(&items, &BTreeMap::new(), Range::All, today());
        assert_eq!(all.bars.len(), 18, "Sep 20 … Oct 7, a bar a day");
        assert_eq!(all.total, 11);
    }

    #[test]
    fn a_long_history_is_shown_by_month() {
        let daily = counter(&[(day(2025, 11, 15), 5, 0), (day(2026, 10, 1), 7, 0), (day(2026, 10, 7), 1, 0)]);
        let s = compute(&[], &daily, Range::All, today());
        assert!(s.monthly);
        assert_eq!(s.bars.len(), 12, "Nov 2025 … Oct 2026");
        assert_eq!(s.bars[0], Bar { label: "Nov".into(), bytes: 5 });
        assert_eq!(s.bars[11], Bar { label: "Oct".into(), bytes: 8 });
        assert_eq!(s.total, 13);
    }

    #[test]
    fn the_stats_screen_hides_the_inspector_and_comes_back() {
        let mut m = crate::state::Model { items: vec![done(1, "https://a.com/1", Category::Video, 1, today())], ..Default::default() };
        m.selected = Some(rdm_core::ItemId(1));
        assert!(m.inspected().is_some());
        m.open_stats();
        assert!(m.inspected().is_none());
        assert!(crate::view::motion_targets(&m).stats && !crate::view::motion_targets(&m).inspector);
        m.close_stats();
        assert!(m.inspected().is_some(), "the selection is kept");
    }

    #[test]
    fn category_labels() {
        assert_eq!(category_label(Category::Video), "Videos");
        assert_eq!(category_label(Category::Other), "Other");
    }

    #[test]
    fn nothing_yet_is_one_empty_day() {
        let s = compute(&[], &BTreeMap::new(), Range::All, today());
        assert_eq!(s.bars, vec![Bar { label: "Oct 7".into(), bytes: 0 }]);
        assert_eq!((s.total, s.files, s.avg_bps), (0, 0, None));
        assert!(s.categories.is_empty() && s.sites.is_empty());
    }
}
