//! Micro-animations: what moves, towards what, and whether a frame loop is needed at all.
//! Targets come from the model after every update (`sync`); the view interpolates at `now`.

use iced::animation::{Animation, Easing};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const DURATION: Duration = Duration::from_millis(180);
const PROGRESS: Duration = Duration::from_millis(320);
const APPEAR: Duration = Duration::from_millis(240);
const PULSE: Duration = Duration::from_millis(900);
/// The stats charts rising as the screen opens.
const GROW: Duration = Duration::from_millis(520);
/// Progress steps smaller than this (of the whole bar) just move: a big file gains a little every
/// tick, and easing each step would keep the frame loop running for the whole download.
const EASE_MIN: f32 = 0.02;

/// What the animations aim at, read off the model.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Targets {
    /// Index of the active filter tab.
    pub tab: usize,
    pub sidebar: bool,
    pub inspector: bool,
    /// A sheet (picker, settings, quit question) is up.
    pub sheet: bool,
    pub popover: bool,
    pub toast: bool,
    /// The toolbar search is a field.
    pub search: bool,
    /// The stats screen is showing.
    pub stats: bool,
}

/// A download row, read off the model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowTarget {
    pub id: u64,
    /// 0.0 … 1.0
    pub progress: f32,
    pub done: bool,
}

/// A row as drawn at some instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowPose {
    pub progress: f32,
    /// 0.0 (just added, invisible) … 1.0
    pub appear: f32,
    /// 1.0 the moment it finished, fading to 0.0.
    pub pulse: f32,
}

#[derive(Clone, Debug)]
struct Row {
    progress: Animation<f32>,
    appear: Animation<bool>,
    pulse: Animation<bool>,
    done: bool,
}

#[derive(Clone, Debug)]
pub struct Motion {
    /// Windows "Animation effects" off: everything jumps.
    pub reduced: bool,
    pub tab: Animation<f32>,
    pub sidebar: Animation<bool>,
    pub inspector: Animation<bool>,
    pub sheet: Animation<bool>,
    pub popover: Animation<bool>,
    pub toast: Animation<bool>,
    pub search: Animation<bool>,
    pub stats: Animation<bool>,
    last: Targets,
    rows: HashMap<u64, Row>,
}

fn float(v: f32, d: Duration) -> Animation<f32> {
    Animation::new(v).duration(d).easing(Easing::EaseOutCubic)
}

fn flag(v: bool, d: Duration) -> Animation<bool> {
    Animation::new(v).duration(d).easing(Easing::EaseOutCubic)
}

impl Motion {
    pub fn new(start: Targets, reduced: bool) -> Self {
        let d = if reduced { Duration::ZERO } else { DURATION };
        Self {
            reduced,
            tab: float(start.tab as f32, d),
            sidebar: flag(start.sidebar, d),
            inspector: flag(start.inspector, d),
            sheet: flag(start.sheet, d),
            popover: flag(start.popover, d),
            toast: flag(start.toast, d),
            search: flag(start.search, d),
            stats: flag(start.stats, if reduced { Duration::ZERO } else { GROW }),
            last: start,
            rows: HashMap::new(),
        }
    }

    fn span(&self, d: Duration) -> Duration {
        if self.reduced { Duration::ZERO } else { d }
    }

    /// Follows the rows: bars ease to visible jumps, new rows fade in (unless `animate` is off,
    /// as for a full reload), finished rows pulse once. Rows no longer there are forgotten.
    pub fn sync_rows(&mut self, rows: &[RowTarget], now: Instant, animate: bool) {
        let (progress, appear, pulse) = (self.span(PROGRESS), self.span(APPEAR), self.span(PULSE));
        self.rows.retain(|id, _| rows.iter().any(|r| r.id == *id));
        for t in rows {
            let Some(row) = self.rows.get_mut(&t.id) else {
                let fade = flag(!animate, appear).go(true, now);
                let row = Row { progress: float(t.progress, progress), appear: fade, pulse: flag(true, pulse), done: t.done };
                self.rows.insert(t.id, row);
                continue;
            };
            if row.progress.value() != t.progress {
                if row.progress.is_animating(now) || (t.progress - row.progress.value()).abs() >= EASE_MIN {
                    row.progress.go_mut(t.progress, now);
                } else {
                    row.progress = float(t.progress, progress);
                }
            }
            if t.done && !row.done && animate {
                row.pulse = flag(false, pulse).go(true, now);
            }
            row.done = t.done;
        }
    }

    /// How row `id` looks at `now` (None: not synced yet).
    pub fn row(&self, id: u64, now: Instant) -> Option<RowPose> {
        let r = self.rows.get(&id)?;
        if self.reduced {
            return Some(RowPose { progress: r.progress.value(), appear: 1.0, pulse: 0.0 });
        }
        let pulse = if r.pulse.is_animating(now) { 1.0 - r.pulse.interpolate(0.0, 1.0, now) } else { 0.0 };
        Some(RowPose { progress: r.progress.interpolate_with(|v| v, now), appear: r.appear.interpolate(0.0, 1.0, now), pulse })
    }

    /// Points every animation at its new target (only what changed starts moving).
    pub fn sync(&mut self, t: Targets, now: Instant) {
        if t.tab != self.last.tab {
            self.tab.go_mut(t.tab as f32, now);
        }
        let flags = [
            (t.sidebar, self.last.sidebar, &mut self.sidebar),
            (t.inspector, self.last.inspector, &mut self.inspector),
            (t.sheet, self.last.sheet, &mut self.sheet),
            (t.popover, self.last.popover, &mut self.popover),
            (t.toast, self.last.toast, &mut self.toast),
            (t.search, self.last.search, &mut self.search),
        ];
        for (new, old, anim) in flags {
            if new != old {
                anim.go_mut(new, now);
            }
        }
        // The charts grow in each time the screen opens; closing is instant.
        if t.stats != self.last.stats {
            let closed = flag(false, self.span(GROW));
            self.stats = if t.stats { closed.go(true, now) } else { closed };
        }
        self.last = t;
    }

    /// True while anything still moves: only then does the window need frames.
    pub fn animating(&self, now: Instant) -> bool {
        !self.reduced
            && (self.tab.is_animating(now)
                || self.sidebar.is_animating(now)
                || self.inspector.is_animating(now)
                || self.sheet.is_animating(now)
                || self.popover.is_animating(now)
                || self.toast.is_animating(now)
                || self.search.is_animating(now)
                || self.stats.is_animating(now)
                || self.rows.values().any(|r| r.progress.is_animating(now) || r.appear.is_animating(now) || r.pulse.is_animating(now)))
    }

    /// Where the tab indicator is, in tab units (1.5 = halfway between the 2nd and 3rd tab).
    pub fn tab_at(&self, now: Instant) -> f32 {
        if self.reduced { self.last.tab as f32 } else { self.tab.interpolate_with(|v| v, now) }
    }

    /// 0.0 (closed) … 1.0 (open).
    pub fn open(&self, anim: &Animation<bool>, now: Instant) -> f32 {
        if self.reduced { if anim.value() { 1.0 } else { 0.0 } } else { anim.interpolate(0.0, 1.0, now) }
    }
}

/// Where the active-tab line sits under each tab, as (x, width): `widths` are the measured tab
/// widths, `gap` the space between tabs, `inset` how far in from each side of a tab the line starts.
pub fn tab_indicator_targets(widths: &[f32], gap: f32, inset: f32) -> Vec<(f32, f32)> {
    let mut x = 0.0;
    widths
        .iter()
        .map(|&w| {
            let at = (x + inset, (w - 2.0 * inset).max(0.0));
            x += w + gap;
            at
        })
        .collect()
}

/// The line at a fractional tab position (`Motion::tab_at`), between its two neighbours.
pub fn indicator_at(targets: &[(f32, f32)], pos: f32) -> (f32, f32) {
    let Some(last) = targets.len().checked_sub(1) else { return (0.0, 0.0) };
    let pos = pos.clamp(0.0, last as f32);
    let (i, f) = (pos.floor() as usize, pos.fract());
    let (a, b) = (targets[i], targets[(i + 1).min(last)]);
    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
}

/// Windows' "Animation effects" setting (Settings → Accessibility → Visual effects).
#[cfg(windows)]
pub fn system_reduced_motion() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW};
    let mut on: i32 = 1;
    // SAFETY: the out pointer is a valid BOOL for the duration of the call.
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&mut on as *mut i32).cast(), 0) };
    ok != 0 && on == 0
}

#[cfg(not(windows))]
pub fn system_reduced_motion() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> Targets {
        Targets { tab: 0, sidebar: true, ..Default::default() }
    }

    #[test]
    fn idle_means_no_frames() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        assert!(!m.animating(t0), "nothing changed: no frame loop");
        m.sync(start(), t0);
        assert!(!m.animating(t0), "same targets again: still idle");
        m.sync_rows(&[RowTarget { id: 1, progress: 0.3, done: false }], t0, false);
        m.sync_rows(&[RowTarget { id: 1, progress: 0.3, done: false }], t0, true);
        assert!(!m.animating(t0), "rows that didn't change: still idle");
    }

    #[test]
    fn a_change_moves_then_settles() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        m.sync(Targets { tab: 2, ..start() }, t0);
        assert!(m.animating(t0 + Duration::from_millis(50)));
        let mid = m.tab_at(t0 + Duration::from_millis(60));
        assert!(mid > 0.0 && mid < 2.0, "slides through: {mid}");
        assert!(!m.animating(t0 + Duration::from_millis(400)));
        assert_eq!(m.tab_at(t0 + Duration::from_millis(400)), 2.0);
    }

    #[test]
    fn progress_eases_monotonic() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        m.sync(Targets { sidebar: false, ..start() }, t0);
        let samples: Vec<f32> = (0..=12).map(|i| m.open(&m.sidebar, t0 + Duration::from_millis(i * 20))).collect();
        assert!(samples.windows(2).all(|w| w[1] <= w[0] + f32::EPSILON), "closing only ever closes: {samples:?}");
        assert_eq!(*samples.last().unwrap(), 0.0);
    }

    #[test]
    fn reduced_motion_jumps() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), true);
        m.sync(Targets { tab: 3, sidebar: false, popover: true, ..start() }, t0);
        assert!(!m.animating(t0));
        assert_eq!(m.tab_at(t0), 3.0);
        assert_eq!(m.open(&m.sidebar, t0), 0.0);
        assert_eq!(m.open(&m.popover, t0), 1.0);
        m.sync_rows(&[row(1, 0.1, false)], t0, false);
        m.sync_rows(&[row(1, 1.0, true), row(2, 0.0, false)], t0, true);
        assert!(!m.animating(t0), "rows jump too");
        assert_eq!(m.row(1, t0), Some(RowPose { progress: 1.0, appear: 1.0, pulse: 0.0 }));
        assert_eq!(m.row(2, t0), Some(RowPose { progress: 0.0, appear: 1.0, pulse: 0.0 }));
    }

    fn row(id: u64, progress: f32, done: bool) -> RowTarget {
        RowTarget { id, progress, done }
    }

    #[test]
    fn tab_indicator_targets() {
        // Tabs 40, 60 and 50 wide, 4 apart; the line sits under the label (10 in from each side).
        let t = super::tab_indicator_targets(&[40.0, 60.0, 50.0], 4.0, 10.0);
        assert_eq!(t, vec![(10.0, 20.0), (54.0, 40.0), (118.0, 30.0)]);
        assert_eq!(indicator_at(&t, 1.0), (54.0, 40.0));
        assert_eq!(indicator_at(&t, 0.5), (32.0, 30.0), "halfway: between both, in place and width");
        assert_eq!(indicator_at(&t, 7.0), (118.0, 30.0), "past the end: the last tab");
        assert_eq!(indicator_at(&[], 1.0), (0.0, 0.0), "not measured yet: nothing");
    }

    #[test]
    fn sidebar_animation_state() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        assert_eq!(m.open(&m.sidebar, t0), 1.0);
        m.sync(Targets { sidebar: false, ..start() }, t0);
        let mid = m.open(&m.sidebar, t0 + Duration::from_millis(60));
        assert!(mid > 0.0 && mid < 1.0, "slides: {mid}");
        // Toggled back while moving: it turns around from where it is, it doesn't jump.
        let t1 = t0 + Duration::from_millis(60);
        m.sync(start(), t1);
        let back = m.open(&m.sidebar, t1 + Duration::from_millis(1));
        assert!((back - mid).abs() < 0.1, "continues from {mid}, got {back}");
        assert!(m.animating(t1 + Duration::from_millis(50)));
        assert_eq!(m.open(&m.sidebar, t1 + Duration::from_millis(400)), 1.0);
        assert!(!m.animating(t1 + Duration::from_millis(400)));
    }

    #[test]
    fn stats_charts_grow_in_once() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        assert_eq!(m.open(&m.stats, t0), 0.0);
        m.sync(Targets { stats: true, ..start() }, t0);
        let mid = m.open(&m.stats, t0 + Duration::from_millis(150));
        assert!(mid > 0.0 && mid < 1.0, "grows: {mid}");
        assert!(m.animating(t0 + Duration::from_millis(150)));
        // Slower than a panel: the bars rise for about half a second.
        assert!(m.animating(t0 + Duration::from_millis(300)));
        let done = t0 + Duration::from_secs(1);
        assert_eq!(m.open(&m.stats, done), 1.0);
        assert!(!m.animating(done), "drawn once grown: no frames while it is open");
        let mut reduced = Motion::new(start(), true);
        reduced.sync(Targets { stats: true, ..start() }, t0);
        assert_eq!(reduced.open(&reduced.stats, t0), 1.0);
    }

    #[test]
    fn rows_progress_eases_to_big_jumps_only() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        m.sync_rows(&[row(1, 0.10, false)], t0, false);
        assert!(!m.animating(t0), "a fresh load doesn't animate");
        // A tick's worth of progress just moves: no frame loop while a big file trickles in.
        m.sync_rows(&[row(1, 0.11, false)], t0, true);
        assert!(!m.animating(t0));
        assert_eq!(m.row(1, t0).unwrap().progress, 0.11);
        // A visible jump eases, never backwards.
        m.sync_rows(&[row(1, 0.60, false)], t0, true);
        let samples: Vec<f32> = (0..=20).map(|i| m.row(1, t0 + Duration::from_millis(i * 20)).unwrap().progress).collect();
        assert!(samples.windows(2).all(|w| w[1] >= w[0]), "{samples:?}");
        assert!(samples[3] > 0.11 && samples[3] < 0.60, "{samples:?}");
        assert_eq!(*samples.last().unwrap(), 0.60);
    }

    #[test]
    fn new_rows_fade_in_and_finished_rows_pulse() {
        let t0 = Instant::now();
        let mut m = Motion::new(start(), false);
        m.sync_rows(&[row(1, 0.5, false)], t0, false);
        assert_eq!(m.row(1, t0).unwrap().appear, 1.0, "rows there at launch are just there");
        m.sync_rows(&[row(1, 0.5, false), row(2, 0.0, false)], t0, true);
        let fading = m.row(2, t0 + Duration::from_millis(40)).unwrap().appear;
        assert!(fading > 0.0 && fading < 1.0, "{fading}");
        // Row 1 finishes: an accent pulse that fades out.
        m.sync_rows(&[row(1, 1.0, true), row(2, 0.0, false)], t0, true);
        let pulse = m.row(1, t0 + Duration::from_millis(40)).unwrap().pulse;
        assert!(pulse > 0.5, "{pulse}");
        let later = t0 + Duration::from_secs(2);
        assert_eq!(m.row(1, later).unwrap().pulse, 0.0);
        assert!(!m.animating(later));
        // Removed rows are forgotten.
        m.sync_rows(&[row(2, 0.0, false)], later, true);
        assert_eq!(m.row(1, later), None);
    }
}
