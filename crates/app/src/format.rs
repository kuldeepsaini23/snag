use rdm_core::Status;

pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{n} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}

pub fn speed(bps: u64) -> String {
    format!("{}/s", bytes(bps))
}

/// Time left at the current speed, e.g. "45s", "3m 05s", "1h 02m".
pub fn eta(remaining: u64, bps: u64) -> Option<String> {
    if bps == 0 {
        return None;
    }
    let secs = remaining.div_ceil(bps);
    Some(match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {:02}s", s / 60, s % 60),
        s => format!("{}h {:02}m", s / 3600, (s % 3600) / 60),
    })
}

pub fn status_label(status: &Status) -> String {
    match status {
        Status::Queued => "Queued".into(),
        Status::Running => "Downloading".into(),
        Status::Paused => "Paused".into(),
        Status::Done => "Done".into(),
        Status::Failed(e) => format!("Failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_units() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn speed_suffix() {
        assert_eq!(speed(2 * 1024 * 1024), "2.0 MB/s");
        assert_eq!(speed(0), "0 B/s");
    }

    #[test]
    fn eta_seconds_minutes_hours() {
        assert_eq!(eta(1024 * 1024, 512 * 1024).as_deref(), Some("2s"));
        assert_eq!(eta(185, 1).as_deref(), Some("3m 05s"));
        assert_eq!(eta(3720, 1).as_deref(), Some("1h 02m"));
    }

    #[test]
    fn eta_none_without_speed() {
        assert_eq!(eta(100, 0), None);
    }

    #[test]
    fn status_labels() {
        assert_eq!(status_label(&Status::Running), "Downloading");
        assert_eq!(status_label(&Status::Failed("HTTP 404".into())), "Failed: HTTP 404");
    }
}
