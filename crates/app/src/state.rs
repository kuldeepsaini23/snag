use crate::hsv::Hsv;
use crate::queues::{QueueDraft, drafts_to_queues};
use crate::view::{Filter, Library};
use rdm_core::{AppState, Event, Item, ItemId, MediaFormat, MediaInfo, Queue, QueueId, Settings, Status, ThemeMode};
use rdm_media::QualityOption;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Which box the Windows folder picker fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderFor {
    /// The download folder (Settings → General, and the tour).
    Downloads,
    /// A "Move to folder" rule being written.
    Rule,
}

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

/// (url, title, format) of one video to add.
pub type Request = (String, String, MediaFormat);
/// (thumbnail, duration) of that video.
pub type Meta = (Option<String>, Option<f64>);

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

    /// `requests` with each video's (thumbnail, duration).
    pub fn requests_with_meta(&self) -> Vec<(Request, Meta)> {
        let all: Vec<_> = self.info.requests(&self.url, self.choice).into_iter().zip(self.info.request_meta()).collect();
        if self.info.entries.is_empty() {
            return all;
        }
        all.into_iter().zip(&self.selected).filter(|(_, on)| **on).map(|(r, _)| r).collect()
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

/// A Windows notification to show.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

/// More than this many at once become a single summary notification.
const NOTE_BURST: usize = 3;
/// "Finished" notes wait while more downloads are under way (a playlist is one toast), but
/// never longer than this.
const NOTE_MAX_WAIT: std::time::Duration = std::time::Duration::from_secs(120);

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
    pub notify: bool,
    pub check_updates: bool,
    pub subtitles: bool,
    pub subtitle_langs: String,
    pub auto_retry: bool,
    pub keep_sharing: bool,
    pub phone_sharing: bool,
    /// "#rrggbb" as typed (Custom colour).
    pub accent: String,
    pub virustotal_key: String,
    /// After-download rules (saved with the rest when Settings closes).
    pub rules: Vec<rdm_core::rules::Rule>,
    pub theme: ThemeMode,
    pub translucent: bool,
    pub use_gpu: bool,
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
            notify: s.notify,
            check_updates: s.check_updates,
            subtitles: s.subtitles,
            subtitle_langs: s.subtitle_langs.clone(),
            auto_retry: s.auto_retry,
            keep_sharing: s.keep_sharing,
            phone_sharing: s.phone_sharing,
            virustotal_key: s.virustotal_key.clone(),
            rules: s.rules.clone(),
            accent: s.accent.clone(),
            theme: s.theme,
            translucent: s.translucent,
            use_gpu: s.use_gpu,
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
            notify: self.notify,
            check_updates: self.check_updates,
            subtitles: self.subtitles,
            subtitle_langs: if self.subtitle_langs.trim().is_empty() { base.subtitle_langs.clone() } else { self.subtitle_langs.trim().to_string() },
            auto_retry: self.auto_retry,
            keep_sharing: self.keep_sharing,
            phone_sharing: self.phone_sharing,
            virustotal_key: self.virustotal_key.trim().to_string(),
            rules: self.rules.clone(),
            accent: accent.to_string(),
            theme: self.theme,
            translucent: self.translucent,
            use_gpu: self.use_gpu,
            ..base.clone()
        })
    }
}

/// A link worth suggesting from the clipboard: a single link that isn't the last one seen and
/// looks downloadable — a file, a video or image site, a torrent, a GitHub repository. Ordinary
/// web pages (copied all day long) don't pop up.
pub fn clipboard_link(text: &str, last_seen: Option<&str>) -> Option<String> {
    use rdm_core::route::{Route, github_zip, route};
    let link = text.trim();
    let web = link.starts_with("http://") || link.starts_with("https://") || rdm_core::is_torrent_link(link);
    let is_url = web && link.len() < 4096 && !link.contains(char::is_whitespace);
    // `route` sends any page to the video reader; only known video sites count here.
    let downloadable = route(link) != Route::Media || rdm_media::is_media_url(link) || github_zip(link).is_some();
    (is_url && downloadable && last_seen != Some(link)).then(|| link.to_string())
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
    /// The folder picker closed: its folder goes into the box it was opened for (Cancel: nothing).
    pub fn folder_picked(&mut self, target: FolderFor, folder: Option<std::path::PathBuf>) {
        let Some(folder) = folder else { return };
        let text = folder.display().to_string();
        match target {
            FolderFor::Downloads => self.draft.download_dir = text,
            FolderFor::Rule => self.rule_form.folder = text,
        }
    }

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

    /// Cancel is two clicks too, and only for unfinished items (it throws away what's downloaded).
    pub fn confirm_cancel(&mut self, id: ItemId) -> bool {
        if !self.items.iter().any(|i| i.id == id && crate::view::can_cancel(i)) {
            self.pending_cancel = None;
            false
        } else if self.pending_cancel == Some(id) {
            self.pending_cancel = None;
            true
        } else {
            self.pending_cancel = Some(id);
            false
        }
    }

    /// A hex typed into the box or a swatch: the picker follows a valid one, unless it already
    /// shows that colour (so rounding never nudges its cursors).
    pub fn type_accent(&mut self, hex: String) {
        if let Some(rgb) = crate::hsv::parse(&hex)
            && rgb != self.accent_hsv.to_rgb()
        {
            self.accent_hsv = Hsv::from_rgb(rgb, self.accent_hsv);
        }
        self.draft.accent = hex;
    }

    /// The saturation/value square was clicked or dragged (both 0 … 1).
    pub fn drag_accent_sv(&mut self, s: f32, v: f32) {
        self.accent_hsv = Hsv::new(self.accent_hsv.h, s, v);
        self.draft.accent = self.accent_hsv.to_hex();
    }

    /// The hue strip was clicked or dragged (degrees).
    pub fn drag_accent_hue(&mut self, h: f32) {
        self.accent_hsv = Hsv::new(h, self.accent_hsv.s, self.accent_hsv.v);
        self.draft.accent = self.accent_hsv.to_hex();
    }

    /// Once per launch, after the first full load: an update shows "What's new" (once), and the
    /// version is noted. Returns the settings to save when the noted version changes.
    pub fn check_version(&mut self, current: &str) -> Option<Settings> {
        if std::mem::replace(&mut self.version_checked, true) {
            return None;
        }
        let seen = self.settings.last_seen_version.trim().to_string();
        match crate::changelog::on_launch(&seen, current, !self.items.is_empty()) {
            crate::changelog::Launch::Nothing => return None,
            crate::changelog::Launch::Show => self.open_info(Info::WhatsNew { since: Some(seen) }),
            crate::changelog::Launch::Record => {}
        }
        self.settings.last_seen_version = current.to_string();
        Some(self.settings.clone())
    }

    /// The "?" menu (a click anywhere else closes it, like the speed popover).
    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
    }

    pub fn open_info(&mut self, info: Info) {
        self.help_open = false;
        self.info = Some(info);
    }

    pub fn open_search(&mut self) {
        self.search_open = true;
    }

    /// Focus moved: an empty search folds back into its magnifier once it isn't focused.
    pub fn search_focus(&mut self, focused: bool) {
        if !focused && self.search.trim().is_empty() {
            self.search_open = false;
        }
    }

    /// Escape on the list: dismiss the toast, else deselect (the inspector slides away).
    pub fn escape_downloads(&mut self) {
        if self.toast.is_some() {
            self.toast = None;
        } else {
            self.selected = None;
            self.pending_cancel = None;
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
    /// The window fills the screen: no resize grips.
    pub maximized: bool,
    /// Quit was asked for while downloads run: the "still downloading" sheet is up.
    pub confirm_quit: bool,
    /// Notifications waiting to be shown (flushed in batches).
    pub notes: Vec<Note>,
    /// When the oldest waiting note arrived.
    pub notes_since: Option<std::time::Instant>,
    /// Thumbnail URL → the cached image file.
    pub thumbs: HashMap<String, PathBuf>,
    /// Thumbnails being downloaded.
    pub thumb_pending: HashSet<String>,
    /// Thumbnails that couldn't be fetched: (failures so far, when to try again).
    pub thumb_failures: HashMap<String, (u32, std::time::Instant)>,
    /// The last minute of speeds, per item and in total (the sparklines).
    pub speeds: crate::speed_history::Speeds,
    /// Each running segmented download's connections (the inspector's segment map).
    pub segments: HashMap<ItemId, Vec<rdm_engine::segments::Segment>>,
    /// Downloaded per day, from the manager (the stats screen).
    pub daily: std::collections::BTreeMap<String, rdm_core::DayTotal>,
    /// The stats screen shows instead of the list.
    pub stats_open: bool,
    pub stats_range: crate::stats::Range,
    /// Libraries shown as a grid of thumbnails (Videos, Images) instead of rows.
    pub grid: HashSet<rdm_core::Category>,
    /// "Refresh link": the item and the new link typed for it.
    pub refresh: Option<(ItemId, String)>,
    /// Why the settings sheet couldn't be saved (shown in it with a warning icon).
    pub settings_error: Option<String>,
    /// Watched channels and playlists.
    pub watches: Vec<rdm_core::watch::Watch>,
    /// A browser extension (id of the request, its origin) waits for "Allow / Don't allow".
    pub pair_request: Option<(u64, String)>,
    /// "Already downloaded": the finished download a new link matched (nothing was added).
    pub duplicate: Option<Item>,
    /// A newer Snag to offer (and whether it is being fetched).
    pub update: Option<UpdateOffer>,
    /// The version put off with "Later" (this session).
    pub update_later: Option<String>,
    /// The mouse, in window coordinates (for the right-click menu).
    pub cursor: iced::Point,
    /// A download's right-click menu is open at this point.
    pub row_menu: Option<(ItemId, iced::Point)>,
    /// The window's size (keeps the menu on screen).
    pub window: iced::Size,
    /// VirusTotal's verdicts on downloaded programs.
    pub safety: std::collections::BTreeMap<ItemId, rdm_core::safety::Safety>,
    /// The new after-download rule being filled in (Settings → General).
    pub rule_form: crate::rules_form::RuleForm,
    /// Why the new rule can't be added yet.
    pub rule_error: Option<&'static str>,
    /// The toolbar search is a field (else just its magnifier).
    pub search_open: bool,
    /// Item whose Cancel was clicked once and waits for confirmation.
    pub pending_cancel: Option<ItemId>,
    /// The row under the mouse (its buttons show).
    pub hovered: Option<ItemId>,
    /// The sidebar's "More" group (sources, watched channels) is expanded.
    pub more_open: bool,
    /// The filter tabs' measured widths, for the sliding underline.
    pub tab_widths: Vec<f32>,
    /// Where the accent picker's cursors are (set from the accent when Settings opens; it
    /// remembers the hue of a grey, see `hsv.rs`).
    pub accent_hsv: Hsv,
    /// The toolbar's "?" menu is open.
    pub help_open: bool,
    /// A sheet from the "?" menu is up (above any other sheet).
    pub info: Option<Info>,
    /// Report a bug: add versions, settings and the log's end.
    pub bug_diagnostics: bool,
    /// The bug report is being written.
    pub bug_saving: bool,
    /// "What's new" was looked at for this launch (once, after the first full load).
    pub version_checked: bool,
    /// Windows' app mode is light (read from the registry; followed when the theme is "Follow Windows").
    pub system_light: bool,
    /// Mica (or acrylic) is behind the window: the translucent tokens can be drawn.
    pub backdrop: bool,
    /// The first-run tour's step, while it runs (see `tour.rs`).
    pub tour: Option<crate::tour::Step>,
    /// Whether to start the tour was decided for this launch.
    pub tour_checked: bool,
    /// Files are dragged over the window.
    pub drop_hover: bool,
}

/// The sheets of the "?" menu.
#[derive(Clone, Debug, PartialEq)]
pub enum Info {
    Shortcuts,
    Help,
    /// After an update, `since` is the version seen before ("" = a build older than that setting)
    /// and only newer releases are listed; from the menu it is None: all of them.
    WhatsNew { since: Option<String> },
    BugReport,
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
            maximized: false,
            confirm_quit: false,
            notes: Vec::new(),
            notes_since: None,
            thumbs: HashMap::new(),
            thumb_pending: HashSet::new(),
            thumb_failures: HashMap::new(),
            speeds: Default::default(),
            segments: HashMap::new(),
            daily: Default::default(),
            stats_open: false,
            stats_range: Default::default(),
            grid: HashSet::new(),
            refresh: None,
            settings_error: None,
            watches: Vec::new(),
            pair_request: None,
            duplicate: None,
            update: None,
            update_later: None,
            cursor: iced::Point::ORIGIN,
            row_menu: None,
            window: iced::Size::new(1280.0, 800.0),
            safety: Default::default(),
            rule_form: Default::default(),
            rule_error: None,
            search_open: false,
            pending_cancel: None,
            hovered: None,
            more_open: false,
            tab_widths: Vec::new(),
            accent_hsv: Hsv::new(0.0, 1.0, 1.0),
            help_open: false,
            info: None,
            bug_diagnostics: true,
            bug_saving: false,
            version_checked: false,
            system_light: false,
            backdrop: false,
            tour: None,
            tour_checked: false,
            drop_hover: false,
        }
    }
}

impl Model {
    /// Replaces everything with a full snapshot from the manager.
    pub fn load(&mut self, state: AppState) {
        self.items = state.items;
        self.safety = state.safety;
        self.queues = state.queues;
        self.watches = state.watches;
        self.daily = state.daily;
        let items = &self.items;
        self.segments.retain(|id, _| items.iter().any(|i| i.id == *id && i.status == Status::Running));
        // An open settings sheet keeps what's being typed; it is saved on close.
        if !self.editing() {
            self.draft = Draft::from_settings(&state.settings);
        }
        self.settings = state.settings;
        if self.selected.is_some_and(|id| !self.items.iter().any(|i| i.id == id)) {
            self.selected = None;
        }
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Added(item) => self.items.push(item),
            Event::Updated(item) => match self.items.iter_mut().find(|i| i.id == item.id) {
                Some(slot) => {
                    // Failures are told once they're final (not while an automatic retry is pending).
                    let was_final_failure = matches!(slot.status, Status::Failed(_)) && slot.retry_at.is_none();
                    let note = match (&slot.status, &item.status) {
                        (old, Status::Done) if *old != Status::Done => Some(Note { title: "Download finished".into(), body: item.name.clone() }),
                        (_, Status::Failed(e)) if item.retry_at.is_none() && !was_final_failure => {
                            Some(Note { title: "Download failed".into(), body: format!("{}: {e}", item.name) })
                        }
                        _ => None,
                    };
                    if let Some(n) = note.filter(|_| self.settings.notify) {
                        self.notes.push(n);
                        self.notes_since.get_or_insert_with(std::time::Instant::now);
                    }
                    if item.status != Status::Running {
                        self.segments.remove(&item.id);
                    }
                    *slot = item;
                }
                None => self.items.push(item),
            },
            Event::Removed(id) => {
                self.items.retain(|i| i.id != id);
                self.safety.remove(&id);
                self.segments.remove(&id);
                if self.selected == Some(id) {
                    self.selected = None;
                }
            }
            Event::PickMedia { url, info, choice } => {
                self.notice = None;
                self.picker = Some(Picker::new(url, info, choice, 0));
                // An open settings sheet keeps its unsaved edits; the picker shows once it closes.
                if self.screen != Screen::Settings {
                    self.screen = Screen::Picker;
                }
            }
            Event::Notice(text) => self.notice = Some(text),
            Event::Focus | Event::Quit => {}
            Event::Watches(watches) => self.watches = watches,
            Event::PairRequest { id, origin } => self.pair_request = Some((id, origin)),
            Event::Duplicate(item) => self.duplicate = Some(item),
            Event::Safety(id, verdict) => {
                self.safety.insert(id, verdict);
            }
            Event::Segments(id, segments) => {
                if self.items.iter().any(|i| i.id == id && i.status == Status::Running) {
                    self.segments.insert(id, segments);
                }
            }
            Event::Queues(queues) => {
                if let crate::view::Library::Queue(id) = self.library
                    && !queues.iter().any(|q| q.id == id)
                {
                    self.library = crate::view::Library::All;
                }
                self.queues = queues;
            }
            Event::Settings(s) => {
                if !self.editing() {
                    self.draft = Draft::from_settings(&s);
                }
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

    /// Closes the popover: a change made without a release (arrow keys, wheel) is applied now.
    pub fn close_speed(&mut self) -> Option<u64> {
        self.speed_open = false;
        self.speed_preview.take()
    }

    /// The limit the popover shows (bytes/s, 0 = none).
    pub fn shown_limit(&self) -> u64 {
        self.speed_preview.unwrap_or(self.settings.speed_limit_bps)
    }

    /// The settings sheet or the tour's sheet is up: the draft is being edited (and previewed).
    pub fn editing(&self) -> bool {
        self.screen == Screen::Settings || self.tour_sheet()
    }

    /// The accent to draw with: the typed one while Settings is open and it's valid, else the saved one.
    pub fn accent_hex(&self) -> &str {
        let draft = self.draft.accent.trim();
        if self.editing() && crate::ui::theme::parse_hex(draft).is_some() { draft } else { &self.settings.accent }
    }

    /// The theme to draw with: the picked one while Settings is open (a preview), else the saved one.
    pub fn theme_mode(&self) -> ThemeMode {
        if self.editing() { self.draft.theme } else { self.settings.theme }
    }

    /// Draw the light token set.
    pub fn light(&self) -> bool {
        match self.theme_mode() {
            ThemeMode::Dark => false,
            ThemeMode::Light => true,
            ThemeMode::System => self.system_light,
        }
    }

    /// Draw see-through panels: switched on, and Mica made it behind the window.
    pub fn translucent(&self) -> bool {
        self.settings.translucent && self.backdrop
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
        self.type_accent(self.settings.accent.clone());
        self.queue_drafts = self.queues.iter().map(QueueDraft::from_queue).collect();
        self.settings_tab = tab;
        self.notice = None;
        self.settings_error = None;
        self.speed_open = false;
        self.screen = Screen::Settings;
    }

    /// Downloads under way right now (running, or queued and allowed to run).
    fn downloading_now(&self) -> bool {
        self.items.iter().any(|i| i.status == Status::Running || (i.status == Status::Queued && !self.waiting_for_schedule(i)))
    }

    #[cfg(test)]
    pub fn take_notes(&mut self) -> Vec<Note> {
        self.take_notes_at(std::time::Instant::now())
    }

    /// The notifications to show at `now`. Failures go out at once; "finished" ones wait while
    /// more downloads are under way (up to `NOTE_MAX_WAIT`), and a burst becomes one summary.
    pub fn take_notes_at(&mut self, now: std::time::Instant) -> Vec<Note> {
        if self.notes.is_empty() {
            return Vec::new();
        }
        let failure = self.notes.iter().any(|n| n.title == "Download failed");
        let waited = self.notes_since.is_some_and(|t| now.saturating_duration_since(t) >= NOTE_MAX_WAIT);
        if !failure && !waited && self.downloading_now() {
            return Vec::new();
        }
        self.notes_since = None;
        let notes = std::mem::take(&mut self.notes);
        if notes.len() <= NOTE_BURST {
            return notes;
        }
        let failed = notes.iter().filter(|n| n.title == "Download failed").count();
        let done = notes.len() - failed;
        let title = match (done, failed) {
            (d, 0) => format!("{d} downloads finished"),
            (0, f) => format!("{f} downloads failed"),
            (d, f) => format!("{d} downloads finished, {f} failed"),
        };
        let names: Vec<&str> = notes.iter().take(NOTE_BURST).map(|n| n.body.as_str()).collect();
        vec![Note { title, body: format!("{}…", names.join(", ")) }]
    }

    /// Downloads that quitting would interrupt (running or waiting their turn).
    pub fn busy_count(&self) -> usize {
        self.items.iter().filter(|i| matches!(i.status, Status::Running | Status::Queued)).count()
    }

    /// Quit from the tray: true = quit now; false = ask first (the confirm sheet is shown).
    pub fn request_quit(&mut self) -> bool {
        self.confirm_quit = self.busy_count() > 0;
        !self.confirm_quit
    }

    pub fn keep_downloading(&mut self) {
        self.confirm_quit = false;
    }

    /// Running, queued, and failed ones waiting for an automatic retry.
    pub fn pause_all_ids(&self) -> Vec<ItemId> {
        self.items.iter().filter(|i| matches!(i.status, Status::Running | Status::Queued) || i.retry_at.is_some()).map(|i| i.id).collect()
    }

    /// Paused and failed ones (not those already about to retry by themselves).
    pub fn resume_all_ids(&self) -> Vec<ItemId> {
        self.items.iter().filter(|i| matches!(i.status, Status::Paused | Status::Failed(_)) && i.retry_at.is_none()).map(|i| i.id).collect()
    }

    /// Another tab of the open sheet; edits on every tab are kept until the sheet closes.
    pub fn switch_settings_tab(&mut self, tab: SettingsTab) {
        self.settings_tab = tab;
    }

    /// Closing the sheet saves it: the settings and queues to send, or it stays open with the error.
    pub fn close_settings(&mut self) -> Result<(Settings, Vec<Queue>), String> {
        let result = self.draft.to_settings(&self.settings).and_then(|s| Ok((s, drafts_to_queues(&self.queue_drafts)?)));
        match &result {
            Ok((settings, _)) => {
                self.settings_error = None;
                // Applied at once (the manager's echo follows), so the accent doesn't flash back.
                self.settings = settings.clone();
                self.screen = if self.picker.is_some() { Screen::Picker } else { Screen::Downloads };
            }
            Err(e) => self.settings_error = Some(e.clone()),
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

    #[test]
    fn a_picked_folder_fills_the_right_box() {
        let mut m = Model::default();
        m.draft.download_dir = r"C:\old".into();
        m.folder_picked(FolderFor::Downloads, Some(std::path::PathBuf::from(r"D:\Media")));
        assert_eq!(m.draft.download_dir, r"D:\Media");
        m.folder_picked(FolderFor::Downloads, None);
        assert_eq!(m.draft.download_dir, r"D:\Media", "Cancel keeps what was there");
        m.folder_picked(FolderFor::Rule, Some(std::path::PathBuf::from(r"E:\Programs")));
        assert_eq!(m.rule_form.folder, r"E:\Programs");
        assert_eq!(m.draft.download_dir, r"D:\Media");
    }

    #[test]
    fn a_duplicate_link_shows_its_earlier_download() {
        let mut m = Model::default();
        m.apply(Event::Duplicate(item(7, Status::Done, 0)));
        assert_eq!(m.duplicate.as_ref().map(|i| i.id), Some(ItemId(7)));
    }

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
            thumbnail: None,
            duration: None,
            retry_at: None,
        }
    }

    #[test]
    fn segments_are_kept_only_while_running() {
        use rdm_engine::segments::Segment;
        let mut m = Model::default();
        m.apply(Event::Added(item(1, Status::Running, 0)));
        m.apply(Event::Added(item(2, Status::Paused, 0)));
        m.apply(Event::Segments(ItemId(1), vec![Segment::new(0, 10), Segment::new(10, 20)]));
        m.apply(Event::Segments(ItemId(2), vec![Segment::new(0, 10)]));
        assert_eq!(m.segments.get(&ItemId(1)).map(Vec::len), Some(2));
        assert!(!m.segments.contains_key(&ItemId(2)), "a late report for a stopped download is dropped");
        m.apply(Event::Updated(item(1, Status::Done, 0)));
        assert!(m.segments.is_empty(), "finished: no map");
        m.apply(Event::Updated(item(1, Status::Running, 0)));
        m.apply(Event::Segments(ItemId(1), vec![Segment::new(0, 20)]));
        m.apply(Event::Removed(ItemId(1)));
        assert!(m.segments.is_empty(), "removed: forgotten");
    }

    #[test]
    fn a_load_brings_the_day_counter() {
        let mut m = Model::default();
        let daily = [("2026-10-07".to_string(), rdm_core::DayTotal { bytes: 5, ms: 9 })].into();
        m.load(AppState { daily, ..AppState::default() });
        assert_eq!(m.daily["2026-10-07"].bytes, 5);
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
            thumbnail: None,
            live: false,
        }
    }

    #[test]
    fn picker_single_video_makes_one_request() {
        let p = Picker::new("https://youtu.be/x".into(), info(vec![]), 1, 0);
        assert_eq!(p.requests(), vec![("https://youtu.be/x".to_string(), "Clip".to_string(), MediaFormat::AudioMp3)]);
    }

    #[test]
    fn picker_playlist_makes_one_request_per_entry() {
        let entry = |url: &str, title: &str| Entry { url: url.into(), title: title.into(), thumbnail: None, duration: None };
        let entries = vec![entry("https://y/1", "One"), entry("https://y/2", "Two")];
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
        let magnet = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x";
        assert_eq!(clipboard_link(magnet, None).as_deref(), Some(magnet), "magnet links too");
    }

    #[test]
    fn clipboard_offers_only_what_looks_downloadable() {
        // Ordinary pages people copy all day: no popup (it once saved a login page as a file).
        for page in ["https://main-preprod.codehelp.in/problems/remove-all-adjacent-duplicates-in-string", "https://docs.rs/iced/latest/iced/", "https://example.com/"] {
            assert_eq!(clipboard_link(page, None), None, "{page}");
        }
        for link in [
            "https://cdn.example.com/files/setup.exe",
            "https://www.youtube.com/watch?v=abc",
            "https://x.com/someone/status/123",
            "https://github.com/rust-lang/rustlings",
            "https://www.pinterest.com/pin/1/",
        ] {
            assert_eq!(clipboard_link(link, None).as_deref(), Some(link), "{link}");
        }
    }

    #[test]
    fn search_collapses_when_empty() {
        let mut m = Model::default();
        assert!(!m.search_open, "a magnifier at first");
        m.open_search();
        assert!(m.search_open);
        m.search_focus(true);
        assert!(m.search_open, "still typing");
        m.search_focus(false);
        assert!(!m.search_open, "empty and focus left: back to the magnifier");
        m.open_search();
        m.search = "ubuntu".into();
        m.search_focus(false);
        assert!(m.search_open, "a search that filters stays visible");
        m.search = "  ".into();
        m.search_focus(false);
        assert!(!m.search_open, "blank counts as empty");
    }

    #[test]
    fn cancel_needs_a_second_click_on_unfinished_items() {
        let mut m = Model::default();
        m.apply(Event::Added(item(1, Status::Paused, 0)));
        m.apply(Event::Added(item(2, Status::Done, 0)));
        assert!(!m.confirm_cancel(ItemId(1)), "first click only arms");
        assert_eq!(m.pending_cancel, Some(ItemId(1)));
        assert!(m.confirm_cancel(ItemId(1)), "second click cancels");
        assert_eq!(m.pending_cancel, None);
        assert!(!m.confirm_cancel(ItemId(2)) && !m.confirm_cancel(ItemId(2)), "a finished item is never cancelled");
        assert_eq!(m.pending_cancel, None);
    }

    #[test]
    fn escape_deselects() {
        let mut m = Model { selected: Some(ItemId(3)), toast: Some("https://x/a.zip".into()), ..Model::default() };
        m.escape_downloads();
        assert_eq!(m.toast, None, "the toast goes first");
        assert_eq!(m.selected, Some(ItemId(3)));
        m.escape_downloads();
        assert_eq!(m.selected, None);
    }

    #[test]
    fn hex_and_picker_agree() {
        let mut m = Model::default();
        m.open_settings(SettingsTab::Appearance);
        assert_eq!(m.accent_hsv.to_hex(), m.settings.accent, "the picker opens on the saved accent");
        // Dragging writes the hex box, and the preview follows.
        m.drag_accent_hue(211.0);
        m.drag_accent_sv(0.96, 1.0);
        assert_eq!(m.draft.accent, m.accent_hsv.to_hex());
        assert_eq!(m.accent_hex(), m.draft.accent, "the window previews the dragged colour");
        // The box echoing what the picker shows doesn't move the picker (no rounding drift).
        let before = m.accent_hsv;
        m.type_accent(m.draft.accent.to_uppercase());
        assert_eq!(m.accent_hsv, before);
        // Half-typed: the picker waits; a whole hex moves it.
        m.type_accent("#32".into());
        assert_eq!(m.accent_hsv, before);
        m.type_accent("#32d74b".into());
        assert_eq!(m.accent_hsv.to_hex(), "#32d74b");
        m.type_accent("#fff".into());
        assert_eq!(m.accent_hsv.to_hex(), "#ffffff", "short hex too");
        // A grey typed in keeps the hue the strip shows; dragging back into colour starts there.
        m.drag_accent_hue(120.0);
        m.type_accent("#808080".into());
        assert_eq!(m.accent_hsv.h, 120.0);
        m.drag_accent_sv(1.0, 1.0);
        assert_eq!(m.draft.accent, "#00ff00");
        // Black keeps both hue and saturation, so the cursor stays where it was dragged.
        m.drag_accent_sv(0.5, 0.0);
        assert_eq!(m.draft.accent, "#000000");
        assert_eq!((m.accent_hsv.h, m.accent_hsv.s), (120.0, 0.5));
        let (saved, _) = m.close_settings().expect("valid");
        assert_eq!(saved.accent, "#000000", "closing saves what the picker shows");
    }

    #[test]
    fn light_follows_the_theme_setting() {
        let mut m = Model::default();
        assert!(!m.light(), "dark by default");
        m.settings.theme = ThemeMode::Light;
        assert!(m.light());
        m.settings.theme = ThemeMode::System;
        assert!(!m.light(), "Windows in dark mode");
        m.system_light = true;
        assert!(m.light(), "Windows in light mode");
        // The sheet previews what's picked, like the accent; closing saves it.
        m.open_settings(SettingsTab::Appearance);
        m.draft.theme = ThemeMode::Dark;
        assert!(!m.light());
        m.draft.translucent = true;
        let (saved, _) = m.close_settings().expect("valid");
        assert_eq!((saved.theme, saved.translucent), (ThemeMode::Dark, true));
        assert!(!m.light());
    }

    #[test]
    fn translucent_only_once_the_backdrop_is_on() {
        let mut m = Model::default();
        m.settings.translucent = true;
        assert!(!m.translucent(), "Mica isn't behind the window yet");
        m.backdrop = true;
        assert!(m.translucent());
        m.settings.translucent = false;
        assert!(!m.translucent(), "switched off: solid at once");
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

/// A newer Snag, offered in the corner card.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOffer {
    pub release: rdm_core::selfupdate::Release,
    /// Downloading and checking it.
    pub busy: bool,
}
