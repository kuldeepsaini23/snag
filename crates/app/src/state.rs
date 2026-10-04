use rdm_core::{AppState, Event, Item, ItemId, MediaFormat, MediaInfo, Settings, Status};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Downloads,
    Settings,
    Picker,
}

/// Quality choice for a video/audio link (or a whole playlist).
#[derive(Clone, Debug, PartialEq)]
pub struct Picker {
    pub url: String,
    pub info: MediaInfo,
    /// Index into `info.options`.
    pub choice: usize,
}

impl Picker {
    /// What to add: (url, title, format), one per video.
    pub fn requests(&self) -> Vec<(String, String, MediaFormat)> {
        let Some(option) = self.info.options.get(self.choice) else { return Vec::new() };
        if self.info.entries.is_empty() {
            return vec![(self.url.clone(), self.info.title.clone(), option.format.clone())];
        }
        self.info.entries.iter().map(|e| (e.url.clone(), e.title.clone(), option.format.clone())).collect()
    }
}

/// Settings as typed into the form (numbers stay text until saved).
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub download_dir: String,
    pub connections: String,
    pub max_concurrent: String,
    /// KB/s, 0 = unlimited.
    pub speed_limit_kbps: String,
    pub sort_into_folders: bool,
    pub start_immediately: bool,
}

impl Draft {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            download_dir: s.download_dir.display().to_string(),
            connections: s.connections.to_string(),
            max_concurrent: s.max_concurrent.to_string(),
            speed_limit_kbps: (s.speed_limit_bps / 1024).to_string(),
            sort_into_folders: s.sort_into_folders,
            start_immediately: s.start_immediately,
        }
    }

    /// Validates the form and applies it on top of `base` (fields the form doesn't show are kept).
    pub fn to_settings(&self, base: &Settings) -> Result<Settings, String> {
        fn number(field: &str, value: &str, range: std::ops::RangeInclusive<u64>) -> Result<u64, String> {
            let n: u64 = value.trim().parse().map_err(|_| format!("{field} must be a whole number"))?;
            if range.contains(&n) { Ok(n) } else { Err(format!("{field} must be between {} and {}", range.start(), range.end())) }
        }
        let dir = self.download_dir.trim();
        if dir.is_empty() {
            return Err("Download folder can't be empty".into());
        }
        Ok(Settings {
            download_dir: PathBuf::from(dir),
            connections: number("Connections", &self.connections, 1..=16)? as usize,
            max_concurrent: number("Downloads at once", &self.max_concurrent, 1..=10)? as usize,
            speed_limit_bps: number("Speed limit", &self.speed_limit_kbps, 0..=10_000_000)? * 1024,
            sort_into_folders: self.sort_into_folders,
            start_immediately: self.start_immediately,
            ..base.clone()
        })
    }
}

/// Everything the window shows. Pure data: no iced, no manager.
#[derive(Clone, Debug)]
pub struct Model {
    pub items: Vec<Item>,
    pub settings: Settings,
    pub url: String,
    pub selected: Option<ItemId>,
    pub screen: Screen,
    pub draft: Draft,
    pub notice: Option<String>,
    pub picker: Option<Picker>,
    /// A video link is being read (yt-dlp probe).
    pub probing: bool,
}

impl Default for Model {
    fn default() -> Self {
        let settings = Settings::default();
        Self {
            items: Vec::new(),
            draft: Draft::from_settings(&settings),
            settings,
            url: String::new(),
            selected: None,
            screen: Screen::Downloads,
            notice: None,
            picker: None,
            probing: false,
        }
    }
}

impl Model {
    /// Replaces everything with a full snapshot from the manager.
    pub fn load(&mut self, state: AppState) {
        self.items = state.items;
        self.draft = Draft::from_settings(&state.settings);
        self.settings = state.settings;
        if self.selected.is_some_and(|id| !self.items.iter().any(|i| i.id == id)) {
            self.selected = None;
        }
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Added(item) => self.items.push(item),
            Event::Updated(item) => match self.items.iter_mut().find(|i| i.id == item.id) {
                Some(slot) => *slot = item,
                None => self.items.push(item),
            },
            Event::Removed(id) => {
                self.items.retain(|i| i.id != id);
                if self.selected == Some(id) {
                    self.selected = None;
                }
            }
            Event::Settings(s) => {
                self.draft = Draft::from_settings(&s);
                self.settings = s;
            }
        }
    }

    /// (number running, combined speed in bytes/s)
    pub fn totals(&self) -> (usize, u64) {
        self.items
            .iter()
            .filter(|i| i.status == Status::Running)
            .fold((0, 0), |(n, speed), i| (n + 1, speed + i.speed_bps))
    }

    pub fn dest_of(&self, id: ItemId) -> Option<PathBuf> {
        self.items.iter().find(|i| i.id == id).and_then(|i| i.dest.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdm_core::Category;
    use rdm_media::{Entry, QualityOption};

    fn item(id: u64, status: Status, speed: u64) -> Item {
        Item {
            id: ItemId(id),
            url: format!("http://x/{id}"),
            name: format!("{id}.bin"),
            category: Category::Other,
            status,
            dest: None,
            downloaded: 0,
            total: None,
            speed_bps: speed,
            queue: 0,
            added: 0,
            kind: Default::default(),
        }
    }

    #[test]
    fn apply_added_appends() {
        let mut m = Model::default();
        m.apply(Event::Added(item(1, Status::Queued, 0)));
        m.apply(Event::Added(item(2, Status::Queued, 0)));
        assert_eq!(m.items.iter().map(|i| i.id.0).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn apply_updated_replaces_in_place() {
        let mut m = Model::default();
        m.apply(Event::Added(item(1, Status::Queued, 0)));
        m.apply(Event::Added(item(2, Status::Queued, 0)));
        m.apply(Event::Updated(item(1, Status::Running, 10)));
        assert_eq!(m.items[0].status, Status::Running);
        assert_eq!(m.items[0].speed_bps, 10);
        assert_eq!(m.items.len(), 2);
    }

    #[test]
    fn apply_removed_deletes_and_clears_selection() {
        let mut m = Model::default();
        m.apply(Event::Added(item(1, Status::Queued, 0)));
        m.selected = Some(ItemId(1));
        m.apply(Event::Removed(ItemId(1)));
        assert!(m.items.is_empty());
        assert_eq!(m.selected, None);
    }

    #[test]
    fn apply_settings_updates_and_resets_draft() {
        let mut m = Model::default();
        let s = Settings { connections: 4, ..Settings::default() };
        m.draft.connections = "junk".into();
        m.apply(Event::Settings(s.clone()));
        assert_eq!(m.settings, s);
        assert_eq!(m.draft.connections, "4");
    }

    #[test]
    fn load_replaces_everything() {
        let mut m = Model::default();
        m.apply(Event::Added(item(9, Status::Done, 0)));
        let mut st = AppState::default();
        st.items.push(item(1, Status::Paused, 0));
        st.settings.max_concurrent = 7;
        m.load(st);
        assert_eq!(m.items.len(), 1);
        assert_eq!(m.items[0].id, ItemId(1));
        assert_eq!(m.draft.max_concurrent, "7");
    }

    #[test]
    fn draft_round_trip() {
        let s = Settings { speed_limit_bps: 2048 * 1024, connections: 6, max_concurrent: 2, ..Settings::default() };
        let d = Draft::from_settings(&s);
        assert_eq!(d.speed_limit_kbps, "2048");
        assert_eq!(d.to_settings(&s).unwrap(), s);
    }

    #[test]
    fn draft_rejects_bad_numbers() {
        let s = Settings::default();
        let bad = |f: fn(&mut Draft)| {
            let mut d = Draft::from_settings(&s);
            f(&mut d);
            d.to_settings(&s)
        };
        assert!(bad(|d| d.connections = "abc".into()).is_err());
        assert!(bad(|d| d.connections = "0".into()).is_err());
        assert!(bad(|d| d.connections = "17".into()).is_err());
        assert!(bad(|d| d.max_concurrent = "0".into()).is_err());
        assert!(bad(|d| d.speed_limit_kbps = "-5".into()).is_err());
        assert!(bad(|d| d.download_dir = "  ".into()).is_err());
    }

    fn info(entries: Vec<Entry>) -> MediaInfo {
        MediaInfo {
            title: "Clip".into(),
            duration: None,
            options: vec![
                QualityOption { label: "720p".into(), format: MediaFormat::Video { max_height: 720 }, approx_size: None },
                QualityOption { label: "Audio only (MP3)".into(), format: MediaFormat::AudioMp3, approx_size: None },
            ],
            entries,
        }
    }

    #[test]
    fn picker_single_video_makes_one_request() {
        let p = Picker { url: "https://youtu.be/x".into(), info: info(vec![]), choice: 1 };
        assert_eq!(p.requests(), vec![("https://youtu.be/x".to_string(), "Clip".to_string(), MediaFormat::AudioMp3)]);
    }

    #[test]
    fn picker_playlist_makes_one_request_per_entry() {
        let entries = vec![Entry { url: "https://y/1".into(), title: "One".into() }, Entry { url: "https://y/2".into(), title: "Two".into() }];
        let p = Picker { url: "https://y/list".into(), info: info(entries), choice: 0 };
        let reqs = p.requests();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[1], ("https://y/2".to_string(), "Two".to_string(), MediaFormat::Video { max_height: 720 }));
    }

    #[test]
    fn totals_count_running_and_sum_speed() {
        let mut m = Model::default();
        m.apply(Event::Added(item(1, Status::Running, 100)));
        m.apply(Event::Added(item(2, Status::Running, 50)));
        m.apply(Event::Added(item(3, Status::Paused, 0)));
        assert_eq!(m.totals(), (2, 150));
    }
}
