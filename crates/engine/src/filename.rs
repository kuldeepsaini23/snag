use percent_encoding::percent_decode_str;
use crate::state::{DownloadState, part_path, state_path};
use std::path::{Path, PathBuf};

pub fn filename_from(content_disposition: Option<&str>, url: &str) -> String {
    let raw = content_disposition
        .and_then(from_content_disposition)
        .or_else(|| from_url(url))
        .unwrap_or_default();
    sanitize(&raw)
}

fn from_content_disposition(cd: &str) -> Option<String> {
    let mut plain = None;
    for part in cd.split(';').map(str::trim) {
        let Some((key, value)) = part.split_once('=') else { continue };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if key == "filename*" {
            // RFC 5987: charset'language'percent-encoded-value
            let encoded = value.splitn(3, '\'').nth(2).unwrap_or(value);
            return Some(percent_decode_str(encoded).decode_utf8_lossy().into_owned());
        }
        if key == "filename" {
            plain = Some(value.trim_matches('"').to_string());
        }
    }
    plain
}

fn from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let after_scheme = path.split_once("://").map_or(path, |(_, rest)| rest);
    let (_, path_part) = after_scheme.split_once('/')?;
    let last = path_part.rsplit('/').next()?;
    if last.is_empty() {
        return None;
    }
    Some(percent_decode_str(last).decode_utf8_lossy().into_owned())
}

pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() { "download".to_string() } else { trimmed.to_string() }
}

/// First free name: `name`, `name (1)`, … A name is taken if the file exists or an
/// unfinished download (`<name>.rdmpart`) is using it.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    candidates(dir, name)
        .find(|p| !p.exists() && !part_path(p).exists())
        .expect("unbounded counter always finds a free name")
}

/// Where to save `name` from `url` in `dir`: an unfinished download of the same URL
/// (`<cand>.rdmpart` + sidecar with that URL) is resumed, otherwise a free name is picked.
pub fn resume_target(dir: &Path, name: &str, url: &str) -> PathBuf {
    candidates(dir, name)
        .find(|p| {
            if part_path(p).exists() {
                sidecar_url(p).as_deref() == Some(url)
            } else {
                !p.exists()
            }
        })
        .expect("unbounded counter always finds a usable name")
}

fn candidates(dir: &Path, name: &str) -> impl Iterator<Item = PathBuf> {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    let dir = dir.to_path_buf();
    std::iter::once(dir.join(name)).chain((1..).map(move |i| dir.join(format!("{stem} ({i}){ext}"))))
}

fn sidecar_url(dest: &Path) -> Option<String> {
    let bytes = std::fs::read(state_path(dest)).ok()?;
    serde_json::from_slice::<DownloadState>(&bytes).ok().map(|s| s.url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unfinished(dir: &Path, file: &str, url: &str) {
        let dest = dir.join(file);
        std::fs::write(crate::state::part_path(&dest), b"partial").unwrap();
        let state = crate::state::DownloadState { url: url.into(), size: 7, etag: None, last_modified: None, segments: vec![] };
        std::fs::write(crate::state::state_path(&dest), serde_json::to_vec(&state).unwrap()).unwrap();
    }

    #[test]
    fn unique_path_skips_unfinished_part() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt.rdmpart"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a (1).txt"));
    }

    #[test]
    fn resume_target_reuses_unfinished_download_of_same_url() {
        let dir = tempfile::tempdir().unwrap();
        unfinished(dir.path(), "a.bin", "http://x/other");
        unfinished(dir.path(), "a (1).bin", "http://x/mine");
        assert_eq!(resume_target(dir.path(), "a.bin", "http://x/mine"), dir.path().join("a (1).bin"));
    }

    #[test]
    fn resume_target_never_reuses_another_urls_part() {
        let dir = tempfile::tempdir().unwrap();
        unfinished(dir.path(), "a.bin", "http://x/other");
        assert_eq!(resume_target(dir.path(), "a.bin", "http://x/mine"), dir.path().join("a (1).bin"));
        assert_eq!(resume_target(dir.path(), "b.bin", "http://x/mine"), dir.path().join("b.bin"));
    }

    #[test]
    fn prefers_rfc5987_filename() {
        let cd = r#"attachment; filename="fallback.zip"; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"#;
        assert_eq!(filename_from(Some(cd), "https://x.com/a"), "résumé.pdf");
    }

    #[test]
    fn uses_quoted_filename() {
        let cd = r#"attachment; filename="report 2026.xlsx""#;
        assert_eq!(filename_from(Some(cd), "https://x.com/a"), "report 2026.xlsx");
    }

    #[test]
    fn falls_back_to_url_path() {
        assert_eq!(
            filename_from(None, "https://cdn.x.com/files/ubuntu%2024.iso?token=abc#frag"),
            "ubuntu 24.iso"
        );
    }

    #[test]
    fn falls_back_to_download_when_nothing() {
        assert_eq!(filename_from(None, "https://x.com/"), "download");
        assert_eq!(filename_from(None, "https://x.com"), "download");
    }

    #[test]
    fn replaces_windows_illegal_chars() {
        let cd = r#"attachment; filename="a<b>:c|d?.mp4""#;
        assert_eq!(filename_from(Some(cd), "https://x.com"), "a_b__c_d_.mp4");
        assert_eq!(sanitize("..\\..\\evil.exe"), ".._.._evil.exe");
    }

    #[test]
    fn strips_trailing_dots_and_spaces() {
        assert_eq!(sanitize("video. . "), "video");
        assert_eq!(sanitize("   "), "download");
    }

    #[test]
    fn unique_path_appends_counter() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a.txt"));
        std::fs::write(dir.path().join("a.txt"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a (1).txt"));
        std::fs::write(dir.path().join("a (1).txt"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a (2).txt"));
        std::fs::write(dir.path().join("README"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "README"), dir.path().join("README (1)"));
    }
}
