use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub start: u64,
    /// Exclusive.
    pub end: u64,
    pub written: u64,
    /// Bytes handed to a worker's write that hasn't completed yet. Never persisted:
    /// only `written` bytes are known to be in the file.
    #[serde(skip, default)]
    pub inflight: u64,
}

impl Segment {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end, written: 0, inflight: 0 }
    }
    pub fn pos(&self) -> u64 {
        self.start + self.written + self.inflight
    }
    pub fn remaining(&self) -> u64 {
        self.end.saturating_sub(self.pos())
    }
}

pub fn plan(size: u64, n: usize) -> Vec<Segment> {
    if size == 0 {
        return Vec::new();
    }
    let n = (n.max(1) as u64).min(size);
    let base = size / n;
    (0..n)
        .map(|i| {
            let start = i * base;
            let end = if i == n - 1 { size } else { start + base };
            Segment::new(start, end)
        })
        .collect()
}

pub fn split_largest(segments: &mut Vec<Segment>, min_split: u64) -> Option<usize> {
    let (idx, remaining, pos, end) = segments
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.remaining(), s.pos(), s.end))
        .max_by_key(|t| t.1)?;
    if remaining < min_split.max(1) * 2 {
        return None;
    }
    let mid = pos + remaining / 2;
    segments[idx].end = mid;
    segments.push(Segment::new(mid, end));
    Some(segments.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_covers_whole_file_contiguously() {
        let segs = plan(10, 3);
        assert_eq!(segs, vec![Segment::new(0, 3), Segment::new(3, 6), Segment::new(6, 10)]);
    }

    #[test]
    fn plan_never_makes_more_segments_than_bytes() {
        assert_eq!(plan(2, 8).len(), 2);
        assert_eq!(plan(5, 0), vec![Segment::new(0, 5)]);
        assert!(plan(0, 8).is_empty());
    }

    #[test]
    fn split_halves_remaining_part_of_largest() {
        let mut segs = vec![Segment::new(0, 100), Segment::new(100, 120)];
        segs[0].written = 20;
        let idx = split_largest(&mut segs, 10).unwrap();
        assert_eq!(idx, 2);
        assert_eq!(segs[0].end, 60);
        assert_eq!(segs[2], Segment::new(60, 100));
    }

    #[test]
    fn split_refuses_small_segments() {
        let mut segs = vec![Segment::new(0, 30)];
        assert_eq!(split_largest(&mut segs, 16), None);
        assert_eq!(segs.len(), 1);
    }

    #[test]
    fn split_on_empty_list_is_none() {
        assert_eq!(split_largest(&mut Vec::new(), 1), None);
    }
}
