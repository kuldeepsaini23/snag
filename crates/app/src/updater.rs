//! Snag updating itself: when to offer a newer release, and handing its installer over
//! (Windows), or putting the new AppImage (Linux) or Snag.app (macOS) in place of the running one.

use std::path::Path;

/// How often a running Snag looks again (it also looks shortly after it starts).
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Opening the window (from the tray, a notification, a second start) looks again, but not more
/// often than this.
pub const ON_SHOW_GAP: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// Whether opening the window should look for an update now: never looked, or not for
/// `ON_SHOW_GAP`.
pub fn due_on_show(last: Option<std::time::Instant>, now: std::time::Instant) -> bool {
    last.is_none_or(|at| now.saturating_duration_since(at) >= ON_SHOW_GAP)
}

/// Worth offering: `latest` is newer than this build and wasn't put off with "Later".
pub fn offers(latest: &str, current: &str, later: Option<&str>) -> bool {
    crate::changelog::newer(latest, current) && later != Some(latest)
}

/// The Windows notification for a newly found update (Snag often runs hidden in the tray, where
/// the card in its window isn't seen); `None` when this version was already offered.
pub fn announce(offered: Option<&str>, latest: &str) -> Option<crate::state::Note> {
    (offered != Some(latest)).then(|| crate::state::Note {
        title: format!("Snag {latest} is ready"),
        body: if can_install() {
            "Open Snag and click Update now: it installs in a few seconds and opens again.".into()
        } else {
            "Open Snag and click Download to get the new package.".into()
        },
    })
}

/// The end of a check: a "Check for updates" click hears back even when nothing is newer (the
/// card says it otherwise); the automatic checks stay quiet.
pub fn checked(m: &mut crate::state::Model, offered: Option<&rdm_core::selfupdate::Release>) -> Option<String> {
    let asked = std::mem::take(&mut m.update_checking);
    (asked && offered.is_none()).then(|| format!("No update found: Snag {} is the newest version", crate::changelog::VERSION))
}

/// Inno Setup, quietly: a progress window only, no questions, then Snag opens again
/// (`/RELAUNCH=1`, see installer/snag.iss).
#[cfg_attr(not(windows), allow(dead_code))]
pub const INSTALLER_ARGS: [&str; 5] = ["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/RELAUNCH=1"];

/// Whether "Update now" can update this copy: always on Windows (the installer); on Linux only
/// the AppImage (`$APPIMAGE`), which Snag can replace. A .deb install gets a Download button, as
/// does a Snag.app in a folder this user can't write to (or one macOS runs from a read-only copy).
pub fn can_install() -> bool {
    cfg!(windows) || appimage().is_some() || app_bundle().is_some()
}

/// macOS: the Snag.app this Snag runs from (`…/Snag.app/Contents/MacOS/snag`), when its folder
/// can be written to.
fn app_bundle() -> Option<std::path::PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let app = exe.parent()?.parent()?.parent()?.to_path_buf();
    if app.extension().is_none_or(|x| x != "app") {
        return None;
    }
    let probe = app.parent()?.join(".snag-update-check");
    std::fs::write(&probe, b"").ok()?;
    let _ = std::fs::remove_file(&probe);
    Some(app)
}

/// Linux: the AppImage file this Snag runs from (set by the AppImage runtime).
fn appimage() -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        return None;
    }
    std::env::var_os("APPIMAGE").map(std::path::PathBuf::from).filter(|p| p.is_file())
}

/// Starts the installer; it closes this Snag (`snag.exe --quit`) before replacing it.
#[cfg(windows)]
pub fn run(installer: &Path) -> std::io::Result<()> {
    std::process::Command::new(installer).args(INSTALLER_ARGS).spawn().map(|_| ())
}

/// macOS: the checked zip's Snag.app replaces this one, loses the quarantine flag (so Gatekeeper
/// doesn't stop it), and is opened as a new instance; it waits for this Snag to exit (`--updated`),
/// which the caller does next.
#[cfg(target_os = "macos")]
pub fn run(new: &Path) -> std::io::Result<()> {
    let app = app_bundle().ok_or_else(|| std::io::Error::other("this copy of Snag isn't a Snag.app it can replace"))?;
    rdm_core::selfupdate::replace_app_bundle(new, &app)?;
    let _ = std::fs::remove_file(new);
    let _ = std::process::Command::new("/usr/bin/xattr").args(["-dr", "com.apple.quarantine"]).arg(&app).status();
    std::process::Command::new("/usr/bin/open").arg("-n").arg(&app).args(["--args", "--updated"]).spawn().map(|_| ())
}

/// Linux: the checked AppImage replaces `$APPIMAGE`, and the new one is started; it waits for
/// this Snag to exit (`--updated`), which the caller does next.
#[cfg(target_os = "linux")]
pub fn run(new: &Path) -> std::io::Result<()> {
    let target = appimage().ok_or_else(|| std::io::Error::other("this copy of Snag isn't an AppImage"))?;
    rdm_core::selfupdate::replace_appimage(new, &target)?;
    let _ = std::fs::remove_file(new);
    crate::platform::host_command(&target).arg("--updated").spawn().map(|_| ())
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
    fn opening_the_window_looks_again_at_most_every_ten_minutes() {
        use std::time::{Duration, Instant};
        let now = Instant::now();
        assert!(due_on_show(None, now), "never looked: look now");
        assert!(!due_on_show(Some(now), now), "just looked");
        assert!(!due_on_show(Some(now), now + Duration::from_secs(9 * 60 + 59)));
        assert!(due_on_show(Some(now), now + Duration::from_secs(10 * 60)));
        assert!(due_on_show(Some(now), now + Duration::from_secs(5 * 60 * 60)));
        assert!(!due_on_show(Some(now + Duration::from_secs(60)), now), "a clock that went back isn't due");
        assert_eq!(EVERY, Duration::from_secs(60 * 60), "and every hour while it runs");
    }

    #[test]
    fn a_new_update_is_announced_once() {
        let note = announce(None, "1.0.2").expect("first time: a notification");
        assert_eq!(note.title, "Snag 1.0.2 is ready");
        assert!(note.body.contains(if can_install() { "Update now" } else { "Download" }), "{}", note.body);
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
