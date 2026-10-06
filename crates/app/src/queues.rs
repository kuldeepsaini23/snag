//! The Queues screen's form: queues as typed (times stay text until saved).

use rdm_core::{Queue, QueueId, Schedule};

pub const DAY_LETTERS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];

#[derive(Clone, Debug, PartialEq)]
pub struct QueueDraft {
    pub id: QueueId,
    pub name: String,
    /// Downloads at once in this queue; not shown for Main (no own limit).
    pub max_concurrent: String,
    pub scheduled: bool,
    /// `HH:MM`.
    pub start: String,
    /// `HH:MM`; empty = until midnight.
    pub stop: String,
    /// Monday … Sunday.
    pub days: [bool; 7],
}

/// `"23:30"` → minutes since midnight.
pub fn parse_hhmm(text: &str) -> Option<u16> {
    let (h, m) = text.trim().split_once(':')?;
    let digits = |s: &str, len: std::ops::RangeInclusive<usize>| len.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(h, 1..=2) || !digits(m, 2..=2) {
        return None;
    }
    let (h, m): (u16, u16) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

pub fn fmt_hhmm(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

impl QueueDraft {
    pub fn from_queue(q: &Queue) -> Self {
        // Times to start from when the user switches a schedule on.
        let (start, stop, days) = match &q.schedule {
            Some(s) => (fmt_hhmm(s.start), s.stop.map(fmt_hhmm).unwrap_or_default(), s.days),
            None => ("23:00".into(), "07:00".into(), [true; 7]),
        };
        Self {
            id: q.id,
            name: q.name.clone(),
            max_concurrent: if q.id == 0 { String::new() } else { q.max_concurrent.to_string() },
            scheduled: q.schedule.is_some(),
            start,
            stop,
            days,
        }
    }

    /// A fresh queue (night downloads, every day) with the next free id.
    pub fn new(existing: &[QueueDraft]) -> Self {
        let id = existing.iter().map(|d| d.id).max().map_or(1, |m| m + 1);
        let schedule = Schedule { start: 23 * 60, stop: Some(7 * 60), days: [true; 7] };
        Self::from_queue(&Queue { id, name: format!("Queue {id}"), max_concurrent: 1, schedule: Some(schedule) })
    }
}

/// Validates the whole form. Main keeps its unlimited share.
pub fn drafts_to_queues(drafts: &[QueueDraft]) -> Result<Vec<Queue>, String> {
    drafts
        .iter()
        .map(|d| {
            let name = d.name.trim();
            if name.is_empty() {
                return Err("Every queue needs a name".to_string());
            }
            let max_concurrent = if d.id == 0 {
                usize::MAX
            } else {
                match d.max_concurrent.trim().parse::<usize>() {
                    Ok(n) if (1..=10).contains(&n) => n,
                    _ => return Err(format!("{name}: downloads at once must be a number from 1 to 10")),
                }
            };
            let schedule = if d.scheduled {
                let start = parse_hhmm(&d.start).ok_or_else(|| format!("{name}: start time must look like 23:30"))?;
                let stop = match d.stop.trim() {
                    "" => None,
                    text => Some(parse_hhmm(text).ok_or_else(|| format!("{name}: stop time must look like 07:00 (or stay empty)"))?),
                };
                if !d.days.contains(&true) {
                    return Err(format!("{name}: pick at least one day"));
                }
                Some(Schedule { start, stop, days: d.days })
            } else {
                None
            };
            Ok(Queue { id: d.id, name: name.to_string(), max_concurrent, schedule })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn night() -> Queue {
        Queue { id: 1, name: "Night".into(), max_concurrent: 2, schedule: Some(Schedule { start: 23 * 60 + 30, stop: Some(7 * 60), days: [true, true, true, true, true, false, false] }) }
    }

    fn main() -> Queue {
        Queue { id: 0, name: "Main".into(), max_concurrent: usize::MAX, schedule: None }
    }

    #[test]
    fn parse_hhmm_cases() {
        assert_eq!(parse_hhmm("00:00"), Some(0));
        assert_eq!(parse_hhmm("7:05"), Some(7 * 60 + 5));
        assert_eq!(parse_hhmm(" 23:59 "), Some(23 * 60 + 59));
        for bad in ["24:00", "25:00", "12:60", "7", "", "ab:cd", "-1:00", "12:5x", "1:2:3"] {
            assert_eq!(parse_hhmm(bad), None, "{bad:?}");
        }
        assert_eq!(fmt_hhmm(7 * 60 + 5), "07:05");
    }

    #[test]
    fn queue_draft_round_trip() {
        let queues = vec![main(), night()];
        let drafts: Vec<_> = queues.iter().map(QueueDraft::from_queue).collect();
        assert_eq!(drafts[1].start, "23:30");
        assert_eq!(drafts[1].stop, "07:00");
        assert_eq!(drafts[0].max_concurrent, "");
        assert_eq!(drafts_to_queues(&drafts), Ok(queues));
    }

    #[test]
    fn queue_draft_rejects_bad_times() {
        let mut d = QueueDraft::from_queue(&night());
        for (start, stop) in [("25:00", "07:00"), ("7", "08:00"), ("", "08:00"), ("23:00", "8")] {
            d.start = start.into();
            d.stop = stop.into();
            let err = drafts_to_queues(std::slice::from_ref(&d)).unwrap_err();
            assert!(err.contains("Night"), "the message names the queue: {err}");
        }
        d.start = "23:00".into();
        d.stop = String::new();
        assert_eq!(drafts_to_queues(std::slice::from_ref(&d)).unwrap()[0].schedule.as_ref().unwrap().stop, None, "empty stop = until midnight");
        d.scheduled = false;
        d.start = "junk".into();
        assert_eq!(drafts_to_queues(std::slice::from_ref(&d)).unwrap()[0].schedule, None, "times are ignored without a schedule");
    }

    #[test]
    fn queue_draft_rejects_bad_name_limit_and_days() {
        let base = QueueDraft::from_queue(&night());
        let with = |f: fn(&mut QueueDraft)| {
            let mut d = base.clone();
            f(&mut d);
            drafts_to_queues(&[d])
        };
        assert!(with(|d| d.name = "  ".into()).is_err());
        assert!(with(|d| d.max_concurrent = "0".into()).is_err());
        assert!(with(|d| d.max_concurrent = "11".into()).is_err());
        assert!(with(|d| d.max_concurrent = "two".into()).is_err());
        assert!(with(|d| d.days = [false; 7]).is_err());
    }

    #[test]
    fn new_queue_gets_next_id() {
        let drafts = vec![QueueDraft::from_queue(&main()), QueueDraft::from_queue(&Queue { id: 4, ..night() })];
        let fresh = QueueDraft::new(&drafts);
        assert_eq!(fresh.id, 5);
        assert!(!fresh.name.trim().is_empty());
        assert!(drafts_to_queues(&[fresh]).is_ok(), "a new queue is valid as it comes");
    }
}
