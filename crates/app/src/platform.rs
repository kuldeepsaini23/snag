//! What differs between Windows and Linux outside the window itself: opening files, folders and
//! links, showing a file in the file manager, signing in with the session (Linux), and the words
//! the UI uses for the system.

use std::ffi::OsStr;
use std::path::Path;

/// "Follow Windows" / "Follow system": the theme and animation choice that tracks the system.
pub const FOLLOW_SYSTEM: &str = if cfg!(windows) { "Follow Windows" } else { "Follow system" };
/// What "Follow …" follows, under Settings → Appearance → Theme.
pub const THEME_HINT: &str =
    if cfg!(windows) { "Follow Windows switches with your app mode in Windows settings" } else { "Follow system switches with your desktop's light or dark style" };
/// The kind of notification Snag sends.
pub const NOTIFICATIONS: &str = if cfg!(windows) { "Windows notifications" } else { "Desktop notifications" };
pub const NOTIFY_HINT: &str =
    if cfg!(windows) { "A Windows notification when a download finishes or fails" } else { "A desktop notification when a download finishes or fails" };
/// Phone sharing listens on the network, which a firewall may ask about.
pub const SHARING_HINT: &str = if cfg!(windows) {
    "Send links from your phone. Windows may ask to allow Snag on private networks"
} else {
    "Send links from your phone. A firewall may need to allow Snag on your network"
};
/// Where the data folder is, as people would type it.
pub const DATA_DIR: &str = if cfg!(windows) { r"%APPDATA%\Snag" } else { "~/.local/share/snag" };
/// The Help page's last troubleshooting line.
pub const LOG_HINT: &str = if cfg!(windows) {
    "Still stuck? ? → Report a bug saves a report you can send. Errors are also kept in %APPDATA%\\Snag\\snag.log."
} else {
    "Still stuck? ? → Report a bug saves a report you can send. Errors are also kept in ~/.local/share/snag/snag.log."
};
/// What "Include diagnostics" adds to a bug report.
pub const DIAGNOSTICS_HINT: &str = if cfg!(windows) {
    "Snag and Windows versions, yt-dlp and gallery-dl versions, your settings without the pairing code, how many downloads are in each state, and the last 50 lines of Snag's log. Link parameters and your user name in paths are left out."
} else {
    "Snag and Linux versions, yt-dlp and gallery-dl versions, your settings without the pairing code, how many downloads are in each state, and the last 50 lines of Snag's log. Link parameters and your user name in paths are left out."
};

/// A file with its usual program, a folder in a file manager window, or a link in the browser.
#[cfg(windows)]
pub fn open(target: impl AsRef<OsStr>) {
    // Explorer opens a file with its usual program (and a folder in a window).
    let _ = std::process::Command::new("explorer").arg(target).spawn();
}

#[cfg(not(windows))]
pub fn open(target: impl AsRef<OsStr>) {
    let _ = std::process::Command::new("xdg-open").arg(target).spawn();
}

/// Opens Explorer with the file selected; if the file isn't there, opens its folder.
#[cfg(windows)]
pub fn reveal(path: &Path) {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new("explorer");
    if path.exists() {
        cmd.raw_arg(crate::state::explorer_select_arg(path));
    } else if let Some(dir) = path.parent().filter(|d| d.exists()) {
        cmd.arg(dir);
    } else {
        return;
    }
    let _ = cmd.spawn();
}

/// Linux: asks the file manager to show the file selected (`org.freedesktop.FileManager1`,
/// which Nautilus, Dolphin, Nemo, Caja and Thunar answer); without one, opens its folder.
#[cfg(not(windows))]
pub fn reveal(path: &Path) {
    let Some(dir) = path.parent().filter(|d| d.exists()).map(Path::to_path_buf) else { return };
    if !path.exists() {
        open(&dir);
        return;
    }
    let uri = file_uri(path);
    // Off the UI thread: the call waits for the file manager's answer.
    std::thread::spawn(move || {
        let shown = std::process::Command::new("dbus-send")
            .args(["--session", "--print-reply", "--dest=org.freedesktop.FileManager1", "/org/freedesktop/FileManager1", "org.freedesktop.FileManager1.ShowItems"])
            .arg(format!("array:string:{uri}"))
            .arg("string:")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !shown {
            open(&dir);
        }
    });
}

/// `file:///home/a/My%20Video.mp4`: a path as a URI (every byte but the unreserved ones escaped).
#[cfg_attr(windows, allow(dead_code))]
pub fn file_uri(path: &Path) -> String {
    use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
    const KEEP: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/').remove(b'-').remove(b'_').remove(b'.').remove(b'~');
    format!("file://{}", utf8_percent_encode(&path.to_string_lossy(), KEEP))
}

/// Linux: `~/.config/autostart/snag.desktop`, which the desktop runs at sign-in.
#[cfg(not(windows))]
fn autostart_file() -> std::path::PathBuf {
    rdm_core::dirs::config_home().join("autostart").join("snag.desktop")
}

/// Linux: Snag starts (in the tray, no window) when you sign in.
#[cfg(not(windows))]
pub fn autostart() -> bool {
    autostart_file().exists()
}

#[cfg(windows)]
pub fn autostart() -> bool {
    false
}

/// Linux: writes or removes the autostart entry. The AppImage is started through `$APPIMAGE`
/// (the mounted copy's path changes every run).
#[cfg(not(windows))]
pub fn set_autostart(on: bool) -> Result<(), String> {
    let file = autostart_file();
    if !on {
        return match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Couldn't remove {}: {e}", file.display())),
            _ => Ok(()),
        };
    }
    let exe = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from).or_else(|| std::env::current_exe().ok()).ok_or("Couldn't find Snag's own program file")?;
    let write = || {
        std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))?;
        std::fs::write(&file, autostart_entry(&exe))
    };
    write().map_err(|e| format!("Couldn't write {}: {e}", file.display()))
}

#[cfg(windows)]
pub fn set_autostart(_on: bool) -> Result<(), String> {
    Err("Start at sign-in is set by the installer on Windows".into())
}

/// The `.desktop` entry that starts `exe --background` at sign-in.
#[cfg_attr(windows, allow(dead_code))]
pub fn autostart_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Snag\nComment=Download manager (waits in the tray)\nExec={} --background\nIcon=snag\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        desktop_exec_arg(&exe.to_string_lossy())
    )
}

/// One `Exec=` argument, quoted when it needs to be (Desktop Entry spec: `"`, `` ` ``, `$` and
/// `\` are escaped inside quotes, and `%` is doubled).
fn desktop_exec_arg(arg: &str) -> String {
    let arg = arg.replace('%', "%%");
    if !arg.chars().any(|c| c.is_whitespace() || "\"'\\><~|&;$*?#()`".contains(c)) {
        return arg;
    }
    let mut out = String::from("\"");
    // Each escape's backslash is itself escaped once more by the file's string rules.
    for c in arg.chars() {
        match c {
            '\\' => out.push_str(r"\\\\"),
            '"' | '`' | '$' => {
                out.push_str(r"\\");
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_become_file_uris() {
        assert_eq!(file_uri(Path::new("/home/a/My Video (1).mp4")), "file:///home/a/My%20Video%20%281%29.mp4");
        assert_eq!(file_uri(Path::new("/tmp/é😳.txt")), "file:///tmp/%C3%A9%F0%9F%98%B3.txt");
        assert_eq!(file_uri(Path::new("/x/a-b_c.d~e")), "file:///x/a-b_c.d~e");
    }

    #[test]
    fn the_autostart_entry_runs_snag_in_the_tray() {
        let entry = autostart_entry(Path::new("/home/a/Apps/Snag-x86_64.AppImage"));
        assert!(entry.starts_with("[Desktop Entry]\n"), "{entry}");
        assert!(entry.contains("\nExec=/home/a/Apps/Snag-x86_64.AppImage --background\n"), "{entry}");
        let spaced = autostart_entry(Path::new("/home/a/My Apps/100% $nag.AppImage"));
        assert!(spaced.contains(r#"Exec="/home/a/My Apps/100%% \\$nag.AppImage" --background"#), "{spaced}");
    }
}
