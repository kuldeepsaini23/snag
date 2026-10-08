//! Image pages (Pinterest, Imgur, Instagram photo posts, Reddit galleries, …) through gallery-dl.

use crate::{MediaOptions, MediaOutcome, MediaProgress, command};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// gallery-dl's official standalone builds (its main repo no longer attaches the exe).
#[cfg(windows)]
pub const GALLERY_DL_URL: &str = "https://github.com/gdl-org/builds/releases/latest/download/gallery-dl_windows.exe";
#[cfg(not(windows))]
pub const GALLERY_DL_URL: &str = "https://github.com/gdl-org/builds/releases/latest/download/gallery-dl_linux";

/// Sites that are mostly images: their links go to gallery-dl rather than yt-dlp or the file engine.
const HOSTS: [&str; 13] = [
    "pinterest.com", "pin.it", "imgur.com", "deviantart.com", "artstation.com", "flickr.com", "tumblr.com",
    "pixiv.net", "behance.net", "unsplash.com", "danbooru.donmai.us", "gelbooru.com", "wallhaven.cc",
];

fn host_and_path(url: &str) -> Option<(String, &str)> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..end].split(':').next().unwrap_or("").to_ascii_lowercase();
    Some((host, &rest[end..]))
}

fn on(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// A page of images gallery-dl can save (also Reddit's `/gallery/` posts).
pub fn is_gallery_url(url: &str) -> bool {
    let Some((host, path)) = host_and_path(url) else { return false };
    HOSTS.iter().any(|d| on(&host, d)) || (on(&host, "reddit.com") && path.starts_with("/gallery/"))
}

/// yt-dlp said the post has no video, on a site gallery-dl also knows: it's a photo post.
pub fn is_photo_post(url: &str, ytdlp_error: &str) -> bool {
    let Some((host, _)) = host_and_path(url) else { return false };
    let photo_sites = ["instagram.com", "x.com", "twitter.com", "tumblr.com", "reddit.com", "facebook.com", "threads.net"];
    photo_sites.iter().any(|d| on(&host, d)) && ytdlp_error.to_ascii_lowercase().contains("no video")
}

pub fn args(url: &str, dir: &Path, opts: &MediaOptions) -> Vec<String> {
    let mut args = vec!["-D".to_string(), dir.display().to_string()];
    if let Some(cookies) = &opts.cookies {
        args.extend(["--cookies".to_string(), cookies.display().to_string()]);
    }
    if opts.limit_bps > 0 {
        args.extend(["--limit-rate".to_string(), opts.limit_bps.to_string()]);
    }
    // `--` ends the options: a link can never be read as one (`--exec`, `--config-locations`).
    args.extend(["--".to_string(), url.to_string()]);
    args
}

/// One stdout line is one image: its path, or "# path" when it was already there.
pub fn file_from_line(line: &str) -> Option<PathBuf> {
    let path = line.strip_prefix("# ").unwrap_or(line).trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// The last `[error]` line of gallery-dl's stderr, else its last line.
fn error_from(stderr: &str) -> Option<String> {
    let lines: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let line = lines.iter().rev().find(|l| l.contains("[error]")).or(lines.last())?;
    Some(line.rsplit_once("] ").map_or(*line, |(_, m)| m).trim().to_string())
}

/// Saves every image on the page into `dir` (existing ones are skipped, so a paused gallery
/// continues where it stopped). Progress is the bytes of the images saved so far.
pub async fn download(
    exe: &Path,
    url: &str,
    dir: &Path,
    opts: &MediaOptions,
    cancel: CancellationToken,
    progress: &watch::Sender<MediaProgress>,
) -> Result<MediaOutcome, String> {
    use std::process::Stdio;

    tokio::fs::create_dir_all(dir).await.map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    let mut child = command(exe)
        .args(args(url, dir, opts))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("can't start gallery-dl: {e}"))?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let stderr_task = tokio::spawn(async move { crate::read_all_lossy(&mut stderr).await });
    let mut lines = crate::ToolLines::new(child.stdout.take().expect("stdout is piped"));
    let (started, mut bytes) = (Instant::now(), 0u64);
    // A page can list the same image twice; count each file once.
    let mut seen = std::collections::HashSet::new();
    loop {
        let line = tokio::select! {
            _ = cancel.cancelled() => {
                crate::kill(&mut child).await;
                return Ok(MediaOutcome::Paused);
            }
            line = lines.next_line() => line.map_err(|e| e.to_string())?,
        };
        let Some(line) = line else { break };
        if let Some(file) = file_from_line(&line).filter(|f| seen.insert(f.clone())) {
            bytes += std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            let secs = started.elapsed().as_secs_f64().max(0.5);
            progress.send_replace(MediaProgress { downloaded: bytes, total: None, speed_bps: (bytes as f64 / secs) as u64 });
        }
    }
    let status = tokio::select! {
        _ = cancel.cancelled() => {
            crate::kill(&mut child).await;
            return Ok(MediaOutcome::Paused);
        }
        status = child.wait() => status.map_err(|e| e.to_string())?,
    };
    let stderr = stderr_task.await.unwrap_or_default();
    if !status.success() {
        return Err(error_from(&stderr).unwrap_or_else(|| format!("gallery-dl failed ({status})")));
    }
    if seen.is_empty() {
        return Err("No images found on that page".into());
    }
    Ok(MediaOutcome::Completed(dir.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_urls_detected() {
        for url in [
            "https://www.pinterest.com/pin/123/",
            "https://pin.it/abc",
            "https://imgur.com/gallery/xyz",
            "https://i.imgur.com/a.png",
            "https://www.deviantart.com/someone/art/x",
            "https://www.reddit.com/gallery/abc",
            "https://wallhaven.cc/w/1",
        ] {
            assert!(is_gallery_url(url), "{url}");
        }
        for url in ["https://www.youtube.com/watch?v=1", "https://www.reddit.com/r/x/comments/1", "https://example.com/a.jpg", "ftp://imgur.com/x", "https://notimgur.com/x"] {
            assert!(!is_gallery_url(url), "{url}");
        }
    }

    #[test]
    fn photo_posts_fall_back_to_gallery() {
        assert!(is_photo_post("https://www.instagram.com/p/abc/", "[Instagram] abc: There is no video in this post"));
        assert!(is_photo_post("https://x.com/a/status/1", "No video could be found in this tweet"));
        assert!(!is_photo_post("https://www.instagram.com/p/abc/", "HTTP Error 429: Too Many Requests"), "other errors stay errors");
        assert!(!is_photo_post("https://www.youtube.com/watch?v=1", "no video"), "not an image site");
    }

    #[test]
    fn args_carry_dir_cookies_and_limit() {
        let opts = MediaOptions { cookies: Some(PathBuf::from(r"C:\c.txt")), limit_bps: 2048, ..Default::default() };
        let a = args("https://imgur.com/a/x", Path::new(r"C:\dl\Images\x"), &opts);
        assert_eq!(a, vec!["-D", r"C:\dl\Images\x", "--cookies", r"C:\c.txt", "--limit-rate", "2048", "--", "https://imgur.com/a/x"]);
        assert_eq!(args("u", Path::new("d"), &MediaOptions::default()), vec!["-D", "d", "--", "u"]);
    }

    #[test]
    fn output_lines_are_files() {
        assert_eq!(file_from_line(r"C:\dl\1.jpg"), Some(PathBuf::from(r"C:\dl\1.jpg")));
        assert_eq!(file_from_line(r"# C:\dl\0.jpg"), Some(PathBuf::from(r"C:\dl\0.jpg")));
        assert_eq!(file_from_line("  "), None);
        assert_eq!(error_from("[gallery-dl][info] x\n[gallery-dl][error] HttpError: 404\n"), Some("HttpError: 404".into()));
    }
}
