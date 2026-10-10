//! "What's new": `CHANGELOG.md` (embedded) read into releases, and whether to show it at launch.

/// The changelog as shipped with this build.
pub const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");
/// This build's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One `## 0.2.0 — 2026-10-07` block.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub date: Option<String>,
    pub sections: Vec<Section>,
}

/// One `### New` block: its bullet points, each joined into one line.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub title: String,
    pub items: Vec<String>,
}

/// The releases in `text`, in the order written (newest first). `##` starts a release, `###` a
/// section, `- ` an item; indented lines continue the item above. Anything else is ignored.
pub fn parse(text: &str) -> Vec<Release> {
    let mut releases: Vec<Release> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(head) = trimmed.strip_prefix("### ") {
            if let Some(r) = releases.last_mut() {
                r.sections.push(Section { title: head.trim().to_string(), items: Vec::new() });
            }
        } else if let Some(head) = trimmed.strip_prefix("## ") {
            let (version, date) = match head.split_once(['—', '–']).or_else(|| head.split_once(" - ")) {
                Some((v, d)) => (v.trim(), Some(d.trim().to_string()).filter(|d| !d.is_empty())),
                None => (head.trim(), None),
            };
            releases.push(Release { version: version.to_string(), date, sections: Vec::new() });
        } else if let Some(item) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")).filter(|_| !line.starts_with(char::is_whitespace)) {
            let Some(r) = releases.last_mut() else { continue };
            if r.sections.is_empty() {
                r.sections.push(Section { title: String::new(), items: Vec::new() });
            }
            if let Some(s) = r.sections.last_mut() {
                s.items.push(item.trim().to_string());
            }
        } else if line.starts_with(char::is_whitespace) && !trimmed.is_empty() {
            // Continues the item above.
            if let Some(last) = releases.last_mut().and_then(|r| r.sections.last_mut()).and_then(|s| s.items.last_mut()) {
                last.push(' ');
                last.push_str(trimmed);
            }
        }
    }
    releases
}

/// The shipped changelog, read once.
pub fn releases() -> &'static [Release] {
    static RELEASES: std::sync::OnceLock<Vec<Release>> = std::sync::OnceLock::new();
    RELEASES.get_or_init(|| parse(CHANGELOG))
}

/// The releases newer than `since` (None: all of them), and at least the newest one.
pub fn releases_since(all: &[Release], since: Option<&str>) -> Vec<Release> {
    let newer: Vec<Release> = all.iter().filter(|r| since.is_none_or(|s| newer(&r.version, s))).cloned().collect();
    if newer.is_empty() { all.iter().take(1).cloned().collect() } else { newer }
}

/// What to do about "What's new" at launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launch {
    /// Already seen this version.
    Nothing,
    /// Note this version without showing anything (a fresh install, or an older build).
    Record,
    /// Updated since the last run: show the sheet once, and note the version.
    Show,
}

/// `last_seen`: the version the user last saw ("" before this setting existed); `has_history`:
/// there are downloads from earlier runs (so an empty `last_seen` is an update, not an install).
pub fn on_launch(last_seen: &str, current: &str, has_history: bool) -> Launch {
    if last_seen.trim().is_empty() && !has_history {
        Launch::Record
    } else if newer(current, last_seen) {
        Launch::Show
    } else {
        Launch::Nothing
    }
}

/// "0.10.2" > "0.9.9": compared number by number (anything that isn't a number counts as 0).
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.trim().split('.').map(|p| p.trim().parse().unwrap_or(0)).collect() };
    let (a, b) = (parts(a), parts(b));
    (0..a.len().max(b.len())).map(|i| (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0))).find(|(x, y)| x != y).is_some_and(|(x, y)| x > y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# What's new in Snag

Intro text that isn't part of any release.

## 0.2.0 — 2026-10-07

### New
- Pick any accent colour.
- A Help menu with a bug
  report that stays on your PC.

### Fixed
- Thumbnails come back.

## 0.1.0
- Everything else.
";

    #[test]
    fn changelog_parse() {
        let releases = parse(SAMPLE);
        assert_eq!(releases.len(), 2);
        let r = &releases[0];
        assert_eq!((r.version.as_str(), r.date.as_deref()), ("0.2.0", Some("2026-10-07")));
        assert_eq!(r.sections.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), vec!["New", "Fixed"]);
        assert_eq!(r.sections[0].items, vec!["Pick any accent colour.", "A Help menu with a bug report that stays on your PC."], "indented lines continue the item");
        assert_eq!(r.sections[1].items, vec!["Thumbnails come back."]);
        let old = &releases[1];
        assert_eq!((old.version.as_str(), old.date.as_deref()), ("0.1.0", None));
        assert_eq!(old.sections, vec![Section { title: String::new(), items: vec!["Everything else.".into()] }], "items before any ### have an untitled section");
        assert!(parse("").is_empty());
        assert!(parse("just words\n- a stray item\n").is_empty(), "nothing outside a release");
    }

    #[test]
    fn shipped_changelog_matches_the_version() {
        let releases = parse(CHANGELOG);
        let latest = releases.first().expect("CHANGELOG.md has a release");
        assert_eq!(latest.version, VERSION, "the newest CHANGELOG.md entry is this build's version");
        assert!(latest.sections.iter().all(|s| !s.items.is_empty()), "no empty sections");
        // The app shows items as plain text: Markdown marks would appear as they are.
        for item in releases.iter().flat_map(|r| &r.sections).flat_map(|s| &s.items) {
            assert!(!item.contains("**") && !item.contains('`'), "plain text only: {item}");
        }
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0", "0.99.99"));
        assert!(newer("0.1.1", "0.1"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0", "0.2.0"));
        assert!(newer("0.1.0", ""), "anything is newer than nothing");
    }

    #[test]
    fn shows_once_after_update() {
        assert_eq!(on_launch("0.1.0", "0.2.0", true), Launch::Show, "updated");
        assert_eq!(on_launch("0.2.0", "0.2.0", true), Launch::Nothing, "seen: never again");
        assert_eq!(on_launch("", "0.2.0", true), Launch::Show, "updated from a build before this setting existed");
        assert_eq!(on_launch("", "0.2.0", false), Launch::Record, "a fresh install just notes the version");
        assert_eq!(on_launch("0.3.0", "0.2.0", true), Launch::Nothing, "an older build after a newer one changes nothing");

        // In the app: the first load after an update opens the sheet and notes the version.
        use crate::state::{Info, Model};
        let item = |id| rdm_core::Item { id: rdm_core::ItemId(id), url: String::new(), name: String::new(), category: rdm_core::Category::Other, status: rdm_core::Status::Done, dest: None, downloaded: 0, total: None, speed_bps: 0, queue: 0, added: 0, kind: Default::default(), referrer: None, work_dir: None, thumbnail: None, duration: None, retry_at: None, subtitle_links: Vec::new(), subtitle_files: Vec::new() };
        let mut m = Model { items: vec![item(1)], ..Model::default() };
        m.settings.last_seen_version = "0.1.0".into();
        let saved = m.check_version("0.2.0").expect("the new version is noted");
        assert_eq!(saved.last_seen_version, "0.2.0");
        assert_eq!(m.info, Some(Info::WhatsNew { since: Some("0.1.0".into()) }));
        m.info = None;
        assert_eq!(m.check_version("0.2.0"), None, "once per launch");
        assert_eq!(m.info, None);
        // Next launch: the saved settings come back, nothing shows.
        let mut next = Model { items: vec![item(1)], ..Model::default() };
        next.settings = saved;
        assert_eq!(next.check_version("0.2.0"), None);
        assert_eq!(next.info, None);
        // A fresh install: nothing shows, the version is noted.
        let mut fresh = Model::default();
        assert_eq!(fresh.check_version("0.2.0").map(|s| s.last_seen_version), Some("0.2.0".into()));
        assert_eq!(fresh.info, None);
        // Which releases the sheet lists.
        let list = parse(SAMPLE);
        let versions = |since| releases_since(&list, since).iter().map(|r| r.version.clone()).collect::<Vec<_>>();
        assert_eq!(versions(Some("0.1.0")), vec!["0.2.0"], "only what's new since the last version seen");
        assert_eq!(versions(None), vec!["0.2.0", "0.1.0"], "the Help menu lists everything");
        assert_eq!(versions(Some("0.2.0")), vec!["0.2.0"], "never empty: at least the newest");
    }
}
