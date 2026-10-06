//! Values shown in drop-down lists (iced `pick_list` needs `Display + PartialEq + Clone`).

use rdm_core::{MediaFormat, QueueId};
use std::fmt;

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

/// A quality option in the playlist's "Quality for all" list.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionChoice {
    pub index: usize,
    pub label: String,
}

impl fmt::Display for OptionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_labels() {
        let labels: Vec<String> = quality_choices().iter().map(ToString::to_string).collect();
        assert_eq!(labels.first().unwrap(), "Best available");
        assert!(labels.contains(&"Up to 1080p".to_string()));
        assert_eq!(labels.last().unwrap(), "Audio only (MP3)");
    }
}
