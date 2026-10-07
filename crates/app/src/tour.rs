//! The first-run tour: a 5-step sheet (welcome, folder, look, extension, clipboard), then
//! coach marks pointing at + Add, the speed button and "?". Pure state; `ui/tour.rs` draws it.

use crate::state::{Draft, Model, Screen};
use rdm_core::Settings;

/// Where the tour is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Welcome,
    Folder,
    /// Theme and accent colour.
    Look,
    Extension,
    Clipboard,
    /// A tooltip pointing at a toolbar button (the sheet is closed by then).
    Coach(Mark),
}

/// The toolbar buttons the coach marks point at, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Add,
    Speed,
    Help,
}

/// The sheet's steps, in order.
pub const SHEET: [Step; 5] = [Step::Welcome, Step::Folder, Step::Look, Step::Extension, Step::Clipboard];
pub const MARKS: [Mark; 3] = [Mark::Add, Mark::Speed, Mark::Help];

/// "Try a link": a small test file from OVH's public speed-test server.
pub const SAMPLE_LINK: &str = "https://proof.ovh.net/files/1Mb.dat";

impl Step {
    /// The steps in order: the sheet, then the coach marks (None after the last one).
    pub fn next(self) -> Option<Step> {
        match self {
            Step::Coach(mark) => MARKS.iter().position(|m| *m == mark).and_then(|i| MARKS.get(i + 1)).map(|m| Step::Coach(*m)),
            Step::Clipboard => Some(Step::Coach(MARKS[0])),
            _ => self.sheet_index().and_then(|i| SHEET.get(i + 1)).copied(),
        }
    }

    /// One step back, within the sheet or within the coach marks.
    pub fn back(self) -> Option<Step> {
        match self {
            Step::Coach(mark) => MARKS.iter().position(|m| *m == mark).and_then(|i| i.checked_sub(1)).map(|i| Step::Coach(MARKS[i])),
            _ => self.sheet_index().and_then(|i| i.checked_sub(1)).map(|i| SHEET[i]),
        }
    }

    /// Position on the sheet (None for coach marks).
    pub fn sheet_index(self) -> Option<usize> {
        SHEET.iter().position(|s| *s == self)
    }
}

/// Show the tour at launch: never finished or skipped, and nothing says Snag was used before
/// (no downloads, no version seen; an update from a build without the tour isn't a new user).
pub fn on_launch(tour_done: bool, has_history: bool, last_seen_version: &str) -> bool {
    !tour_done && !has_history && last_seen_version.trim().is_empty()
}

impl Model {
    /// The coach mark showing, if any.
    pub fn coach(&self) -> Option<Mark> {
        match self.tour {
            Some(Step::Coach(mark)) => Some(mark),
            _ => None,
        }
    }

    /// The tour's sheet is up (it edits the draft, like the settings sheet).
    pub fn tour_sheet(&self) -> bool {
        self.tour.is_some_and(|s| s.sheet_index().is_some())
    }

    /// Once per launch, after the first full load: a fresh install starts the tour; anyone else
    /// has it marked done (returned to save), so it never shows up later out of the blue.
    pub fn check_tour(&mut self) -> Option<Settings> {
        if std::mem::replace(&mut self.tour_checked, true) || self.settings.tour_done {
            return None;
        }
        if on_launch(self.settings.tour_done, !self.items.is_empty(), &self.settings.last_seen_version) {
            self.start_tour();
            return None;
        }
        self.settings.tour_done = true;
        Some(self.settings.clone())
    }

    /// From the start (also Settings → General → Show the tour again).
    pub fn start_tour(&mut self) {
        self.draft = Draft::from_settings(&self.settings);
        self.settings_error = None;
        self.screen = Screen::Downloads;
        (self.speed_open, self.help_open, self.info) = (false, false, None);
        self.tour = Some(Step::Welcome);
    }

    /// Next: a sheet step's choices are checked and saved (returned to send); after the last
    /// coach mark the tour is done.
    pub fn tour_next(&mut self) -> Option<Settings> {
        let step = self.tour?;
        if step.sheet_index().is_some() {
            let saved = match self.draft.to_settings(&self.settings) {
                Ok(s) => s,
                Err(e) => {
                    self.settings_error = Some(e);
                    return None;
                }
            };
            self.settings_error = None;
            self.settings = saved.clone();
            self.tour = step.next();
            return Some(saved);
        }
        self.tour = step.next();
        if self.tour.is_some() {
            return None;
        }
        self.settings.tour_done = true;
        Some(self.settings.clone())
    }

    pub fn tour_back(&mut self) {
        if let Some(back) = self.tour.and_then(Step::back) {
            self.settings_error = None;
            self.tour = Some(back);
        }
    }

    /// Skip, at any step: what was chosen so far is kept (if it's valid), and the tour is done.
    pub fn skip_tour(&mut self) -> Settings {
        if self.tour_sheet()
            && let Ok(s) = self.draft.to_settings(&self.settings)
        {
            self.settings = s;
        }
        self.tour = None;
        self.settings_error = None;
        self.settings.tour_done = true;
        self.draft = Draft::from_settings(&self.settings);
        self.settings.clone()
    }

    /// The last step's "Try a link": a test file goes into the link bar, then the coach marks.
    pub fn try_link(&mut self) -> Option<Settings> {
        let saved = self.tour_next()?;
        self.url = SAMPLE_LINK.to_string();
        Some(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SettingsTab;
    use rdm_core::{Event, ThemeMode};

    fn fresh() -> Model {
        let mut m = Model::default();
        assert!(m.check_tour().is_none(), "a fresh install has nothing to save yet");
        m
    }

    #[test]
    fn steps_in_order() {
        let mut seen = vec![Step::Welcome];
        while let Some(next) = seen.last().unwrap().next() {
            seen.push(next);
        }
        let marks = MARKS.map(Step::Coach);
        assert_eq!(seen, [SHEET.as_slice(), marks.as_slice()].concat());
        assert_eq!(Step::Welcome.back(), None);
        assert_eq!(Step::Look.back(), Some(Step::Folder));
        assert_eq!(Step::Coach(Mark::Add).back(), None, "the sheet is gone once the coach marks start");
        assert_eq!(Step::Coach(Mark::Help).back(), Some(Step::Coach(Mark::Speed)));
        assert_eq!(Step::Clipboard.sheet_index(), Some(4));
        assert_eq!(Step::Coach(Mark::Speed).sheet_index(), None);
    }

    #[test]
    fn shows_on_fresh_installs_only() {
        assert!(on_launch(false, false, ""));
        assert!(!on_launch(true, false, ""), "finished or skipped");
        assert!(!on_launch(false, true, ""), "has downloads: not new");
        assert!(!on_launch(false, false, "0.1.0"), "updated from a build without the tour");

        let m = fresh();
        assert_eq!(m.tour, Some(Step::Welcome));
        assert!(m.tour_sheet());

        let mut old = Model::default();
        old.apply(Event::Added(rdm_core::Item { ..sample_item() }));
        let saved = old.check_tour().expect("marked done");
        assert!(saved.tour_done && old.settings.tour_done);
        assert_eq!(old.tour, None);
        assert_eq!(old.check_tour(), None, "once per launch");
    }

    #[test]
    fn walks_through_and_finishes_once() {
        let mut m = fresh();
        for _ in 0..4 {
            let saved = m.tour_next().expect("each sheet step saves");
            assert!(!saved.tour_done);
        }
        assert_eq!(m.tour, Some(Step::Clipboard));
        m.tour_next();
        assert_eq!(m.tour, Some(Step::Coach(Mark::Add)));
        assert!(!m.tour_sheet() && m.coach() == Some(Mark::Add));
        m.tour_next();
        m.tour_next();
        assert_eq!(m.coach(), Some(Mark::Help));
        let done = m.tour_next().expect("finishing saves");
        assert!(done.tour_done && m.settings.tour_done);
        assert_eq!(m.tour, None);
        assert_eq!(m.tour_next(), None, "nothing left to do");
        // Next launch: loaded from what was saved, it doesn't come back.
        let mut next = Model::default();
        next.load(rdm_core::AppState { settings: done, ..Default::default() });
        assert_eq!(next.check_tour(), None);
        assert_eq!(next.tour, None);
    }

    #[test]
    fn skip_at_any_step() {
        for k in 0..8 {
            let mut m = fresh();
            for _ in 0..k {
                m.tour_next();
            }
            let saved = m.skip_tour();
            assert!(saved.tour_done && m.settings.tour_done, "step {k}");
            assert_eq!(m.tour, None, "step {k}");
            assert_eq!(m.screen, Screen::Downloads);
        }
    }

    #[test]
    fn choices_are_saved() {
        let mut m = fresh();
        m.tour_next();
        assert_eq!(m.tour, Some(Step::Folder));
        m.draft.download_dir = r"D:\Downloads".into();
        m.draft.sort_into_folders = false;
        let saved = m.tour_next().unwrap();
        assert_eq!((saved.download_dir.as_os_str().to_str(), saved.sort_into_folders), (Some(r"D:\Downloads"), false));
        // The look previews as it's picked, and sticks.
        m.draft.accent = "#0a84ff".into();
        m.draft.theme = ThemeMode::Light;
        assert_eq!(m.accent_hex(), "#0a84ff");
        assert!(m.light());
        // A settings echo from the manager doesn't wipe what's being picked.
        m.apply(Event::Settings(m.settings.clone()));
        assert_eq!(m.draft.accent, "#0a84ff");
        let saved = m.tour_next().unwrap();
        assert_eq!((saved.accent.as_str(), saved.theme), ("#0a84ff", ThemeMode::Light));
        m.tour_next();
        m.draft.clipboard_watch = false;
        let saved = m.tour_next().unwrap();
        assert!(!saved.clipboard_watch);
        assert_eq!(m.settings.accent, "#0a84ff");
        assert!(m.light(), "still light after the sheet closed");
        // Skipping keeps the choices on the current step too.
        let mut s = fresh();
        s.tour_next();
        s.tour_next();
        s.draft.accent = "#bf5af2".into();
        assert_eq!(s.skip_tour().accent, "#bf5af2");
    }

    #[test]
    fn a_bad_folder_stays_on_its_step() {
        let mut m = fresh();
        m.tour_next();
        m.draft.download_dir = "   ".into();
        assert_eq!(m.tour_next(), None);
        assert_eq!(m.tour, Some(Step::Folder));
        assert!(m.settings_error.is_some());
        m.draft.download_dir = r"C:\dl".into();
        assert!(m.tour_next().is_some());
        assert_eq!(m.settings_error, None);
        // Skipping with a bad folder keeps the saved one.
        let before = m.settings.download_dir.clone();
        m.tour_back();
        assert_eq!(m.tour, Some(Step::Folder));
        m.draft.download_dir = String::new();
        assert_eq!(m.skip_tour().download_dir, before);
    }

    #[test]
    fn try_a_link_fills_the_bar() {
        let mut m = fresh();
        for _ in 0..4 {
            m.tour_next();
        }
        let saved = m.try_link().expect("the last step saves");
        assert!(!saved.tour_done, "the coach marks are still to come");
        assert_eq!(m.url, SAMPLE_LINK);
        assert_eq!(m.coach(), Some(Mark::Add));
    }

    #[test]
    fn show_again_from_settings() {
        let mut m = fresh();
        m.skip_tour();
        m.open_settings(SettingsTab::General);
        m.start_tour();
        assert_eq!(m.tour, Some(Step::Welcome));
        assert_eq!(m.screen, Screen::Downloads, "the settings sheet makes way");
    }

    fn sample_item() -> rdm_core::Item {
        rdm_core::Item {
            id: rdm_core::ItemId(1),
            url: "https://x/a.zip".into(),
            name: "a.zip".into(),
            category: rdm_core::Category::Archive,
            status: rdm_core::Status::Done,
            dest: None,
            downloaded: 0,
            total: None,
            speed_bps: 0,
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
}
