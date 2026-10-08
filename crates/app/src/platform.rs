//! What differs between Windows, Linux and macOS outside the window itself: opening files,
//! folders and links, showing a file in the file manager, starting with the session (Linux,
//! macOS), and the words the UI uses for the system.

use std::ffi::OsStr;
use std::path::Path;

/// "Follow Windows" / "Follow system": the theme and animation choice that tracks the system.
pub const FOLLOW_SYSTEM: &str = if cfg!(windows) { "Follow Windows" } else { "Follow system" };
/// What "Follow …" follows, under Settings → Appearance → Theme.
pub const THEME_HINT: &str = if cfg!(windows) {
    "Follow Windows switches with your app mode in Windows settings"
} else if cfg!(target_os = "macos") {
    "Follow system switches with your Mac's Light or Dark appearance"
} else {
    "Follow system switches with your desktop's light or dark style"
};
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
/// What closing the window does (Help → Basics).
pub const CLOSE_HINT: &str = if cfg!(target_os = "macos") {
    "Closing the window keeps Snag downloading in the menu bar. Quit from its menu bar icon or with Cmd + Q."
} else {
    "Closing the window keeps Snag downloading in the tray. Quit from the tray icon."
};
/// Where the data folder is, as people would type it.
pub const DATA_DIR: &str = if cfg!(windows) {
    r"%APPDATA%\Snag"
} else if cfg!(target_os = "macos") {
    "~/Library/Application Support/Snag"
} else {
    "~/.local/share/snag"
};
/// The Help page's last troubleshooting line.
pub const LOG_HINT: &str = if cfg!(windows) {
    "Still stuck? ? → Report a bug saves a report you can send. Errors are also kept in %APPDATA%\\Snag\\snag.log."
} else if cfg!(target_os = "macos") {
    "Still stuck? ? → Report a bug saves a report you can send. Errors are also kept in ~/Library/Application Support/Snag/snag.log."
} else {
    "Still stuck? ? → Report a bug saves a report you can send. Errors are also kept in ~/.local/share/snag/snag.log."
};
/// What "Include diagnostics" adds to a bug report.
pub const DIAGNOSTICS_HINT: &str = if cfg!(windows) {
    "Snag and Windows versions, yt-dlp and gallery-dl versions, your settings without the pairing code, how many downloads are in each state, and the last 50 lines of Snag's log. Link parameters and your user name in paths are left out."
} else if cfg!(target_os = "macos") {
    "Snag and macOS versions, yt-dlp and gallery-dl versions, your settings without the pairing code, how many downloads are in each state, and the last 50 lines of Snag's log. Link parameters and your user name in paths are left out."
} else {
    "Snag and Linux versions, yt-dlp and gallery-dl versions, your settings without the pairing code, how many downloads are in each state, and the last 50 lines of Snag's log. Link parameters and your user name in paths are left out."
};

/// A file with its usual program, a folder in a file manager window, or a link in the browser.
#[cfg(windows)]
pub fn open(target: impl AsRef<OsStr>) {
    // Explorer opens a file with its usual program (and a folder in a window).
    let _ = std::process::Command::new("explorer").arg(target).spawn();
}

#[cfg(target_os = "linux")]
pub fn open(target: impl AsRef<OsStr>) {
    let _ = host_command("xdg-open").arg(target).spawn();
}

/// macOS: `open` (the Finder for a folder, the default app for a file or link).
#[cfg(target_os = "macos")]
pub fn open(target: impl AsRef<OsStr>) {
    let _ = std::process::Command::new("/usr/bin/open").arg(target).spawn();
}

/// Linux: a program of the desktop's (xdg-open, the file manager, gsettings), started without
/// what the AppImage's launcher set up for Snag itself: paths into its bundle (`$APPDIR/…`) would
/// break the programs it starts, and vanish when Snag exits.
#[cfg(target_os = "linux")]
pub fn host_command(program: impl AsRef<OsStr>) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    if let Some(appdir) = std::env::var("APPDIR").ok().filter(|d| d.len() > 1) {
        for (key, value) in host_env(std::env::vars_os(), &appdir) {
            match value {
                Some(v) => cmd.env(key, v),
                None => cmd.env_remove(key),
            };
        }
    }
    cmd
}

/// The variables to change for `host_command`: each one naming something inside `appdir` loses
/// those entries (`a:b` lists keep the rest), or goes when nothing is left.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn host_env(vars: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>, appdir: &str) -> Vec<(std::ffi::OsString, Option<String>)> {
    let appdir = appdir.trim_end_matches('/');
    let inside = |entry: &str| entry == appdir || entry.starts_with(&format!("{appdir}/"));
    vars.filter_map(|(key, value)| {
        let value = value.into_string().ok()?;
        if !value.split(':').any(inside) {
            return None;
        }
        let kept: Vec<&str> = value.split(':').filter(|e| !inside(e)).collect();
        let kept = kept.join(":");
        Some((key, (!kept.is_empty()).then_some(kept)))
    })
    .collect()
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

/// macOS: the Finder shows the file selected (`open -R`); if the file isn't there, its folder.
#[cfg(target_os = "macos")]
pub fn reveal(path: &Path) {
    if path.exists() {
        let _ = std::process::Command::new("/usr/bin/open").arg("-R").arg(path).spawn();
    } else if let Some(dir) = path.parent().filter(|d| d.exists()) {
        open(dir);
    }
}

/// Linux: asks the file manager to show the file selected (`org.freedesktop.FileManager1`,
/// which Nautilus, Dolphin, Nemo, Caja and Thunar answer); without one, opens its folder.
#[cfg(target_os = "linux")]
pub fn reveal(path: &Path) {
    let Some(dir) = path.parent().filter(|d| d.exists()).map(Path::to_path_buf) else { return };
    if !path.exists() {
        open(&dir);
        return;
    }
    let uri = file_uri(path);
    // Off the UI thread: the call waits for the file manager's answer.
    std::thread::spawn(move || {
        let shown = host_command("dbus-send")
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
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn file_uri(path: &Path) -> String {
    use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
    const KEEP: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/').remove(b'-').remove(b'_').remove(b'.').remove(b'~');
    format!("file://{}", utf8_percent_encode(&path.to_string_lossy(), KEEP))
}

/// Linux: `~/.config/autostart/snag.desktop`, which the desktop runs at sign-in.
#[cfg(target_os = "linux")]
fn autostart_file() -> std::path::PathBuf {
    rdm_core::dirs::config_home().join("autostart").join("snag.desktop")
}

/// macOS: `~/Library/LaunchAgents/dev.kuldeepsaini.snag.plist`, which launchd runs at login.
#[cfg(target_os = "macos")]
fn autostart_file() -> std::path::PathBuf {
    rdm_core::dirs::home().join("Library").join("LaunchAgents").join(format!("{BUNDLE_ID}.plist"))
}

/// The app's bundle identifier (packaging/macos/Info.plist), also the LaunchAgent's label.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const BUNDLE_ID: &str = "dev.kuldeepsaini.snag";

/// Linux, macOS: Snag starts (in the tray or menu bar, no window) when you sign in.
#[cfg(unix)]
pub fn autostart() -> bool {
    autostart_file().exists()
}

#[cfg(windows)]
pub fn autostart() -> bool {
    false
}

/// Linux, macOS: writes or removes the autostart entry. The AppImage is started through
/// `$APPIMAGE` (the mounted copy's path changes every run); on macOS the program inside Snag.app.
#[cfg(unix)]
pub fn set_autostart(on: bool) -> Result<(), String> {
    let file = autostart_file();
    if !on {
        return match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Couldn't remove {}: {e}", file.display())),
            _ => Ok(()),
        };
    }
    let exe = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from).or_else(|| std::env::current_exe().ok()).ok_or("Couldn't find Snag's own program file")?;
    #[cfg(target_os = "macos")]
    let entry = launch_agent(&exe);
    #[cfg(not(target_os = "macos"))]
    let entry = autostart_entry(&exe);
    let write = || {
        std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))?;
        std::fs::write(&file, entry)
    };
    write().map_err(|e| format!("Couldn't write {}: {e}", file.display()))
}

#[cfg(windows)]
pub fn set_autostart(_on: bool) -> Result<(), String> {
    Err("Start at sign-in is set by the installer on Windows".into())
}

/// macOS: the LaunchAgent that starts `exe --background` at login (only in the user's graphical
/// session, once: it isn't restarted when you quit Snag).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn launch_agent(exe: &Path) -> String {
    let exe = exe.to_string_lossy().replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{BUNDLE_ID}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{exe}</string>
		<string>--background</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>ProcessType</key>
	<string>Interactive</string>
	<key>LimitLoadToSessionType</key>
	<string>Aqua</string>
</dict>
</plist>
"#
    )
}

/// The `.desktop` entry that starts `exe --background` at sign-in.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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
    fn programs_snag_starts_leave_the_appimage_behind() {
        let vars = [
            ("APPDIR", "/tmp/.mount_SnagAb"),
            ("APPIMAGE", "/home/a/Snag-x86_64.AppImage"),
            ("GDK_PIXBUF_MODULE_FILE", "/tmp/.mount_SnagAb/usr/lib/loaders.cache"),
            ("XDG_DATA_DIRS", "/tmp/.mount_SnagAb/usr/share:/usr/local/share:/usr/share"),
            ("PATH", "/tmp/.mount_SnagAb/usr/bin:/usr/bin"),
            ("HOME", "/home/a"),
            ("OTHER", "/tmp/.mount_SnagAbc/x"),
        ]
        .map(|(k, v)| (k.into(), v.into()));
        let mut changes = host_env(vars.into_iter(), "/tmp/.mount_SnagAb/");
        changes.sort();
        let expected: Vec<(std::ffi::OsString, Option<String>)> = vec![
            ("APPDIR".into(), None),
            ("GDK_PIXBUF_MODULE_FILE".into(), None),
            ("PATH".into(), Some("/usr/bin".into())),
            ("XDG_DATA_DIRS".into(), Some("/usr/local/share:/usr/share".into())),
        ];
        assert_eq!(changes, expected, "the rest (the AppImage file itself, a look-alike folder) untouched");
    }

    #[test]
    fn the_launch_agent_runs_snag_in_the_menu_bar() {
        let plist = launch_agent(Path::new("/Applications/Snag & Co.app/Contents/MacOS/snag"));
        assert!(plist.contains("<string>dev.kuldeepsaini.snag</string>"), "{plist}");
        assert!(plist.contains("<string>/Applications/Snag &amp; Co.app/Contents/MacOS/snag</string>\n\t\t<string>--background</string>"), "{plist}");
        assert!(plist.contains("<key>RunAtLoad</key>\n\t<true/>"), "{plist}");
        assert!(!plist.contains("KeepAlive"), "quitting Snag keeps it quit");
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
