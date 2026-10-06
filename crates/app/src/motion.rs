//! Micro-animations: what moves, towards what, and whether a frame loop is needed at all.
//! Targets come from the model after every update (`sync`); the view interpolates at `now`.

use iced::animation::{Animation, Easing};
use std::time::{Duration, Instant};

const DURATION: Duration = Duration::from_millis(180);

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
    last: Targets,
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
            last: start,
        }
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
        ];
        for (new, old, anim) in flags {
            if new != old {
                anim.go_mut(new, now);
            }
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
                || self.toast.is_animating(now))
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
    }
}
