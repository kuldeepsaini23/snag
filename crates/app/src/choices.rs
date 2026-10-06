//! Values shown in drop-down lists (iced `pick_list` needs `Display + PartialEq + Clone`).

use crate::format;
use rdm_core::{MediaFormat, QueueId};
use std::fmt;

/// A speed limit in bytes/s; 0 = none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Limit(pub u64);

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 { f.write_str("No limit") } else { write!(f, "Limit {}", format::speed(self.0)) }
    }
}

/// The footer's choices, always including the current limit (it may be a custom value from Settings).
pub fn speed_presets(current: u64) -> Vec<Limit> {
    const KB: u64 = 1024;
    let mut list: Vec<Limit> = [0, 256 * KB, 512 * KB, 1024 * KB, 2048 * KB, 5120 * KB, 10240 * KB, current].map(Limit).to_vec();
    list.sort();
    list.dedup();
    list
}

/// The preferred video quality; `None` = the best available.
#[derive(Clone, Debug, PartialEq)]
pub struct Quality(pub Option<MediaFormat>);

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            None => f.write_str("Best available"),
            Some(MediaFormat::Video { max_height }) => write!(f, "Up to {max_height}p"),
            Some(MediaFormat::AudioMp3) => f.write_str("Audio only (MP3)"),
        }
    }
}

pub fn quality_choices() -> Vec<Quality> {
    let mut list = vec![Quality(None)];
    list.extend([2160, 1440, 1080, 720, 480, 360].map(|h| Quality(Some(MediaFormat::Video { max_height: h }))));
    list.push(Quality(Some(MediaFormat::AudioMp3)));
    list
}

/// A queue in an item's "move to" list.
#[derive(Clone, Debug, PartialEq)]
pub struct QueueChoice {
    pub id: QueueId,
    pub name: String,
}

impl fmt::Display for QueueChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_presets_include_custom_value() {
        let plain = speed_presets(0);
        assert_eq!(plain.first(), Some(&Limit(0)));
        assert!(plain.contains(&Limit(1024 * 1024)));
        let custom = speed_presets(300 * 1024);
        assert!(custom.contains(&Limit(300 * 1024)), "custom value is listed");
        assert!(custom.windows(2).all(|w| w[0] < w[1]), "sorted, no duplicates: {custom:?}");
        assert_eq!(speed_presets(1024 * 1024).len(), plain.len(), "a preset isn't listed twice");
        assert_eq!(Limit(0).to_string(), "No limit");
        assert_eq!(Limit(512 * 1024).to_string(), "Limit 512.0 KB/s");
    }

    #[test]
    fn quality_labels() {
        let labels: Vec<String> = quality_choices().iter().map(ToString::to_string).collect();
        assert_eq!(labels.first().unwrap(), "Best available");
        assert!(labels.contains(&"Up to 1080p".to_string()));
        assert_eq!(labels.last().unwrap(), "Audio only (MP3)");
    }
}
