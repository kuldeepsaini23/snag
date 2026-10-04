use serde::{Deserialize, Serialize};

/// A moment in local time, reduced to what schedules need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Now {
    /// 0 = Monday … 6 = Sunday.
    pub weekday: u8,
    /// Minutes since local midnight.
    pub minute: u16,
}

impl Now {
    pub fn local() -> Self {
        use chrono::{Datelike, Local, Timelike};
        let t = Local::now();
        Self { weekday: t.weekday().num_days_from_monday() as u8, minute: (t.hour() * 60 + t.minute()) as u16 }
    }

    pub fn at(weekday: u8, hour: u16, minute: u16) -> Self {
        Self { weekday, minute: hour * 60 + minute }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    /// Minutes since midnight.
    pub start: u16,
    /// Minutes since midnight; `None` runs until midnight. `stop < start` crosses midnight.
    pub stop: Option<u16>,
    /// Monday … Sunday: days on which the window *starts*.
    pub days: [bool; 7],
}

impl Schedule {
    pub fn is_active(&self, now: Now) -> bool {
        let on = |day: u8| self.days[day as usize % 7];
        let yesterday = (now.weekday + 6) % 7;
        match self.stop {
            None => on(now.weekday) && now.minute >= self.start,
            Some(stop) if stop > self.start => on(now.weekday) && (self.start..stop).contains(&now.minute),
            // Crosses midnight: tonight's start, or the tail of a window that began yesterday.
            Some(stop) => (on(now.weekday) && now.minute >= self.start) || (on(yesterday) && now.minute < stop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: u8 = 0;
    const TUE: u8 = 1;

    fn only(day: u8) -> [bool; 7] {
        let mut d = [false; 7];
        d[day as usize] = true;
        d
    }

    #[test]
    fn schedule_same_day_window() {
        let s = Schedule { start: 9 * 60, stop: Some(17 * 60), days: only(MON) };
        assert!(s.is_active(Now::at(MON, 9, 0)));
        assert!(s.is_active(Now::at(MON, 16, 59)));
        assert!(!s.is_active(Now::at(MON, 17, 0)));
        assert!(!s.is_active(Now::at(MON, 8, 59)));
    }

    #[test]
    fn schedule_overnight_window() {
        let s = Schedule { start: 23 * 60, stop: Some(7 * 60), days: only(MON) };
        assert!(s.is_active(Now::at(MON, 23, 30)));
        assert!(s.is_active(Now::at(TUE, 6, 0)));
        assert!(!s.is_active(Now::at(TUE, 8, 0)));
        assert!(!s.is_active(Now::at(MON, 6, 0)), "window started Monday night, not Sunday");
    }

    #[test]
    fn schedule_no_stop_runs_until_midnight() {
        let s = Schedule { start: 20 * 60, stop: None, days: only(MON) };
        assert!(s.is_active(Now::at(MON, 23, 59)));
        assert!(!s.is_active(Now::at(TUE, 0, 30)));
    }

    #[test]
    fn schedule_wrong_day_inactive() {
        let s = Schedule { start: 0, stop: Some(24 * 60 - 1), days: only(TUE) };
        assert!(!s.is_active(Now::at(MON, 12, 0)));
        assert!(s.is_active(Now::at(TUE, 12, 0)));
    }
}
