use percent_encoding::percent_decode_str;
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

pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (1..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .expect("unbounded counter always finds a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

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
