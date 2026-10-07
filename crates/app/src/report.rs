//! Report a bug: a text file written to the Desktop (and copied). Nothing is sent anywhere.
//! It never holds the pairing code, cookies, the user's name in paths, or link parameters.

use rdm_core::{Item, Settings, Status};
use std::path::{Path, PathBuf};

/// How much of `snag.log` goes in.
const LOG_LINES: usize = 50;

/// What "include diagnostics" adds.
pub struct Diagnostics<'a> {
    /// "Windows 11 24H2 (build 26200.6584)"
    pub windows: String,
    /// (tool, version or why there's none)
    pub tools: Vec<(&'static str, String)>,
    pub settings: &'a Settings,
    pub items: &'a [Item],
    /// The whole of `snag.log` (only its end is used).
    pub log: &'a str,
    /// %USERPROFILE%, written as that wherever it appears.
    pub home: Option<&'a str>,
}

/// Where bug reports are filed.
const ISSUES: &str = "https://github.com/kuldeepsaini23/snag/issues/new";
/// Longer links than this fail on GitHub (or in the browser); the report is pasted instead.
const ISSUE_URL_MAX: usize = 7000;

/// A new GitHub issue filled in with the report: the first line of `what` as its title. A report
/// too long for a link stays on the clipboard and the issue says to paste it.
pub fn issue_url(what: &str, report: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    let enc = |t: &str| utf8_percent_encode(t, NON_ALPHANUMERIC).to_string();
    let first = what.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Bug report");
    let title: String = first.chars().take(80).collect();
    let link = |body: &str| format!("{ISSUES}?labels=bug&title={}&body={}", enc(&title), enc(body));
    let full = link(report);
    if full.len() <= ISSUE_URL_MAX {
        return full;
    }
    let note = "

The full report from Snag is copied: press Ctrl+V here to paste it.";
    // As much of what the user typed as fits (non-English text takes up to 9 characters each).
    let mut budget = ISSUE_URL_MAX - link(note).len();
    let mut short = String::new();
    for ch in what.trim().chars() {
        let cost = enc(ch.encode_utf8(&mut [0; 4])).len();
        if cost > budget {
            break;
        }
        budget -= cost;
        short.push(ch);
    }
    link(&(short + note))
}

/// `Snag-bug-report-2026-10-07.txt`
pub fn file_name(date: chrono::NaiveDate) -> String {
    format!("Snag-bug-report-{}.txt", date.format("%Y-%m-%d"))
}

/// The report: `what` the user typed, `stamp` the local time it was written.
pub fn build(what: &str, stamp: &str, diagnostics: Option<&Diagnostics>) -> String {
    let what = what.trim();
    let mut out = format!(
        "Snag bug report · {stamp}\nSnag {}\n\nWhat happened\n-------------\n{}\n",
        crate::changelog::VERSION,
        if what.is_empty() { "(not filled in)" } else { what }
    );
    let Some(d) = diagnostics else { return out };
    out.push_str("\nDiagnostics\n-----------\n");
    out.push_str(&format!("{}\n", d.windows));
    for (tool, version) in &d.tools {
        out.push_str(&format!("{tool}: {version}\n"));
    }
    let count = |f: fn(&Status) -> bool| d.items.iter().filter(|i| f(&i.status)).count();
    out.push_str(&format!(
        "{} downloads: {} running, {} queued, {} paused, {} failed, {} finished\n",
        d.items.len(),
        count(|s| *s == Status::Running),
        count(|s| *s == Status::Queued),
        count(|s| *s == Status::Paused),
        count(|s| matches!(s, Status::Failed(_))),
        count(|s| *s == Status::Done),
    ));
    out.push_str("\nSettings\n");
    if let Ok(serde_json::Value::Object(map)) = serde_json::to_value(d.settings).map(redact) {
        for (key, value) in map {
            let shown = match value {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            out.push_str(&format!("  {key}: {shown}\n"));
        }
    }
    let lines: Vec<&str> = d.log.lines().filter(|l| !l.trim().is_empty()).collect();
    out.push_str(&format!("\nRecent log (last {LOG_LINES} lines of snag.log)\n"));
    if lines.is_empty() {
        out.push_str("  (empty)\n");
    }
    for line in &lines[lines.len().saturating_sub(LOG_LINES)..] {
        out.push_str(&format!("  {line}\n"));
    }
    scrub(&out, &d.settings.extension_token, d.home)
}

/// Drops every setting whose name says it's a secret (pairing token, cookies, passwords).
pub fn redact(value: serde_json::Value) -> serde_json::Value {
    const SECRET: [&str; 5] = ["token", "cookie", "password", "secret", "key"];
    match value {
        serde_json::Value::Object(map) => map
            .into_iter()
            .filter(|(k, _)| !SECRET.iter().any(|s| k.to_lowercase().contains(s)))
            .map(|(k, v)| (k, redact(v)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
        other => other,
    }
}

/// The pairing code wherever it appears, the user's home folder as `%USERPROFILE%` (any case),
/// and everything after `?` in links (signed links and sessions live there).
fn scrub(text: &str, token: &str, home: Option<&str>) -> String {
    let mut out = if token.len() >= 8 { text.replace(token, "<pairing code>") } else { text.to_string() };
    if let Some(home) = home.map(|h| h.trim_end_matches(['\\', '/'])).filter(|h| h.len() >= 4) {
        // ASCII case-folding keeps byte offsets the same in both strings.
        let lower_home = home.to_ascii_lowercase();
        let mut masked = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(at) = rest.to_ascii_lowercase().find(&lower_home) {
            masked.push_str(&rest[..at]);
            masked.push_str("%USERPROFILE%");
            rest = &rest[at + home.len()..];
        }
        masked.push_str(rest);
        out = masked;
    }
    out.split_inclusive(char::is_whitespace)
        .map(|word| {
            let link = word.find("http://").or_else(|| word.find("https://"));
            // Parameters and the #part both go: signed links, sessions and file keys live there.
            match link.and_then(|at| word[at..].find(['?', '#']).map(|q| at + q)) {
                Some(q) => format!("{}{}…{}", &word[..q], &word[q..q + 1], &word[word.trim_end().len()..]),
                None => word.to_string(),
            }
        })
        .collect()
}

/// Reads what the report needs from this PC, writes it into `dir` (the Desktop) and returns the
/// file and its text (for the clipboard).
pub async fn save(what: String, include: bool, settings: Settings, items: Vec<Item>, data_dir: PathBuf, dir: PathBuf) -> Result<(PathBuf, String), String> {
    let now = chrono::Local::now();
    let stamp = now.format("%Y-%m-%d %H:%M").to_string();
    let text = if include {
        let log = tokio::fs::read(data_dir.join("snag.log")).await.map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let bin = data_dir.join("bin");
        let tools = vec![("yt-dlp", tool_version(&bin.join("yt-dlp.exe")).await), ("gallery-dl", tool_version(&bin.join("gallery-dl.exe")).await)];
        let home = std::env::var("USERPROFILE").ok();
        let d = Diagnostics { windows: windows_version(), tools, settings: &settings, items: &items, log: &log, home: home.as_deref() };
        build(&what, &stamp, Some(&d))
    } else {
        build(&what, &stamp, None)
    };
    let file = dir.join(file_name(now.date_naive()));
    tokio::fs::write(&file, text.replace('\n', "\r\n")).await.map_err(|e| format!("Couldn't save the report: {e}"))?;
    Ok((file, text))
}

/// `yt-dlp --version`, without a console window and never waiting long.
async fn tool_version(exe: &Path) -> String {
    if !exe.exists() {
        return "not downloaded yet".into();
    }
    let mut cmd = tokio::process::Command::new(exe);
    cmd.arg("--version").kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    match tokio::time::timeout(std::time::Duration::from_secs(8), cmd.output()).await {
        Ok(Ok(out)) => String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_string(),
        Ok(Err(e)) => format!("couldn't run it ({e})"),
        Err(_) => "didn't answer in time".into(),
    }
}

/// "Windows 11 24H2 (build 26200.6584)", from the registry.
fn windows_version() -> String {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let Ok(key) = key else { return "Windows (version unknown)".into() };
    let build: String = key.get_value("CurrentBuild").unwrap_or_default();
    let display: String = key.get_value("DisplayVersion").unwrap_or_default();
    let ubr: Option<u32> = key.get_value("UBR").ok();
    // Windows 11 still calls itself Windows 10 in ProductName; the build number tells them apart.
    let name = if build.parse::<u32>().is_ok_and(|b| b >= 22000) { "Windows 11" } else { "Windows 10" };
    let build = match ubr {
        Some(u) => format!("{build}.{u}"),
        None => build,
    };
    format!("{name} {display} (build {build})").replace("  ", " ")
}

/// The user's Desktop (also when OneDrive moved it).
pub fn desktop() -> Option<PathBuf> {
    let folders = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders");
    let from_registry = folders.ok().and_then(|k| k.get_value::<String, _>("Desktop").ok()).map(PathBuf::from);
    from_registry.filter(|p| p.is_dir()).or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Desktop")).filter(|p| p.is_dir()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdm_core::{Category, ItemId, Kind};

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn item(id: u64, status: Status) -> Item {
        Item {
            id: ItemId(id),
            url: format!("https://x.example/{id}?sig=SECRETSIG"),
            name: format!("file {id}.zip"),
            category: Category::Archive,
            status,
            dest: Some(format!(r"C:\Users\alice\Downloads\Snag\file {id}.zip").into()),
            downloaded: 0,
            total: None,
            speed_bps: 0,
            queue: 0,
            added: 0,
            kind: Kind::Http,
            referrer: None,
            work_dir: None,
            thumbnail: None,
            duration: None,
            retry_at: None,
        }
    }

    fn settings() -> Settings {
        Settings { extension_token: TOKEN.into(), download_dir: r"C:\Users\alice\Downloads\Snag".into(), ..Settings::default() }
    }

    fn report(what: &str, log: &str, with: bool) -> String {
        let (s, items) = (settings(), [item(1, Status::Running), item(2, Status::Done), item(3, Status::Done), item(4, Status::Failed("HTTP 403".into()))]);
        let d = Diagnostics {
            windows: "Windows 11 24H2 (build 26200.6584)".into(),
            tools: vec![("yt-dlp", "2025.09.26".into()), ("gallery-dl", "not installed yet".into())],
            settings: &s,
            items: &items,
            log,
            home: Some(r"C:\Users\alice"),
        };
        build(what, "2026-10-07 14:02", with.then_some(&d))
    }

    #[test]
    fn a_github_issue_is_filled_in_with_the_report() {
        let url = issue_url("Video stuck at 99%
It was a YouTube live stream.", "REPORT & more
line 2");
        assert!(url.starts_with("https://github.com/kuldeepsaini23/snag/issues/new?"), "{url}");
        assert!(url.contains("title=Video%20stuck%20at%2099%25"), "first line as the title: {url}");
        assert!(url.contains("body=REPORT%20%26%20more%0Aline%202"), "the whole report, encoded: {url}");
        assert!(url.contains("labels=bug"));
        assert_eq!(issue_url("", "r"), issue_url("   ", "r"), "no text: a plain title");
        assert!(issue_url("", "r").contains("title=Bug%20report"));
    }

    #[test]
    fn a_long_report_is_pasted_instead_of_sent_in_the_link() {
        let report = "x".repeat(20_000);
        let url = issue_url(&"a very long first line ".repeat(20), &report);
        assert!(url.len() <= ISSUE_URL_MAX, "GitHub refuses very long links: {}", url.len());
        assert!(!url.contains("xxxxxxxxxx"), "the report itself stays on the clipboard");
        assert!(url.contains("Ctrl%2BV"), "says how to paste it");
        let hindi = issue_url(&"डाउनलोड रुक गया ".repeat(400), &report);
        assert!(hindi.len() <= ISSUE_URL_MAX && hindi.contains("Ctrl%2BV"), "any language fits: {}", hindi.len());
    }

    #[test]
    fn bug_report_has_no_secrets() {
        let log = format!(
            "2026-10-07 13:00:00  Phone link http://192.168.1.5:47330/{TOKEN}\n\
             2026-10-07 13:01:00  clip.mp4 failed: HTTP 403 for https://rr3.googlevideo.com/videoplayback?expire=1&sig=SECRETSIG\n\
             2026-10-07 13:02:00  Couldn't write C:\\Users\\alice\\Videos\\clip.mp4\n\
             2026-10-07 13:03:00  c:\\users\\ALICE\\AppData\\Roaming\\Snag\\cookies\\youtube.txt is gone\n\
             2026-10-07 13:04:00  share.zip failed: https://mega.nz/file/AbC#DECRYPTKEY\n"
        );
        let text = report("It froze. My code is pasted nowhere.", &log, true);
        assert!(!text.contains(TOKEN), "pairing code");
        assert!(!text.contains("extension_token"), "not even the setting's name");
        assert!(!text.contains("SECRETSIG"), "link parameters (signed links, sessions)");
        assert!(!text.contains("DECRYPTKEY"), "a link's #part (file keys live there)");
        assert!(!text.to_lowercase().contains("alice"), "the user's name in paths");
        assert!(text.contains(r"%USERPROFILE%\Downloads\Snag"), "folder names stay readable");
        assert!(text.contains("https://rr3.googlevideo.com/videoplayback?…"), "the link minus its parameters");
        // Settings added later that hold secrets are dropped by name.
        let value = serde_json::json!({ "accent": "#fff", "browser_cookies": "SID=1", "virustotal_key": "vt", "nested": { "api_token": "x", "password": "y", "keep": 1, "keep_sharing": true } });
        assert_eq!(redact(value), serde_json::json!({ "accent": "#fff", "nested": { "keep": 1, "keep_sharing": true } }));
    }

    #[tokio::test]
    async fn save_writes_the_report_file() {
        let (data, desk) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        std::fs::write(data.path().join("snag.log"), format!("2026-10-07 13:00:00  clip.mp4 failed: HTTP 403 ({TOKEN})\n")).unwrap();
        let (file, text) = save("Stuck at 0 B".into(), true, settings(), vec![item(1, Status::Paused)], data.path().into(), desk.path().into()).await.expect("written");
        assert_eq!(file.parent(), Some(desk.path()));
        assert!(file.file_name().unwrap().to_string_lossy().starts_with("Snag-bug-report-"));
        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert_eq!(on_disk.replace("\r\n", "\n"), text, "the file is what gets copied (Notepad line ends)");
        for want in ["Stuck at 0 B", "yt-dlp: not downloaded yet", "clip.mp4 failed: HTTP 403", "1 paused"] {
            assert!(text.contains(want), "missing {want:?} in:\n{text}");
        }
        assert!(!text.contains(TOKEN));
    }

    #[test]
    fn bug_report_contents() {
        let log: String = (1..=60).map(|n| format!("2026-10-07 13:00:00  entry {n:03}\n")).collect();
        let text = report("The Download button did nothing.", &log, true);
        for want in [
            "The Download button did nothing.",
            "2026-10-07 14:02",
            &format!("Snag {}", env!("CARGO_PKG_VERSION")),
            "Windows 11 24H2 (build 26200.6584)",
            "yt-dlp: 2025.09.26",
            "gallery-dl: not installed yet",
            "4 downloads: 1 running, 0 queued, 0 paused, 1 failed, 2 finished",
            "connections: 8",
            "entry 060",
            "entry 011",
        ] {
            assert!(text.contains(want), "missing {want:?} in:\n{text}");
        }
        assert!(!text.contains("entry 010"), "only the last 50 log lines");
        assert!(!text.contains("file 2.zip"), "no list of what was downloaded");
        // Without diagnostics: just the words and the version.
        let bare = report("", &log, false);
        assert!(bare.contains("(not filled in)"));
        assert!(bare.contains(&format!("Snag {}", env!("CARGO_PKG_VERSION"))));
        assert!(!bare.contains("Windows") && !bare.contains("entry") && !bare.contains("connections"), "{bare}");
        assert_eq!(file_name(chrono::NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()), "Snag-bug-report-2026-10-07.txt");
    }
}
