//! Snag updating itself: when to offer a newer release, and handing its installer over.

use std::path::Path;

/// How often a running Snag looks again (it also looks shortly after it starts).
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// Worth offering: `latest` is newer than this build and wasn't put off with "Later".
pub fn offers(latest: &str, current: &str, later: Option<&str>) -> bool {
    crate::changelog::newer(latest, current) && later != Some(latest)
}

/// The Windows notification for a newly found update (Snag often runs hidden in the tray, where
/// the card in its window isn't seen); `None` when this version was already offered.
pub fn announce(offered: Option<&str>, latest: &str) -> Option<crate::state::Note> {
    (offered != Some(latest)).then(|| crate::state::Note {
        title: format!("Snag {latest} is ready"),
        body: "Open Snag and click Update now: it installs in a few seconds and opens again.".into(),
    })
}

/// Inno Setup, quietly: a progress window only, no questions, then Snag opens again
/// (`/RELAUNCH=1`, see installer/snag.iss).
pub const INSTALLER_ARGS: [&str; 5] = ["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/RELAUNCH=1"];

/// Starts the installer; it closes this Snag (`snag.exe --quit`) before replacing it.
pub fn run(installer: &Path) -> std::io::Result<()> {
    std::process::Command::new(installer).args(INSTALLER_ARGS).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_newer_releases_are_offered() {
        assert!(offers("1.0.2", "1.0.1", None));
        assert!(offers("1.1.0", "1.0.9", None));
        assert!(!offers("1.0.1", "1.0.1", None), "the same version");
        assert!(!offers("1.0.0", "1.0.1", None), "an older one");
        assert!(!offers("1.0.2", "1.0.1", Some("1.0.2")), "put off with Later");
        assert!(offers("1.0.3", "1.0.1", Some("1.0.2")), "a newer one than the one put off");
    }

    #[test]
    fn a_new_update_is_announced_once() {
        let note = announce(None, "1.0.2").expect("first time: a notification");
        assert_eq!(note.title, "Snag 1.0.2 is ready");
        assert!(note.body.contains("Update now"), "{}", note.body);
        assert_eq!(announce(Some("1.0.2"), "1.0.2"), None, "already offered: no second notification");
        assert!(announce(Some("1.0.2"), "1.0.3").is_some(), "a newer one is announced");
    }

    #[test]
    fn the_installer_runs_quietly_and_reopens_snag() {
        assert!(INSTALLER_ARGS.contains(&"/SILENT") && INSTALLER_ARGS.contains(&"/SUPPRESSMSGBOXES"));
        assert!(INSTALLER_ARGS.contains(&"/RELAUNCH=1"));
        assert!(!INSTALLER_ARGS.contains(&"/VERYSILENT"), "a progress window shows something is happening");
    }
}
