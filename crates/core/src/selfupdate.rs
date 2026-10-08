//! Snag's own updates: the newest GitHub release, its installer and the checksum it must match.
//! Windows installs with `Snag-Setup-<version>.exe`; Linux replaces its AppImage with the
//! release's `Snag-x86_64.AppImage`; macOS replaces Snag.app with the one in `Snag-macOS.zip`.

/// Which release file updates this copy of Snag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
}

impl Platform {
    pub const CURRENT: Platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Linux
    };

    /// The release asset this platform installs, as named in the checksums file.
    pub fn installer_name(self, version: &str) -> String {
        match self {
            Platform::Windows => format!("Snag-Setup-{version}.exe"),
            Platform::Linux => APPIMAGE.into(),
            Platform::MacOS => MAC_ZIP.into(),
        }
    }
}

/// The macOS release: Snag.app zipped (universal; the same name in every release).
pub const MAC_ZIP: &str = "Snag-macOS.zip";

/// The Linux release's AppImage (the same name in every release; SHA256SUMS-<version>.txt lists it).
pub const APPIMAGE: &str = "Snag-x86_64.AppImage";

/// Where releases are listed (tests point it at a local server with `SNAG_UPDATE_URL`).
pub fn latest_url() -> String {
    const GITHUB: &str = "https://api.github.com/repos/kuldeepsaini23/snag/releases/latest";
    // Release builds always ask GitHub: an environment variable must not pick what gets installed.
    if !cfg!(debug_assertions) {
        return GITHUB.into();
    }
    std::env::var("SNAG_UPDATE_URL").unwrap_or_else(|_| GITHUB.into())
}

/// A published version of Snag.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    /// "1.0.2"
    pub version: String,
    /// The installer to download.
    pub installer: String,
    /// The installer's file name, as listed in the checksums file.
    pub installer_name: String,
    /// The release's SHA256SUMS file.
    pub sums: String,
    /// The release page (its notes).
    pub page: String,
}

/// The release in GitHub's "latest release" answer; `None` for a draft or pre-release, or one
/// without both an installer and a checksums file (never installed unchecked).
pub fn parse_latest(json: &str) -> Option<Release> {
    parse_latest_for(json, Platform::CURRENT)
}

/// `parse_latest` for `platform`'s installer.
pub fn parse_latest_for(json: &str, platform: Platform) -> Option<Release> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        return None;
    }
    let version = v["tag_name"].as_str()?.trim_start_matches('v').to_string();
    let assets = v["assets"].as_array()?;
    let asset = |name: &str| assets.iter().find(|a| a["name"].as_str() == Some(name)).and_then(|a| a["browser_download_url"].as_str()).map(String::from);
    let installer_name = platform.installer_name(&version);
    Some(Release {
        installer: asset(&installer_name)?,
        sums: asset(&format!("SHA256SUMS-{version}.txt"))?,
        page: v["html_url"].as_str().unwrap_or("https://github.com/kuldeepsaini23/snag/releases/latest").to_string(),
        installer_name,
        version,
    })
}

/// The SHA-256 the checksums file gives for `file` (lines like `<hash> *installer/<file>`).
pub fn checksum_for(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.trim().split_once(char::is_whitespace)?;
        let name = name.trim().trim_start_matches('*');
        let base = name.rsplit(['/', '\\']).next()?;
        (base == file && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())).then(|| hash.to_ascii_lowercase())
    })
}

/// Puts the downloaded, checked AppImage `new` in place of the running one (`target`, which is
/// `$APPIMAGE`): copied next to it first (same file system), made executable, then renamed over
/// it in one step, so `target` is always either the old or the new Snag. The running copy keeps
/// working from the old file until it exits.
#[cfg(unix)]
pub fn replace_appimage(new: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let dir = target.parent().ok_or_else(|| std::io::Error::other("the AppImage has no folder"))?;
    let name = target.file_name().ok_or_else(|| std::io::Error::other("the AppImage has no name"))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".update");
    let tmp = dir.join(tmp_name);
    let placed = (|| {
        std::fs::copy(new, &tmp)?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        std::fs::File::open(&tmp)?.sync_all()?;
        std::fs::rename(&tmp, target)
    })();
    if placed.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    placed
}

/// Puts the Snag.app inside the checked `zip` in place of the running one (`app`, the `.app`
/// folder): unpacked next to it first (same file system), then the two swapped by renames, so
/// `app` is the old or the new Snag at any moment and a failure leaves the old one. The running
/// copy keeps its open files until it exits.
pub fn replace_app_bundle(zip: &std::path::Path, app: &std::path::Path) -> std::io::Result<()> {
    let dir = app.parent().ok_or_else(|| std::io::Error::other("Snag.app has no folder"))?;
    let name = app.file_name().ok_or_else(|| std::io::Error::other("Snag.app has no name"))?;
    let hidden = |suffix: &str| {
        let mut n = std::ffi::OsString::from(".");
        n.push(name);
        n.push(suffix);
        dir.join(n)
    };
    let (staging, old) = (hidden(".update"), hidden(".old"));
    for leftover in [&staging, &old] {
        let _ = std::fs::remove_dir_all(leftover);
    }
    let swapped = (|| {
        let file = std::fs::File::open(zip)?;
        zip::ZipArchive::new(std::io::BufReader::new(file)).and_then(|mut z| z.extract(&staging)).map_err(std::io::Error::other)?;
        let new = bundle_in(&staging)?;
        std::fs::rename(app, &old)?;
        if let Err(e) = std::fs::rename(&new, app) {
            let _ = std::fs::rename(&old, app);
            return Err(e);
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(&old);
    swapped
}

/// The one `*.app` folder at the top of an unpacked release zip.
fn bundle_in(dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let mut apps = std::fs::read_dir(dir)?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir() && p.extension().is_some_and(|x| x == "app"));
    match (apps.next(), apps.next()) {
        (Some(app), None) if app.join("Contents").join("Info.plist").is_file() => Ok(app),
        _ => Err(std::io::Error::other("the update doesn't hold one Snag.app")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATEST: &str = r#"{
        "tag_name": "v1.0.2", "draft": false, "prerelease": false,
        "html_url": "https://github.com/kuldeepsaini23/snag/releases/tag/v1.0.2",
        "assets": [
            {"name": "Snag-Setup.exe", "browser_download_url": "https://x.test/Snag-Setup.exe"},
            {"name": "Snag-Setup-1.0.2.exe", "browser_download_url": "https://x.test/Snag-Setup-1.0.2.exe"},
            {"name": "SHA256SUMS-1.0.2.txt", "browser_download_url": "https://x.test/SHA256SUMS-1.0.2.txt"}
        ]
    }"#;

    #[test]
    fn reads_the_latest_release() {
        let r = parse_latest_for(LATEST, Platform::Windows).expect("a release");
        assert_eq!(r.version, "1.0.2");
        assert_eq!(r.installer, "https://x.test/Snag-Setup-1.0.2.exe", "the versioned file: it's the one the checksums list");
        assert_eq!(r.installer_name, "Snag-Setup-1.0.2.exe");
        assert_eq!(r.sums, "https://x.test/SHA256SUMS-1.0.2.txt");
        assert_eq!(r.page, "https://github.com/kuldeepsaini23/snag/releases/tag/v1.0.2");
    }

    #[test]
    fn skips_what_shouldnt_be_installed() {
        assert_eq!(parse_latest_for(&LATEST.replace(r#""prerelease": false"#, r#""prerelease": true"#), Platform::Windows), None, "pre-release");
        assert_eq!(parse_latest_for(&LATEST.replace(r#""draft": false"#, r#""draft": true"#), Platform::Windows), None, "draft");
        assert_eq!(parse_latest_for(&LATEST.replace("SHA256SUMS-1.0.2.txt", "notes.txt"), Platform::Windows), None, "no checksums: never installed unchecked");
        assert_eq!(parse_latest_for(&LATEST.replace("Snag-Setup-1.0.2.exe", "Other.exe"), Platform::Windows), None, "no installer");
        assert_eq!(parse_latest_for("{\"message\": \"Not Found\"}", Platform::Windows), None);
        assert_eq!(parse_latest_for("not json", Platform::Windows), None);
    }

    const LATEST_BOTH: &str = r#"{
        "tag_name": "v1.1.0", "draft": false, "prerelease": false,
        "html_url": "https://github.com/kuldeepsaini23/snag/releases/tag/v1.1.0",
        "assets": [
            {"name": "Snag-Setup-1.1.0.exe", "browser_download_url": "https://x.test/Snag-Setup-1.1.0.exe"},
            {"name": "Snag-x86_64.AppImage", "browser_download_url": "https://x.test/Snag-x86_64.AppImage"},
            {"name": "snag_1.1.0_amd64.deb", "browser_download_url": "https://x.test/snag_1.1.0_amd64.deb"},
            {"name": "SHA256SUMS-1.1.0.txt", "browser_download_url": "https://x.test/SHA256SUMS-1.1.0.txt"}
        ]
    }"#;

    #[test]
    fn each_platform_picks_its_own_installer() {
        let win = parse_latest_for(LATEST_BOTH, Platform::Windows).expect("windows");
        assert_eq!((win.installer.as_str(), win.installer_name.as_str()), ("https://x.test/Snag-Setup-1.1.0.exe", "Snag-Setup-1.1.0.exe"));
        let linux = parse_latest_for(LATEST_BOTH, Platform::Linux).expect("linux");
        assert_eq!((linux.installer.as_str(), linux.installer_name.as_str()), ("https://x.test/Snag-x86_64.AppImage", APPIMAGE));
        assert_eq!(linux.sums, "https://x.test/SHA256SUMS-1.1.0.txt", "the same checksums file");
        assert_eq!(parse_latest_for(LATEST, Platform::Linux), None, "a Windows-only release offers Linux nothing");
        assert_eq!(parse_latest_for(&LATEST_BOTH.replace("SHA256SUMS-1.1.0.txt", "x.txt"), Platform::Linux), None, "never unchecked");
    }

    const LATEST_ALL: &str = r#"{
        "tag_name": "v1.2.0", "draft": false, "prerelease": false,
        "html_url": "https://github.com/kuldeepsaini23/snag/releases/tag/v1.2.0",
        "assets": [
            {"name": "Snag-Setup-1.2.0.exe", "browser_download_url": "https://x.test/Snag-Setup-1.2.0.exe"},
            {"name": "Snag-x86_64.AppImage", "browser_download_url": "https://x.test/Snag-x86_64.AppImage"},
            {"name": "Snag-macOS.dmg", "browser_download_url": "https://x.test/Snag-macOS.dmg"},
            {"name": "Snag-macOS.zip", "browser_download_url": "https://x.test/Snag-macOS.zip"},
            {"name": "SHA256SUMS-1.2.0.txt", "browser_download_url": "https://x.test/SHA256SUMS-1.2.0.txt"}
        ]
    }"#;

    #[test]
    fn a_mac_takes_the_zip_not_the_disk_image() {
        let mac = parse_latest_for(LATEST_ALL, Platform::MacOS).expect("macos");
        assert_eq!((mac.installer.as_str(), mac.installer_name.as_str()), ("https://x.test/Snag-macOS.zip", MAC_ZIP));
        assert_eq!(mac.sums, "https://x.test/SHA256SUMS-1.2.0.txt", "the same checksums file");
        assert_eq!(parse_latest_for(LATEST_ALL, Platform::Windows).unwrap().installer_name, "Snag-Setup-1.2.0.exe");
        assert_eq!(parse_latest_for(LATEST_ALL, Platform::Linux).unwrap().installer_name, APPIMAGE);
        assert_eq!(parse_latest_for(LATEST_BOTH, Platform::MacOS), None, "a release without a Mac build offers a Mac nothing");
        assert_eq!(parse_latest_for(&LATEST_ALL.replace("SHA256SUMS-1.2.0.txt", "x.txt"), Platform::MacOS), None, "never unchecked");
        let (a, b) = ("a".repeat(64), "b".repeat(64));
        // As the release job appends it: shasum over the macOS files.
        let sums = format!("{a} *installer/Snag-Setup-1.2.0.exe\n{b}  macos/Snag-macOS.zip\n");
        assert_eq!(checksum_for(&sums, MAC_ZIP), Some(b));
    }

    /// A release zip as `ditto -c -k --keepParent Snag.app` makes it: `Snag.app/…` at the top.
    fn mac_zip(at: &std::path::Path, version: &[u8]) {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::fs::File::create(at).unwrap());
        let exe = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.add_directory("Snag.app/", exe).unwrap();
        zip.start_file("Snag.app/Contents/Info.plist", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"<plist/>").unwrap();
        zip.start_file("Snag.app/Contents/MacOS/snag", exe).unwrap();
        zip.write_all(version).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn the_app_bundle_is_swapped_whole() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("Applications");
        let app = apps.join("Snag.app");
        std::fs::create_dir_all(app.join("Contents").join("MacOS")).unwrap();
        std::fs::write(app.join("Contents").join("MacOS").join("snag"), b"old").unwrap();
        std::fs::write(app.join("Contents").join("stale.txt"), b"only in the old one").unwrap();
        let zip = dir.path().join("Snag-macOS.zip");
        mac_zip(&zip, b"new");
        replace_app_bundle(&zip, &app).unwrap();
        assert_eq!(std::fs::read(app.join("Contents").join("MacOS").join("snag")).unwrap(), b"new");
        assert!(!app.join("Contents").join("stale.txt").exists(), "the whole bundle is the new one");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(app.join("Contents").join("MacOS").join("snag")).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "runnable");
        }
        let left: Vec<_> = std::fs::read_dir(&apps).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, ["Snag.app"], "no staging or old copy left behind");
    }

    #[test]
    fn a_bad_update_leaves_the_old_app() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Snag.app");
        std::fs::create_dir_all(app.join("Contents")).unwrap();
        std::fs::write(app.join("Contents").join("Info.plist"), b"old").unwrap();
        let not_zip = dir.path().join("Snag-macOS.zip");
        std::fs::write(&not_zip, b"<html>not found</html>").unwrap();
        assert!(replace_app_bundle(&not_zip, &app).is_err());
        let empty = dir.path().join("empty.zip");
        zip::ZipWriter::new(std::fs::File::create(&empty).unwrap()).finish().unwrap();
        assert!(replace_app_bundle(&empty, &app).is_err(), "no Snag.app inside");
        assert_eq!(std::fs::read(app.join("Contents").join("Info.plist")).unwrap(), b"old");
        let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).filter(|n| n.starts_with('.')).collect();
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn finds_the_appimages_checksum() {
        let (a, b, c) = ("a".repeat(64), "b".repeat(64), "c".repeat(64));
        // As the release job writes it: sha256sum over the Windows and the Linux files.
        let sums = format!("{a} *installer/Snag-Setup-1.1.0.exe\n{b}  linux/Snag-x86_64.AppImage\n{c}  linux/snag_1.1.0_amd64.deb\n");
        assert_eq!(checksum_for(&sums, APPIMAGE), Some(b));
        assert_eq!(checksum_for(&sums, "snag_1.1.0_amd64.deb"), Some(c));
        assert_eq!(checksum_for(&sums, "Snag-Setup-1.1.0.exe"), Some(a));
    }

    #[cfg(unix)]
    #[test]
    fn the_appimage_is_replaced_in_one_step() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("apps").join("Snag-x86_64.AppImage");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"old").unwrap();
        let new = dir.path().join("download");
        std::fs::write(&new, b"new").unwrap();
        replace_appimage(&new, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(std::fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755, "runnable");
        let left: Vec<_> = std::fs::read_dir(target.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, ["Snag-x86_64.AppImage"], "no temporary file left behind");
        assert!(replace_appimage(&dir.path().join("missing"), &target).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"new", "a failed update leaves the old one");
    }

    #[test]
    fn finds_the_installers_checksum() {
        let a = "a".repeat(64);
        let b = "B".repeat(64);
        let sums = format!("{a} *installer/Snag-Setup-1.0.2.exe\n{b} *store/snag-chrome-1.0.2.zip\n");
        assert_eq!(checksum_for(&sums, "Snag-Setup-1.0.2.exe"), Some(a.clone()));
        assert_eq!(checksum_for(&sums, "snag-chrome-1.0.2.zip"), Some(b.to_ascii_lowercase()));
        assert_eq!(checksum_for(&sums, "Snag-Setup-1.0.1.exe"), None);
        assert_eq!(checksum_for(&format!("{a}  Snag-Setup-1.0.2.exe"), "Snag-Setup-1.0.2.exe"), Some(a), "sha256sum's text mode");
        assert_eq!(checksum_for("short *installer/Snag-Setup-1.0.2.exe", "Snag-Setup-1.0.2.exe"), None);
    }
}
