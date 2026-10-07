//! Safety check for downloaded programs: the file's SHA-256 is looked up on VirusTotal with the
//! user's own free API key. Only the hash leaves the PC, never the file.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// What VirusTotal knows about a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Safety {
    /// No scanner flagged it.
    Clean { scanners: u32 },
    /// `bad` scanners call it malicious or suspicious.
    Flagged { bad: u32, scanners: u32 },
    /// VirusTotal has never seen this file.
    Unknown,
    /// The check couldn't be done (no network, wrong key, daily quota used up…).
    Failed(String),
}

/// Kinds of files worth checking: programs, scripts and archives that may hold them.
const CHECKED: [&str; 12] = ["exe", "msi", "bat", "cmd", "ps1", "scr", "com", "vbs", "js", "jar", "zip", "7z"];

pub fn worth_checking(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| CHECKED.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Reads VirusTotal's answer (`GET /api/v3/files/<sha256>`).
pub fn parse_report(status: u16, body: &str) -> Safety {
    match status {
        404 => return Safety::Unknown,
        401 | 403 => return Safety::Failed("VirusTotal refused the API key (Settings → Tools)".into()),
        429 => return Safety::Failed("VirusTotal's daily limit for your key is used up; try again tomorrow".into()),
        200 => {}
        other => return Safety::Failed(format!("VirusTotal answered {other}")),
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else { return Safety::Failed("couldn't read VirusTotal's answer".into()) };
    let stats = &v["data"]["attributes"]["last_analysis_stats"];
    let n = |k: &str| stats[k].as_u64().unwrap_or(0) as u32;
    if !stats.is_object() {
        return Safety::Failed("couldn't read VirusTotal's answer".into());
    }
    // Scanners that actually looked at it (not timeouts or unsupported types).
    let scanners = n("malicious") + n("suspicious") + n("harmless") + n("undetected");
    match n("malicious") + n("suspicious") {
        0 => Safety::Clean { scanners },
        bad => Safety::Flagged { bad, scanners },
    }
}

/// Where VirusTotal is (tests point it at a local server with `SNAG_VIRUSTOTAL_URL`).
fn api_base() -> String {
    const VIRUSTOTAL: &str = "https://www.virustotal.com";
    // Release builds always use VirusTotal: an environment variable must not be able to send the
    // user's key elsewhere.
    if !cfg!(debug_assertions) {
        return VIRUSTOTAL.into();
    }
    std::env::var("SNAG_VIRUSTOTAL_URL").unwrap_or_else(|_| VIRUSTOTAL.into())
}

/// Hashes `path` and asks VirusTotal about it. `None` without a key (nothing is sent).
pub async fn check(path: &Path, key: &str) -> Option<Safety> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let file = path.to_path_buf();
    let hash = match tokio::task::spawn_blocking(move || sha256_file(&file)).await {
        Ok(Ok(hash)) => hash,
        _ => return Some(Safety::Failed("couldn't read the file".into())),
    };
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build().ok()?;
    let answer = client.get(format!("{}/api/v3/files/{hash}", api_base())).header("x-apikey", key).send().await;
    Some(match answer {
        Ok(resp) => {
            let status = resp.status().as_u16();
            parse_report(status, &resp.text().await.unwrap_or_default())
        }
        Err(e) => Safety::Failed(format!("couldn't reach VirusTotal: {e}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_and_archives_are_checked_media_is_not() {
        for name in ["setup.exe", "Setup.EXE", "tool.msi", "run.ps1", "pack.zip", "a.7z"] {
            assert!(worth_checking(Path::new(name)), "{name}");
        }
        for name in ["movie.mp4", "song.mp3", "photo.jpg", "readme", "doc.pdf"] {
            assert!(!worth_checking(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn hashes_the_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.exe");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(sha256_file(&file).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn reads_what_virustotal_says() {
        let stats = |m: u32, s: u32, h: u32, u: u32| {
            format!(r#"{{"data":{{"attributes":{{"last_analysis_stats":{{"malicious":{m},"suspicious":{s},"harmless":{h},"undetected":{u},"timeout":0,"type-unsupported":3}}}}}}}}"#)
        };
        assert_eq!(parse_report(200, &stats(0, 0, 10, 62)), Safety::Clean { scanners: 72 });
        assert_eq!(parse_report(200, &stats(4, 1, 10, 57)), Safety::Flagged { bad: 5, scanners: 72 });
        assert_eq!(parse_report(404, r#"{"error":{"code":"NotFoundError"}}"#), Safety::Unknown);
        assert!(matches!(parse_report(429, r#"{"error":{"code":"QuotaExceededError"}}"#), Safety::Failed(e) if e.contains("limit")));
        assert!(matches!(parse_report(401, r#"{"error":{"code":"WrongCredentialsError"}}"#), Safety::Failed(e) if e.contains("API key")));
        assert!(matches!(parse_report(200, "not json"), Safety::Failed(_)));
    }
}
