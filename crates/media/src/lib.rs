//! Video and audio downloads through yt-dlp (+ ffmpeg for merging and MP3).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaFormat {
    /// Best video at or below this height, merged with best audio into MP4.
    Video { max_height: u32 },
    /// Best audio converted to MP3.
    AudioMp3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualityOption {
    pub label: String,
    pub format: MediaFormat,
    pub approx_size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub url: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaInfo {
    pub title: String,
    pub duration: Option<f64>,
    pub options: Vec<QualityOption>,
    /// Non-empty for playlists and channels.
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: u64,
}

/// Turns per-stream progress (video, then audio: each restarts at 0) into one running total.
#[derive(Debug, Default)]
pub struct StreamProgress {
    /// Bytes of streams already finished.
    done: u64,
    last: Option<MediaProgress>,
}

impl StreamProgress {
    pub fn push(&mut self, p: MediaProgress) -> MediaProgress {
        if let Some(last) = &self.last
            && p.downloaded < last.downloaded
        {
            // The counter went back: the previous stream finished and a new one began.
            self.done += last.total.unwrap_or(last.downloaded);
        }
        self.last = Some(p.clone());
        MediaProgress { downloaded: self.done + p.downloaded, total: p.total.map(|t| self.done + t), speed_bps: p.speed_bps }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MediaOutcome {
    Completed(PathBuf),
    Paused,
}

/// Sites that need yt-dlp rather than a plain file download.
pub fn is_media_url(url: &str) -> bool {
    const HOSTS: [&str; 14] = [
        "youtube.com", "youtu.be", "vimeo.com", "x.com", "twitter.com", "instagram.com", "tiktok.com",
        "facebook.com", "fb.watch", "dailymotion.com", "twitch.tv", "soundcloud.com", "reddit.com", "bilibili.com",
    ];
    let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) else { return false };
    let host = rest.split(['/', '?', '#', ':']).next().unwrap_or("").to_ascii_lowercase();
    HOSTS.iter().any(|h| host == *h || host.ends_with(&format!(".{h}")))
}

/// Parses `yt-dlp -J --flat-playlist` output.
pub fn parse_probe(json: &str) -> Result<MediaInfo, String> {
    use serde_json::Value;
    let v: Value = serde_json::from_str(json).map_err(|e| format!("couldn't read yt-dlp output: {e}"))?;
    let title = v["title"].as_str().unwrap_or("Untitled").to_string();
    let duration = v["duration"].as_f64();
    let size = |f: &Value| f["filesize"].as_u64().or_else(|| f["filesize_approx"].as_u64());
    let audio_mp3 = |approx_size| QualityOption { label: "Audio only (MP3)".into(), format: MediaFormat::AudioMp3, approx_size };

    if v["_type"] == "playlist" {
        let entries = v["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let url = e["url"].as_str().or_else(|| e["webpage_url"].as_str())?;
                Some(Entry { url: url.to_string(), title: e["title"].as_str().unwrap_or(url).to_string() })
            })
            .collect();
        // A flat playlist doesn't list formats: offer the usual choices.
        let mut options: Vec<_> = [1080, 720, 480]
            .into_iter()
            .map(|h| QualityOption { label: format!("{h}p"), format: MediaFormat::Video { max_height: h }, approx_size: None })
            .collect();
        options.push(audio_mp3(None));
        return Ok(MediaInfo { title, duration, options, entries });
    }

    let formats = v["formats"].as_array().cloned().unwrap_or_default();
    let is_video = |f: &Value| f["vcodec"].as_str().is_some_and(|c| c != "none") && f["height"].as_u64().is_some_and(|h| h >= 144);
    let is_audio = |f: &Value| f["vcodec"] == "none" && f["acodec"].as_str().is_some_and(|c| c != "none");
    let best_audio = formats.iter().filter(|f| is_audio(f)).filter_map(size).max();
    let mut heights: Vec<u64> = formats.iter().filter(|f| is_video(f)).filter_map(|f| f["height"].as_u64()).collect();
    heights.sort_unstable_by(|a, b| b.cmp(a));
    heights.dedup();
    let mut options: Vec<_> = heights
        .into_iter()
        .map(|h| {
            let video = formats.iter().filter(|f| is_video(f) && f["height"].as_u64() == Some(h)).filter_map(size).max();
            QualityOption {
                label: format!("{h}p"),
                format: MediaFormat::Video { max_height: h as u32 },
                approx_size: video.map(|v| v + best_audio.unwrap_or(0)),
            }
        })
        .collect();
    options.push(audio_mp3(best_audio));
    Ok(MediaInfo { title, duration, options, entries: Vec::new() })
}

pub fn build_args(format: &MediaFormat, out_dir: &Path, url: &str) -> Vec<String> {
    let mut args: Vec<String> = [
        "--newline",
        "--encoding",
        "utf-8",
        "--no-warnings",
        "--no-playlist",
        "--progress",
        "--progress-template",
        "download:RDMP %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s %(progress.speed)s",
        "--print",
        "after_move:RDMF %(filepath)s",
    ]
    .map(String::from)
    .to_vec();
    match format {
        MediaFormat::Video { max_height: h } => {
            args.extend(["-f".into(), format!("bv*[height<={h}]+ba/b[height<={h}]"), "--merge-output-format".into(), "mp4".into()]);
        }
        MediaFormat::AudioMp3 => {
            args.extend(["-f", "ba/b", "-x", "--audio-format", "mp3", "--audio-quality", "0"].map(String::from));
        }
    }
    args.push("-o".into());
    args.push(out_dir.join("%(title)s.%(ext)s").display().to_string());
    args.push(url.to_string());
    args
}

/// Parses our progress template: `RDMP <downloaded> <total> <estimate> <speed>` (values may be `NA`).
pub fn parse_progress_line(line: &str) -> Option<MediaProgress> {
    let mut parts = line.strip_prefix("RDMP ")?.split_whitespace();
    let num = |s: Option<&str>| s.and_then(|v| v.parse::<f64>().ok()).map(|v| v as u64);
    let downloaded = num(parts.next())?;
    let total = num(parts.next());
    let estimate = num(parts.next());
    let speed_bps = num(parts.next()).unwrap_or(0);
    Some(MediaProgress { downloaded, total: total.or(estimate), speed_bps })
}

fn command(program: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing up
    // Report titles and paths in UTF-8, not the console code page (which drops emoji).
    cmd.env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
    cmd.stdin(std::process::Stdio::null()).kill_on_drop(true);
    cmd
}

/// The most useful line of yt-dlp's stderr: the last `ERROR:` line, else the last line.
fn error_from(stderr: &str) -> Option<String> {
    let lines: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let line = lines.iter().rev().find(|l| l.starts_with("ERROR:")).or(lines.last())?;
    Some(line.trim_start_matches("ERROR:").trim().to_string())
}

pub async fn probe(ytdlp: &Path, url: &str) -> Result<MediaInfo, String> {
    let out = command(ytdlp)
        .args(["-J", "--flat-playlist", "--no-warnings", url])
        .output()
        .await
        .map_err(|e| format!("can't start yt-dlp: {e}"))?;
    if !out.status.success() {
        return Err(error_from(&String::from_utf8_lossy(&out.stderr)).unwrap_or_else(|| format!("yt-dlp failed ({})", out.status)));
    }
    parse_probe(&String::from_utf8_lossy(&out.stdout))
}

/// Runs yt-dlp until it finishes or `cancel` fires (the process is killed; yt-dlp
/// continues its `.part` files next time).
pub async fn download(
    ytdlp: &Path,
    url: &str,
    format: &MediaFormat,
    out_dir: &Path,
    cancel: CancellationToken,
    progress: &watch::Sender<MediaProgress>,
) -> Result<MediaOutcome, String> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

    tokio::fs::create_dir_all(out_dir).await.map_err(|e| format!("can't create {}: {e}", out_dir.display()))?;
    let mut child = command(ytdlp)
        .args(build_args(format, out_dir, url))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("can't start yt-dlp: {e}"))?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let stderr_task = tokio::spawn(async move {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text).await;
        text
    });
    let mut lines = BufReader::new(child.stdout.take().expect("stdout is piped")).lines();
    let mut final_path = None;
    let mut streams = StreamProgress::default();
    loop {
        let line = tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                return Ok(MediaOutcome::Paused);
            }
            line = lines.next_line() => line.map_err(|e| e.to_string())?,
        };
        let Some(line) = line else { break };
        if let Some(p) = parse_progress_line(&line) {
            progress.send_replace(streams.push(p));
        } else if let Some(path) = line.strip_prefix("RDMF ") {
            final_path = Some(PathBuf::from(path.trim()));
        }
    }
    let status = tokio::select! {
        _ = cancel.cancelled() => {
            let _ = child.kill().await;
            return Ok(MediaOutcome::Paused);
        }
        status = child.wait() => status.map_err(|e| e.to_string())?,
    };
    let stderr = stderr_task.await.unwrap_or_default();
    if !status.success() {
        return Err(error_from(&stderr).unwrap_or_else(|| format!("yt-dlp failed ({status})")));
    }
    final_path.map(MediaOutcome::Completed).ok_or_else(|| "yt-dlp finished without reporting the file".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_media_hosts() {
        for url in [
            "https://www.youtube.com/watch?v=abc",
            "https://m.youtube.com/watch?v=abc",
            "https://youtu.be/abc",
            "https://vimeo.com/123",
            "https://x.com/user/status/1",
            "https://www.instagram.com/reel/xyz/",
            "https://www.tiktok.com/@a/video/1",
        ] {
            assert!(is_media_url(url), "{url}");
        }
        for url in ["https://example.com/file.zip", "https://notyoutube.com/watch", "ftp://youtube.com", "youtube"] {
            assert!(!is_media_url(url), "{url}");
        }
    }

    const SINGLE: &str = r#"{"_type":"video","title":"Test clip","duration":10.5,"formats":[
        {"format_id":"a1","vcodec":"none","acodec":"mp4a","filesize":1000},
        {"format_id":"a2","vcodec":"none","acodec":"opus","filesize_approx":1200},
        {"format_id":"v720a","height":720,"vcodec":"avc1","acodec":"none","filesize":5000},
        {"format_id":"v720b","height":720,"vcodec":"vp9","acodec":"none","filesize_approx":4000},
        {"format_id":"v1080","height":1080,"vcodec":"avc1","acodec":"none","filesize":9000},
        {"format_id":"muxed","height":360,"vcodec":"avc1","acodec":"mp4a","filesize":2000},
        {"format_id":"sb","height":90,"vcodec":"none","acodec":"none"}]}"#;

    #[test]
    fn parses_single_video() {
        let info = parse_probe(SINGLE).unwrap();
        assert_eq!(info.title, "Test clip");
        assert_eq!(info.duration, Some(10.5));
        assert!(info.entries.is_empty());
        let got: Vec<_> = info.options.iter().map(|o| (o.label.as_str(), o.approx_size)).collect();
        assert_eq!(got, vec![("1080p", Some(10200)), ("720p", Some(6200)), ("360p", Some(3200)), ("Audio only (MP3)", Some(1200))]);
        assert_eq!(info.options[0].format, MediaFormat::Video { max_height: 1080 });
        assert_eq!(info.options[3].format, MediaFormat::AudioMp3);
    }

    #[test]
    fn parses_playlist() {
        let json = r#"{"_type":"playlist","title":"My list","entries":[
            {"url":"https://www.youtube.com/watch?v=a1","title":"One"},
            {"url":"https://www.youtube.com/watch?v=b2","title":"Two"},
            {"id":"c3","title":"Three","webpage_url":"https://www.youtube.com/watch?v=c3"}]}"#;
        let info = parse_probe(json).unwrap();
        assert_eq!(info.title, "My list");
        assert_eq!(info.entries.len(), 3);
        assert_eq!(info.entries[2], Entry { url: "https://www.youtube.com/watch?v=c3".into(), title: "Three".into() });
        let labels: Vec<_> = info.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["1080p", "720p", "480p", "Audio only (MP3)"]);
    }

    #[test]
    fn audio_only_source_offers_mp3_only() {
        let json = r#"{"title":"Song","formats":[{"vcodec":"none","acodec":"opus","filesize":700}]}"#;
        let info = parse_probe(json).unwrap();
        assert_eq!(info.options.len(), 1);
        assert_eq!(info.options[0].format, MediaFormat::AudioMp3);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_probe("not json").is_err());
    }

    fn window(args: &[String], pair: [&str; 2]) -> bool {
        args.windows(2).any(|w| w[0] == pair[0] && w[1] == pair[1])
    }

    #[test]
    fn video_args() {
        let args = build_args(&MediaFormat::Video { max_height: 720 }, Path::new(r"C:\dl\Videos"), "https://youtu.be/x");
        assert!(window(&args, ["-f", "bv*[height<=720]+ba/b[height<=720]"]), "{args:?}");
        assert!(window(&args, ["--merge-output-format", "mp4"]));
        assert!(window(&args, ["-o", r"C:\dl\Videos\%(title)s.%(ext)s"]));
        assert_eq!(args.last().unwrap(), "https://youtu.be/x");
        // Titles with emoji: yt-dlp must report the real file name, not a console-encoded one.
        assert!(window(&args, ["--encoding", "utf-8"]), "{args:?}");
    }

    #[test]
    fn audio_args() {
        let args = build_args(&MediaFormat::AudioMp3, Path::new(r"C:\dl\Music"), "https://youtu.be/x");
        assert!(args.contains(&"-x".to_string()));
        assert!(window(&args, ["--audio-format", "mp3"]));
    }

    #[test]
    fn progress_line_with_total() {
        assert_eq!(
            parse_progress_line("RDMP 1048576 4194304 NA 524288.0"),
            Some(MediaProgress { downloaded: 1_048_576, total: Some(4_194_304), speed_bps: 524_288 })
        );
    }

    #[test]
    fn progress_line_with_estimate_only() {
        assert_eq!(parse_progress_line("RDMP 10 NA 5000.5 NA"), Some(MediaProgress { downloaded: 10, total: Some(5000), speed_bps: 0 }));
    }

    #[test]
    fn ignores_other_lines() {
        assert_eq!(parse_progress_line("[download]  5.0% of 10MiB"), None);
        assert_eq!(parse_progress_line("RDMF C:\\a.mp4"), None);
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    fn p(downloaded: u64, total: u64) -> MediaProgress {
        MediaProgress { downloaded, total: Some(total), speed_bps: 7 }
    }

    #[test]
    fn progress_adds_up_separate_streams() {
        // yt-dlp downloads video, then audio: each stream restarts its own counter.
        let mut acc = StreamProgress::default();
        assert_eq!(acc.push(p(100, 1000)), MediaProgress { downloaded: 100, total: Some(1000), speed_bps: 7 });
        assert_eq!(acc.push(p(1000, 1000)).downloaded, 1000);
        let second = acc.push(p(50, 200));
        assert_eq!((second.downloaded, second.total), (1050, Some(1200)));
        let done = acc.push(p(200, 200));
        assert_eq!((done.downloaded, done.total), (1200, Some(1200)));
    }
}
