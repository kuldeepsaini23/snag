//! Where things live on this system: the home folder, Snag's data folder, Downloads and Desktop.
//! Windows keeps its long-standing places (`%APPDATA%\Snag`, `%USERPROFILE%\Downloads`); Linux
//! follows the XDG base and user directories; macOS uses `~/Library/Application Support/Snag`.

use std::path::{Path, PathBuf};

/// The user's home folder (`%USERPROFILE%` on Windows, `$HOME` elsewhere); empty if unset.
pub fn home() -> PathBuf {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).map(PathBuf::from).unwrap_or_default()
}

/// Snag's own folder: state.json, the log, its tools (`bin/`) and updates.
/// `%APPDATA%\Snag` on Windows; `$XDG_DATA_HOME/snag` (`~/.local/share/snag`) on Linux;
/// `~/Library/Application Support/Snag` on macOS.
pub fn app_dir() -> PathBuf {
    if cfg!(windows) {
        crate::store::data_dir(&std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default())
    } else if cfg!(target_os = "macos") {
        home().join("Library").join("Application Support").join("Snag")
    } else {
        xdg_home("XDG_DATA_HOME", ".local/share").join("snag")
    }
}

/// `~/.config` (or `$XDG_CONFIG_HOME`), e.g. for `autostart/`.
pub fn config_home() -> PathBuf {
    xdg_home("XDG_CONFIG_HOME", ".config")
}

/// An XDG base directory: the variable when it holds an absolute path, else `~/<fallback>`.
fn xdg_home(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(fallback))
}

/// The Downloads folder: `%USERPROFILE%\Downloads` on Windows, `~/Downloads` on macOS; on Linux
/// the XDG download directory (`~/.config/user-dirs.dirs`), else `~/Downloads`.
pub fn downloads() -> PathBuf {
    if cfg!(any(windows, target_os = "macos")) {
        return home().join("Downloads");
    }
    user_dir("XDG_DOWNLOAD_DIR").unwrap_or_else(|| home().join("Downloads"))
}

/// Linux: the Desktop as `user-dirs.dirs` names it, else `~/Desktop` (when either exists).
/// macOS: `~/Desktop`.
pub fn desktop() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return Some(home().join("Desktop")).filter(|p| p.is_dir());
    }
    user_dir("XDG_DESKTOP_DIR").filter(|p| p.is_dir()).or_else(|| Some(home().join("Desktop")).filter(|p| p.is_dir()))
}

fn user_dir(key: &str) -> Option<PathBuf> {
    let text = std::fs::read_to_string(config_home().join("user-dirs.dirs")).ok()?;
    parse_user_dir(&text, key, &home())
}

/// One entry of `user-dirs.dirs` (`XDG_DOWNLOAD_DIR="$HOME/Downloads"`). Only `$HOME/…` and
/// absolute paths are allowed (the file's own rules); `$HOME/` alone means "not set".
pub fn parse_user_dir(text: &str, key: &str, home: &Path) -> Option<PathBuf> {
    let value = text.lines().map(str::trim).filter(|l| !l.starts_with('#')).find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('='))?;
    let value = value.trim().trim_matches('"');
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None if value.starts_with('/') => PathBuf::from(value),
        None => return None,
    };
    (path != home).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_xdg_user_dirs() {
        let text = "# written by xdg-user-dirs-update\nXDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\nXDG_DOWNLOAD_DIR=\"/data/dl\"\nXDG_MUSIC_DIR=\"$HOME/\"\n";
        let home = Path::new("/home/a");
        assert_eq!(parse_user_dir(text, "XDG_DESKTOP_DIR", home), Some(home.join("Schreibtisch")));
        assert_eq!(parse_user_dir(text, "XDG_DOWNLOAD_DIR", home), Some(PathBuf::from("/data/dl")), "absolute");
        assert_eq!(parse_user_dir(text, "XDG_MUSIC_DIR", home), None, "the home folder itself: not set");
        assert_eq!(parse_user_dir(text, "XDG_VIDEOS_DIR", home), None, "missing");
        assert_eq!(parse_user_dir("XDG_DOWNLOAD_DIR=\"Downloads\"", "XDG_DOWNLOAD_DIR", home), None, "relative: not allowed");
        assert_eq!(parse_user_dir("# XDG_DOWNLOAD_DIR=\"/x\"", "XDG_DOWNLOAD_DIR", home), None, "commented out");
    }
}
