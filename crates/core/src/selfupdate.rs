//! Snag's own updates: the newest GitHub release, its installer and the checksum it must match.

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
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        return None;
    }
    let version = v["tag_name"].as_str()?.trim_start_matches('v').to_string();
    let assets = v["assets"].as_array()?;
    let asset = |name: &str| assets.iter().find(|a| a["name"].as_str() == Some(name)).and_then(|a| a["browser_download_url"].as_str()).map(String::from);
    let installer_name = format!("Snag-Setup-{version}.exe");
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
        let r = parse_latest(LATEST).expect("a release");
        assert_eq!(r.version, "1.0.2");
        assert_eq!(r.installer, "https://x.test/Snag-Setup-1.0.2.exe", "the versioned file: it's the one the checksums list");
        assert_eq!(r.installer_name, "Snag-Setup-1.0.2.exe");
        assert_eq!(r.sums, "https://x.test/SHA256SUMS-1.0.2.txt");
        assert_eq!(r.page, "https://github.com/kuldeepsaini23/snag/releases/tag/v1.0.2");
    }

    #[test]
    fn skips_what_shouldnt_be_installed() {
        assert_eq!(parse_latest(&LATEST.replace(r#""prerelease": false"#, r#""prerelease": true"#)), None, "pre-release");
        assert_eq!(parse_latest(&LATEST.replace(r#""draft": false"#, r#""draft": true"#)), None, "draft");
        assert_eq!(parse_latest(&LATEST.replace("SHA256SUMS-1.0.2.txt", "notes.txt")), None, "no checksums: never installed unchecked");
        assert_eq!(parse_latest(&LATEST.replace("Snag-Setup-1.0.2.exe", "Other.exe")), None, "no installer");
        assert_eq!(parse_latest("{\"message\": \"Not Found\"}"), None);
        assert_eq!(parse_latest("not json"), None);
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
