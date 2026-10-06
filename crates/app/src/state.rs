use crate::queues::{QueueDraft, drafts_to_queues};
use crate::view::{Filter, Library};
use rdm_core::{AppState, Event, Item, ItemId, MediaFormat, MediaInfo, Queue, QueueId, Settings, Status};
use rdm_media::QualityOption;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Downloads,
    /// The settings sheet (its tab is `Model::settings_tab`).
    Settings,
    Picker,
}

/// The settings sheet's sections, in nav order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    Connections,
    Speed,
    Extension,
    Tools,
}

pub const SETTINGS_TABS: [SettingsTab; 6] =
    [SettingsTab::General, SettingsTab::Appearance, SettingsTab::Connections, SettingsTab::Speed, SettingsTab::Extension, SettingsTab::Tools];

/// The picker's Video ⇄ Audio switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaTab {
    Video,
    Audio,
}

impl MediaTab {
    fn of(format: &MediaFormat) -> Self {
        if *format == MediaFormat::AudioMp3 { Self::Audio } else { Self::Video }
    }
}

/// Quality choice for a video/audio link (or a whole playlist).
#[derive(Clone, Debug, PartialEq)]
pub struct Picker {
    pub url: String,
    pub info: MediaInfo,
    /// Index into `info.options`.
    pub choice: usize,
    pub tab: MediaTab,
    /// Playlists: which entries to download (one per entry).
    pub selected: Vec<bool>,
    /// Where the downloads go.
    pub queue: QueueId,
}

impl Picker {
    pub fn new(url: String, info: MediaInfo, choice: usize, queue: QueueId) -> Self {
        let tab = info.options.get(choice).map_or(MediaTab::Video, |o| MediaTab::of(&o.format));
        let selected = vec![true; info.entries.len()];
        Self { url, info, choice, tab, selected, queue }
    }

    /// The options on the current tab, with their index into `info.options`.
    pub fn visible_options(&self) -> Vec<(usize, &QualityOption)> {
        self.info.options.iter().enumerate().filter(|(_, o)| MediaTab::of(&o.format) == self.tab).collect()
    }

    pub fn has_both_tabs(&self) -> bool {
        let audio = self.info.options.iter().filter(|o| MediaTab::of(&o.format) == MediaTab::Audio).count();
        audio > 0 && audio < self.info.options.len()
    }

    /// Switches the tab and selects its first option.
    pub fn set_tab(&mut self, tab: MediaTab) {
        self.tab = tab;
        if let Some(i) = self.info.options.iter().position(|o| MediaTab::of(&o.format) == tab) {
            self.choice = i;
        }
    }

    pub fn toggle_entry(&mut self, i: usize) {
        if let Some(on) = self.selected.get_mut(i) {
            *on = !*on;
        }
    }

    pub fn select_all(&mut self, on: bool) {
        self.selected.iter_mut().for_each(|s| *s = on);
    }

    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|s| **s).count()
    }

    /// What to add: (url, title, format), one per video (only the selected playlist entries).
    pub fn requests(&self) -> Vec<(String, String, MediaFormat)> {
        let all = self.info.requests(&self.url, self.choice);
        if self.info.entries.is_empty() {
            return all;
        }
        all.into_iter().zip(&self.selected).filter(|(_, on)| **on).map(|(r, _)| r).collect()
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
    pub clipboard_watch: bool,
    pub ask_quality: bool,
    pub preferred_quality: Option<MediaFormat>,
    /// "#rrggbb" as typed (Custom colour).
    pub accent: String,
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
            clipboard_watch: s.clipboard_watch,
            ask_quality: s.ask_quality,
            preferred_quality: s.preferred_quality.clone(),
            accent: s.accent.clone(),
        }
    }

    /// Validates the form and applies it on top of `base` (fields the form doesn't show are kept).
    pub fn to_settings(&self, base: &Settings) -> Result<Settings, String> {
        fn number(field: &str, value: &str, range: std::ops::RangeInclusive<u64>) -> Result<u64, String> {
            let n: u64 = value.trim().parse().map_err(|_| format!("{field} must be a whole number"))?;
            if range.contains(&n) { Ok(n) } else { Err(format!("{field} must be between {} and {}", range.start(), range.end())) }
        }
        let accent = self.accent.trim();
        if crate::ui::theme::parse_hex(accent).is_none() {
            return Err("Custom colour must be a hex value like #ff9f0a".into());
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
            clipboard_watch: self.clipboard_watch,
            ask_quality: self.ask_quality,
            preferred_quality: self.preferred_quality.clone(),
            accent: accent.to_string(),
            ..base.clone()
        })
    }
}

/// A link worth suggesting from the clipboard: a single http(s) URL that isn't the last one seen.
pub fn clipboard_link(text: &str, last_seen: Option<&str>) -> Option<String> {
    let link = text.trim();
    let is_url = (link.starts_with("http://") || link.starts_with("https://")) && link.len() < 4096 && !link.contains(char::is_whitespace);
    (is_url && last_seen != Some(link)).then(|| link.to_string())
}

/// Explorer argument that selects `path`. Quoted, because file names can contain
/// commas and spaces, which Explorer would otherwise split on.
pub fn explorer_select_arg(path: &std::path::Path) -> String {
    format!("/select,\"{}\"", path.display())
}

/// A finished item whose file is no longer on disk.
pub fn file_missing(item: &Item) -> bool {
    item.status == Status::Done && item.dest.as_ref().is_none_or(|p| !p.exists())
}

impl Model {
    /// Delete is two clicks: the first arms it for `id`, the second (same item) confirms.
    pub fn confirm_delete(&mut self, id: ItemId) -> bool {
        if self.pending_delete == Some(id) {
            self.pending_delete = None;
            true
        } else {
            self.pending_delete = Some(id);
            false
        }
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
    /// Last clipboard link seen (suggested once, never again).
    pub last_clipboard: Option<String>,
    /// The first clipboard read only records what's there, so an old link isn't suggested at launch.
    pub clipboard_primed: bool,
    /// Where the browser extension can reach us, or why it can't.
    pub bridge_status: String,
    /// Item whose Delete was clicked once and waits for confirmation.
    pub pending_delete: Option<ItemId>,
    pub queues: Vec<Queue>,
    /// The Queues screen's form.
    pub queue_drafts: Vec<QueueDraft>,
    /// "Update yt-dlp" is running.
    pub updating_ytdlp: bool,
    pub filter: Filter,
    pub library: Library,
    pub search: String,
    pub sidebar_open: bool,
    /// The speed-limit popover is open.
    pub speed_open: bool,
    /// The speed slider is being dragged: the value it shows (applied on release).
    pub speed_preview: Option<u64>,
    pub settings_tab: SettingsTab,
    /// A link seen on the clipboard, offered in a toast.
    pub toast: Option<String>,
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
            last_clipboard: None,
            clipboard_primed: false,
            bridge_status: String::new(),
            pending_delete: None,
            queues: AppState::default().queues,
            queue_drafts: Vec::new(),
            updating_ytdlp: false,
            filter: Filter::All,
            library: Library::All,
            search: String::new(),
            sidebar_open: true,
            speed_open: false,
            speed_preview: None,
            settings_tab: SettingsTab::General,
            toast: None,
        }
    }
}

impl Model {
    /// Replaces everything with a full snapshot from the manager.
    pub fn load(&mut self, state: AppState) {
        self.items = state.items;
        self.queues = state.queues;
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
            Event::PickMedia { url, info, choice } => {
                self.notice = None;
                self.picker = Some(Picker::new(url, info, choice, 0));
                self.screen = Screen::Picker;
            }
            Event::Notice(text) => self.notice = Some(text),
            Event::Focus => {}
            Event::Queues(queues) => self.queues = queues,
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

    /// Queued, but its queue's schedule doesn't allow it to run right now.
    pub fn waiting_for_schedule(&self, item: &Item) -> bool {
        let schedule = self.queues.iter().find(|q| q.id == item.queue).and_then(|q| q.schedule.as_ref());
        item.status == Status::Queued && schedule.is_some_and(|s| !s.is_active(rdm_core::Now::local()))
    }

    /// The slider is being dragged: show `v`, apply nothing yet.
    pub fn preview_speed(&mut self, v: f32) {
        self.speed_preview = Some(crate::view::slider_to_bps(v));
    }

    /// The slider was let go: the limit to apply, if it was dragged.
    pub fn release_speed(&mut self) -> Option<u64> {
        self.speed_preview.take()
    }

    /// The limit the popover shows (bytes/s, 0 = none).
    pub fn shown_limit(&self) -> u64 {
        self.speed_preview.unwrap_or(self.settings.speed_limit_bps)
    }

    /// The accent to draw with: the typed one while Settings is open and it's valid, else the saved one.
    pub fn accent_hex(&self) -> &str {
        let draft = self.draft.accent.trim();
        if self.screen == Screen::Settings && crate::ui::theme::parse_hex(draft).is_some() { draft } else { &self.settings.accent }
    }

    /// A clipboard read: a new link becomes a toast (the first read only records what's there).
    pub fn clipboard_seen(&mut self, text: Option<String>) {
        if let Some(link) = text.and_then(|t| clipboard_link(&t, self.last_clipboard.as_deref())) {
            self.last_clipboard = Some(link.clone());
            if self.clipboard_primed {
                self.toast = Some(link);
            }
        }
        self.clipboard_primed = true;
    }

    pub fn open_settings(&mut self, tab: SettingsTab) {
        self.draft = Draft::from_settings(&self.settings);
        self.queue_drafts = self.queues.iter().map(QueueDraft::from_queue).collect();
        self.settings_tab = tab;
        self.notice = None;
        self.speed_open = false;
        self.screen = Screen::Settings;
    }

    /// Closing the sheet saves it: the settings and queues to send, or it stays open with the error.
    pub fn close_settings(&mut self) -> Result<(Settings, Vec<Queue>), String> {
        let result = self.draft.to_settings(&self.settings).and_then(|s| Ok((s, drafts_to_queues(&self.queue_drafts)?)));
        match &result {
            Ok(_) => {
                self.notice = None;
                self.screen = Screen::Downloads;
            }
            Err(e) => self.notice = Some(e.clone()),
        }
        result
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
            referrer: None,
            work_dir: None,
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
    fn queues_event_updates_model() {
        let mut m = Model::default();
        assert_eq!(m.queues.len(), 1, "Main is there from the start");
        let night = Queue { id: 1, name: "Night".into(), max_concurrent: 1, schedule: None };
        m.apply(Event::Queues(vec![m.queues[0].clone(), night.clone()]));
        assert_eq!(m.queues[1], night);
        let mut state = AppState::default();
        state.queues.push(night);
        let mut fresh = Model::default();
        fresh.load(state.clone());
        assert_eq!(fresh.queues, state.queues, "a snapshot brings the queues too");
    }

    #[test]
    fn waiting_for_schedule_only_when_queue_is_off_now() {
        let mut m = Model::default();
        let never = rdm_core::Schedule { start: 0, stop: None, days: [false; 7] };
        m.queues.push(Queue { id: 1, name: "Never".into(), max_concurrent: 1, schedule: Some(never) });
        let mut queued = item(1, Status::Queued, 0);
        assert!(!m.waiting_for_schedule(&queued), "Main has no schedule");
        queued.queue = 1;
        assert!(m.waiting_for_schedule(&queued));
        assert!(!m.waiting_for_schedule(&Item { queue: 1, ..item(2, Status::Paused, 0) }), "paused isn't waiting");
    }

    #[test]
    fn draft_keeps_quality_preference() {
        let s = Settings { ask_quality: false, preferred_quality: Some(MediaFormat::Video { max_height: 720 }), ..Settings::default() };
        let d = Draft::from_settings(&s);
        assert_eq!(d.to_settings(&Settings::default()).unwrap(), s);
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
        let p = Picker::new("https://youtu.be/x".into(), info(vec![]), 1, 0);
        assert_eq!(p.requests(), vec![("https://youtu.be/x".to_string(), "Clip".to_string(), MediaFormat::AudioMp3)]);
    }

    #[test]
    fn picker_playlist_makes_one_request_per_entry() {
        let entries = vec![Entry { url: "https://y/1".into(), title: "One".into() }, Entry { url: "https://y/2".into(), title: "Two".into() }];
        let p = Picker::new("https://y/list".into(), info(entries), 0, 0);
        let reqs = p.requests();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[1], ("https://y/2".to_string(), "Two".to_string(), MediaFormat::Video { max_height: 720 }));
    }

    #[test]
    fn pick_media_event_opens_picker() {
        let mut m = Model::default();
        m.apply(Event::PickMedia { url: "https://youtu.be/x".into(), info: info(vec![]), choice: 0 });
        assert_eq!(m.screen, Screen::Picker);
        let p = m.picker.expect("picker open");
        assert_eq!(p.url, "https://youtu.be/x");
        assert_eq!(p.choice, 0);
    }

    #[test]
    fn explorer_select_quotes_the_path() {
        let p = std::path::Path::new(r"C:\dl\Videos\A, B 😳.mp4");
        assert_eq!(explorer_select_arg(p), r#"/select,"C:\dl\Videos\A, B 😳.mp4""#);
    }

    #[test]
    fn done_item_without_its_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let present = dir.path().join("here.bin");
        std::fs::write(&present, b"x").unwrap();
        let mut done = item(1, Status::Done, 0);
        done.dest = Some(present);
        assert!(!file_missing(&done));
        done.dest = Some(dir.path().join("gone.bin"));
        assert!(file_missing(&done));
        done.dest = None;
        assert!(file_missing(&done));
        let mut running = item(2, Status::Running, 0);
        running.dest = Some(dir.path().join("gone.bin"));
        assert!(!file_missing(&running), "only finished items can be missing");
    }

    #[test]
    fn delete_needs_a_second_click_on_the_same_item() {
        let mut m = Model::default();
        assert!(!m.confirm_delete(ItemId(1)), "first click only arms");
        assert_eq!(m.pending_delete, Some(ItemId(1)));
        assert!(!m.confirm_delete(ItemId(2)), "another item re-arms instead of deleting");
        assert!(m.confirm_delete(ItemId(2)), "second click on the same item deletes");
        assert_eq!(m.pending_delete, None);
    }

    #[test]
    fn notice_event_is_shown() {
        let mut m = Model::default();
        m.apply(Event::Notice("Couldn't read that link".into()));
        assert_eq!(m.notice.as_deref(), Some("Couldn't read that link"));
        m.apply(Event::Focus);
        assert_eq!(m.notice.as_deref(), Some("Couldn't read that link"), "focus doesn't change the view");
    }

    #[test]
    fn clipboard_accepts_new_links_only() {
        assert_eq!(clipboard_link("  https://x.com/a.zip \n", None).as_deref(), Some("https://x.com/a.zip"));
        assert_eq!(clipboard_link("https://x.com/a.zip", Some("https://x.com/a.zip")), None);
        assert_eq!(clipboard_link("hello world", None), None);
        assert_eq!(clipboard_link("http://a b", None), None);
        assert_eq!(clipboard_link("https://a\nhttps://b", None), None);
        assert_eq!(clipboard_link("ftp://x/y", None), None);
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
