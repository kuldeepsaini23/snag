//! The last 60 seconds of download speed, per item and in total, for the sparklines.
//! One slot per second in a fixed ring: recording and reading never allocate or grow.

use rdm_core::{Item, ItemId, Status};
use std::collections::HashMap;
use std::time::Instant;

/// Seconds of history kept (and drawn).
pub const WINDOW: usize = 60;

/// One speed per second. Seconds without a sample keep the speed last recorded (the engine only
/// reports a speed when it changes); seconds before the first sample are 0.
#[derive(Clone, Debug, PartialEq)]
pub struct History {
    slots: [u64; WINDOW],
    /// (first, latest) second recorded; None until the first sample.
    span: Option<(u64, u64)>,
}

impl Default for History {
    fn default() -> Self {
        Self { slots: [0; WINDOW], span: None }
    }
}

impl History {
    /// The speed at second `sec`. A second recorded twice keeps the later value; an older second
    /// than the latest is ignored (the clock never runs backwards here).
    pub fn record(&mut self, sec: u64, bps: u64) {
        const W: u64 = WINDOW as u64;
        let first = match self.span {
            None => sec,
            Some((_, last)) if sec < last => return,
            Some((first, last)) => {
                // The seconds in between had no sample: the old speed held (only the last
                // minute of them matters).
                let held = self.slots[(last % W) as usize];
                for s in (last + 1).max(sec.saturating_sub(W - 1))..sec {
                    self.slots[(s % W) as usize] = held;
                }
                first
            }
        };
        self.slots[(sec % W) as usize] = bps;
        self.span = Some((first, sec));
    }

    /// The `WINDOW` speeds ending at second `now`, oldest first.
    pub fn values(&self, now: u64) -> [u64; WINDOW] {
        const W: u64 = WINDOW as u64;
        let mut out = [0; WINDOW];
        let Some((first, last)) = self.span else { return out };
        for (k, v) in out.iter_mut().enumerate() {
            let Some(s) = (now + k as u64 + 1).checked_sub(W) else { continue };
            *v = if s < first || last.saturating_sub(s) >= W {
                0
            } else {
                self.slots[(s.min(last) % W) as usize]
            };
        }
        out
    }

    /// The speed last recorded.
    pub fn latest(&self) -> u64 {
        self.span.map_or(0, |(_, last)| self.slots[(last % WINDOW as u64) as usize])
    }
}

/// The histories the window keeps: one per item that has downloaded, and the total.
#[derive(Clone, Debug)]
pub struct Speeds {
    /// Second 0.
    base: Instant,
    pub total: History,
    items: HashMap<ItemId, History>,
}

impl Default for Speeds {
    fn default() -> Self {
        Self { base: Instant::now(), total: History::default(), items: HashMap::new() }
    }
}

impl Speeds {
    pub fn sec(&self, at: Instant) -> u64 {
        at.saturating_duration_since(self.base).as_secs()
    }

    /// Samples every running item (and any that ran before, so its line drops to 0) and the total.
    /// Items no longer in the list are forgotten.
    pub fn sample(&mut self, items: &[Item], at: Instant) {
        let sec = self.sec(at);
        self.items.retain(|id, _| items.iter().any(|i| i.id == *id));
        let mut total = 0;
        for i in items {
            let running = i.status == Status::Running;
            let speed = if running { i.speed_bps } else { 0 };
            total += speed;
            if running {
                self.items.entry(i.id).or_default().record(sec, speed);
            } else if let Some(h) = self.items.get_mut(&i.id) {
                h.record(sec, 0);
            }
        }
        self.total.record(sec, total);
    }

    pub fn item(&self, id: ItemId) -> Option<&History> {
        self.items.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn empty_history_is_flat_zero() {
        let h = History::default();
        assert_eq!(h.values(100), [0; WINDOW]);
        assert_eq!(h.latest(), 0);
    }

    #[test]
    fn samples_land_in_their_second_and_carry_forward() {
        let mut h = History::default();
        h.record(10, 5);
        h.record(12, 7);
        let v = h.values(13);
        // Oldest first: the last slot is second 13.
        assert_eq!(&v[WINDOW - 4..], &[5, 5, 7, 7], "second 11 had no sample: 5 carries on; 13 carries 7");
        assert_eq!(v[WINDOW - 5], 0, "before the first sample");
        assert_eq!(h.latest(), 7);
    }

    #[test]
    fn same_second_keeps_the_latest_and_old_seconds_are_ignored() {
        let mut h = History::default();
        h.record(10, 5);
        h.record(10, 9);
        h.record(8, 100);
        assert_eq!(h.values(10)[WINDOW - 1], 9);
        assert_eq!(h.values(10)[WINDOW - 3], 0, "a late sample doesn't rewrite the past");
    }

    #[test]
    fn the_ring_keeps_only_the_last_minute() {
        let mut h = History::default();
        for s in 0..200 {
            h.record(s, s);
        }
        let v = h.values(199);
        assert_eq!(v[0], 140);
        assert_eq!(v[WINDOW - 1], 199);
        assert!(v.windows(2).all(|w| w[1] == w[0] + 1), "{v:?}");
        // A long gap: the last speed fills the whole window.
        assert_eq!(h.values(1000), [199; WINDOW]);
        h.record(1000, 3);
        assert_eq!(h.values(1000)[WINDOW - 2], 199, "the gap before it carries the old speed");
        assert_eq!(h.values(1000)[WINDOW - 1], 3);
    }

    fn item(id: u64, status: Status, speed: u64) -> Item {
        let mut i: Item = serde_json::from_value(serde_json::json!({
            "id": id, "url": "https://x/f", "name": "f", "category": "Other", "status": "Queued",
            "dest": null, "downloaded": 0, "total": null, "queue": 0, "added": 0
        }))
        .unwrap();
        (i.status, i.speed_bps) = (status, speed);
        i
    }

    #[test]
    fn speeds_sample_items_and_total() {
        let mut s = Speeds::default();
        let t0 = s.base;
        s.sample(&[item(1, Status::Running, 100), item(2, Status::Running, 50), item(3, Status::Paused, 0)], t0);
        assert_eq!(s.total.latest(), 150);
        assert_eq!(s.item(ItemId(1)).map(History::latest), Some(100));
        assert!(s.item(ItemId(3)).is_none(), "never ran: no history");
        // Item 1 pauses: its line drops to 0; item 2 is removed: forgotten.
        let t1 = t0 + Duration::from_secs(1);
        s.sample(&[item(1, Status::Paused, 0), item(3, Status::Paused, 0)], t1);
        assert_eq!(s.item(ItemId(1)).map(History::latest), Some(0));
        assert!(s.item(ItemId(2)).is_none());
        assert_eq!(s.total.latest(), 0);
        assert_eq!(s.total.values(s.sec(t1))[WINDOW - 2], 150);
    }
}
