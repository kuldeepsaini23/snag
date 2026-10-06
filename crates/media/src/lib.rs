//! Video and audio downloads through yt-dlp (+ ffmpeg for merging and MP3).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub mod gallery;

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
    pub thumbnail: Option<String>,
    /// Seconds.
    pub duration: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaInfo {
    pub title: String,
    pub duration: Option<f64>,
    pub options: Vec<QualityOption>,
    /// Non-empty for playlists and channels.
    pub entries: Vec<Entry>,
    /// A small preview image (about 320 px wide when the site offers sizes).
    pub thumbnail: Option<String>,
}

/// The smallest listed thumbnail that is still at least 320 px wide, else the main one.
fn pick_thumbnail(v: &serde_json::Value) -> Option<String> {
    let listed = v["thumbnails"].as_array().into_iter().flatten().filter_map(|t| Some((t["width"].as_u64()?, t["url"].as_str()?)));
    let small = listed.filter(|(w, _)| *w >= 320).min_by_key(|(w, _)| *w).map(|(_, u)| u.to_string());
    small.or_else(|| v["thumbnail"].as_str().map(str::to_string))
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

/// Per-download extras for yt-dlp.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaOptions {
    /// Netscape cookies.txt from the browser (`--cookies`).
    pub cookies: Option<PathBuf>,
    /// Bytes per second, 0 = unlimited (`--limit-rate`).
    pub limit_bps: u64,
    /// Where partial files live until the final file is moved into place (`-P temp:`).
    pub temp_dir: Option<PathBuf>,
    /// Videos: subtitle languages to fetch and embed (yt-dlp `--sub-langs`, e.g. "en.*").
    pub subtitles: Option<String>,
}

impl MediaInfo {
    /// Index of the option that best matches `pref`: None = the best (first) option;
    /// a video height = the highest option at or below it, else the lowest video; MP3 = the MP3 option.
    pub fn preferred(&self, pref: Option<&MediaFormat>) -> usize {
        let video_height = |o: &QualityOption| match o.format {
            MediaFormat::Video { max_height } => Some(max_height),
            MediaFormat::AudioMp3 => None,
        };
        let found = match pref {
            None => None,
            Some(MediaFormat::AudioMp3) => self.options.iter().position(|o| o.format == MediaFormat::AudioMp3),
            Some(MediaFormat::Video { max_height }) => {
                let videos = || self.options.iter().enumerate().filter_map(|(i, o)| video_height(o).map(|h| (i, h)));
                videos()
                    .filter(|(_, h)| h <= max_height)
                    .max_by_key(|(_, h)| *h)
                    .or_else(|| videos().min_by_key(|(_, h)| *h))
                    .map(|(i, _)| i)
            }
        };
        found.unwrap_or(0)
    }

    /// What to add for `choice`: (url, title, format), one item or one per playlist entry.
    pub fn requests(&self, url: &str, choice: usize) -> Vec<(String, String, MediaFormat)> {
        let Some(option) = self.options.get(choice) else { return Vec::new() };
        if self.entries.is_empty() {
            return vec![(url.to_string(), self.title.clone(), option.format.clone())];
        }
        self.entries.iter().map(|e| (e.url.clone(), e.title.clone(), option.format.clone())).collect()
    }

    /// (thumbnail, duration) for each of `requests`, in the same order.
    pub fn request_meta(&self) -> Vec<(Option<String>, Option<f64>)> {
        if self.entries.is_empty() {
            return vec![(self.thumbnail.clone(), self.duration)];
        }
        self.entries.iter().map(|e| (e.thumbnail.clone(), e.duration)).collect()
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
                Some(Entry {
                    url: url.to_string(),
                    title: e["title"].as_str().unwrap_or(url).to_string(),
                    thumbnail: pick_thumbnail(e),
                    duration: e["duration"].as_f64(),
                })
            })
            .collect();
        // A flat playlist doesn't list formats: offer the usual choices.
        let mut options: Vec<_> = [1080, 720, 480]
            .into_iter()
            .map(|h| QualityOption { label: format!("{h}p"), format: MediaFormat::Video { max_height: h }, approx_size: None })
            .collect();
        options.push(audio_mp3(None));
        return Ok(MediaInfo { title, duration, options, entries, thumbnail: pick_thumbnail(&v) });
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
    Ok(MediaInfo { title, duration, options, entries: Vec::new(), thumbnail: pick_thumbnail(&v) })
}

pub fn build_args(format: &MediaFormat, out_dir: &Path, url: &str, opts: &MediaOptions) -> Vec<String> {
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
    if let (MediaFormat::Video { .. }, Some(langs)) = (format, &opts.subtitles) {
        // Refused subtitles (YouTube often answers 429) must not stop the video itself.
        args.extend(["--write-subs".into(), "--write-auto-subs".into(), "--sub-langs".into(), langs.clone(), "--embed-subs".into(), "--ignore-errors".into()]);
    }
    if let Some(cookies) = &opts.cookies {
        args.extend(["--cookies".into(), cookies.display().to_string()]);
    }
    if opts.limit_bps > 0 {
        args.extend(["--limit-rate".into(), opts.limit_bps.to_string()]);
    }
    if let Some(temp) = &opts.temp_dir {
        // Partial files stay in the item's own folder, so removing the item can clean them up.
        args.extend(["-P".into(), format!("temp:{}", temp.display())]);
    }
    // Title alone isn't unique: two videos (or one video twice) would share a file.
    args.extend(["-P".into(), out_dir.display().to_string(), "-o".into(), "%(title).150B [%(id)s].%(ext)s".into()]);
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

/// Arguments for reading a link. A link to one video inside a playlist (`watch?v=…&list=…`)
/// is read as that one video.
pub fn probe_args(url: &str, cookies: Option<&Path>) -> Vec<String> {
    let query = url.split_once('?').map_or("", |(_, q)| q);
    let one_video = query.split('&').any(|kv| kv.starts_with("v="));
    let mut args: Vec<String> = ["-J", "--flat-playlist", "--no-warnings"].map(String::from).to_vec();
    if one_video {
        args.push("--no-playlist".into());
    }
    if let Some(cookies) = cookies {
        args.extend(["--cookies".into(), cookies.display().to_string()]);
    }
    args.push(url.to_string());
    args
}

pub async fn probe(ytdlp: &Path, url: &str, cookies: Option<&Path>) -> Result<MediaInfo, String> {
    let out = command(ytdlp)
        .args(probe_args(url, cookies))
        .output()
        .await
        .map_err(|e| format!("can't start yt-dlp: {e}"))?;
    if !out.status.success() {
        return Err(error_from(&String::from_utf8_lossy(&out.stderr)).unwrap_or_else(|| format!("yt-dlp failed ({})", out.status)));
    }
    parse_probe(&String::from_utf8_lossy(&out.stdout))
}

/// `yt-dlp -U`: updates the standalone exe in place. Returns its last line
/// ("Updated yt-dlp to …" or "yt-dlp is up to date …").
pub async fn self_update(ytdlp: &Path) -> Result<String, String> {
    let out = command(ytdlp).args(["-U", "--encoding", "utf-8"]).output().await.map_err(|e| format!("can't start yt-dlp: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().map(str::trim).rfind(|l| !l.is_empty()).map(str::to_string);
    if !out.status.success() {
        return Err(error_from(&String::from_utf8_lossy(&out.stderr)).or(last).unwrap_or_else(|| format!("yt-dlp failed ({})", out.status)));
    }
    Ok(last.unwrap_or_else(|| "yt-dlp is up to date".into()))
}

/// Runs yt-dlp until it finishes or `cancel` fires (the process is killed; yt-dlp
/// continues its `.part` files next time).
pub async fn download(
    ytdlp: &Path,
    url: &str,
    format: &MediaFormat,
    out_dir: &Path,
    opts: &MediaOptions,
    cancel: CancellationToken,
    progress: &watch::Sender<MediaProgress>,
) -> Result<MediaOutcome, String> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

    tokio::fs::create_dir_all(out_dir).await.map_err(|e| format!("can't create {}: {e}", out_dir.display()))?;
    let mut child = command(ytdlp)
        .args(build_args(format, out_dir, url, opts))
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
    // The final file was moved into place: whatever failed after that (subtitles, embedding)
    // doesn't undo the download.
    if let (false, Some(path)) = (status.success(), final_path.clone()) {
        return Ok(MediaOutcome::Completed(path));
    }
    if !status.success() {
        return Err(error_from(&stderr).unwrap_or_else(|| format!("yt-dlp failed ({status})")));
    }
    final_path.map(MediaOutcome::Completed).ok_or_else(|| "yt-dlp finished without reporting the file".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reads_a_small_thumbnail() {
        let json = r#"{"title":"T","duration":2538.0,"thumbnail":"https://i.ytimg.com/vi/x/maxresdefault.webp",
            "thumbnails":[{"url":"https://i.ytimg.com/vi/x/default.jpg","width":120},
                          {"url":"https://i.ytimg.com/vi/x/mqdefault.jpg","width":320},
                          {"url":"https://i.ytimg.com/vi/x/maxresdefault.jpg","width":1280},
                          {"url":"https://i.ytimg.com/vi/x/nowidth.jpg"}],
            "formats":[]}"#;
        let info = parse_probe(json).unwrap();
        assert_eq!(info.thumbnail.as_deref(), Some("https://i.ytimg.com/vi/x/mqdefault.jpg"), "smallest that is still sharp");
        let only_main = parse_probe(r#"{"title":"T","thumbnail":"https://x/t.jpg","formats":[]}"#).unwrap();
        assert_eq!(only_main.thumbnail.as_deref(), Some("https://x/t.jpg"));
        assert_eq!(parse_probe(r#"{"title":"T","formats":[]}"#).unwrap().thumbnail, None);
    }

    #[test]
    fn playlist_entries_carry_thumbnail_and_duration() {
        let json = r#"{"_type":"playlist","title":"L","entries":[
            {"url":"https://y/1","title":"One","duration":61.0,"thumbnails":[{"url":"https://i/1.jpg","width":336}]},
            {"url":"https://y/2","title":"Two"}]}"#;
        let info = parse_probe(json).unwrap();
        assert_eq!(info.entries[0].thumbnail.as_deref(), Some("https://i/1.jpg"));
        assert_eq!(info.entries[0].duration, Some(61.0));
        assert_eq!(info.entries[1].thumbnail, None);
    }

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
        assert_eq!(info.entries[2], Entry { url: "https://www.youtube.com/watch?v=c3".into(), title: "Three".into(), thumbnail: None, duration: None });
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
        let args = build_args(&MediaFormat::Video { max_height: 720 }, Path::new(r"C:\dl\Videos"), "https://youtu.be/x", &MediaOptions::default());
        assert!(window(&args, ["-f", "bv*[height<=720]+ba/b[height<=720]"]), "{args:?}");
        assert!(window(&args, ["--merge-output-format", "mp4"]));
        // Unique per video (title + id), written into the folder given with -P.
        assert!(window(&args, ["-P", r"C:\dl\Videos"]), "{args:?}");
        assert!(window(&args, ["-o", "%(title).150B [%(id)s].%(ext)s"]), "{args:?}");
        assert_eq!(args.last().unwrap(), "https://youtu.be/x");
        // Titles with emoji: yt-dlp must report the real file name, not a console-encoded one.
        assert!(window(&args, ["--encoding", "utf-8"]), "{args:?}");
    }

    #[test]
    fn audio_args() {
        let args = build_args(&MediaFormat::AudioMp3, Path::new(r"C:\dl\Music"), "https://youtu.be/x", &MediaOptions::default());
        assert!(args.contains(&"-x".to_string()));
        assert!(window(&args, ["--audio-format", "mp3"]));
    }

    #[test]
    fn probe_args_keep_a_single_video_from_a_playlist_link() {
        let single = probe_args("https://www.youtube.com/watch?v=abc&list=PL1", None);
        assert!(single.contains(&"--no-playlist".to_string()), "{single:?}");
        let list = probe_args("https://www.youtube.com/playlist?list=PL1", None);
        assert!(!list.contains(&"--no-playlist".to_string()), "{list:?}");
        assert_eq!(list.last().unwrap(), "https://www.youtube.com/playlist?list=PL1");
    }

    #[test]
    fn subtitles_are_fetched_and_embedded_for_videos_only() {
        let opts = MediaOptions { subtitles: Some("en.*,hi".into()), ..Default::default() };
        let video = build_args(&MediaFormat::Video { max_height: 720 }, Path::new("out"), "u", &opts);
        assert!(window(&video, ["--sub-langs", "en.*,hi"]), "{video:?}");
        for flag in ["--write-subs", "--write-auto-subs", "--embed-subs", "--ignore-errors"] {
            assert!(video.iter().any(|a| a == flag), "{flag}: {video:?}");
        }
        let mp3 = build_args(&MediaFormat::AudioMp3, Path::new("out"), "u", &opts);
        assert!(!mp3.iter().any(|a| a.contains("sub")), "no subtitles in an MP3: {mp3:?}");
        let none = build_args(&MediaFormat::Video { max_height: 720 }, Path::new("out"), "u", &MediaOptions::default());
        assert!(!none.iter().any(|a| a.contains("sub")));
    }

    #[test]
    fn args_carry_cookies_limit_and_temp_dir() {
        let opts = MediaOptions {
            cookies: Some(PathBuf::from(r"C:\rdm\cookies\7.txt")),
            limit_bps: 512 * 1024,
            temp_dir: Some(PathBuf::from(r"C:\dl\Videos\.rdm-parts\7")),
            ..Default::default()
        };
        let args = build_args(&MediaFormat::Video { max_height: 720 }, Path::new(r"C:\dl\Videos"), "https://youtu.be/x", &opts);
        assert!(window(&args, ["--cookies", r"C:\rdm\cookies\7.txt"]), "{args:?}");
        assert!(window(&args, ["--limit-rate", "524288"]), "{args:?}");
        assert!(window(&args, ["-P", r"temp:C:\dl\Videos\.rdm-parts\7"]), "{args:?}");
        assert!(window(&args, ["-P", r"C:\dl\Videos"]), "home folder still given: {args:?}");
        assert_eq!(args.last().unwrap(), "https://youtu.be/x");
    }

    #[test]
    fn args_without_options_have_none_of_them() {
        let args = build_args(&MediaFormat::AudioMp3, Path::new(r"C:\dl"), "https://youtu.be/x", &MediaOptions::default());
        for flag in ["--cookies", "--limit-rate"] {
            assert!(!args.iter().any(|a| a == flag), "{flag} in {args:?}");
        }
        assert!(!args.iter().any(|a| a.starts_with("temp:")), "{args:?}");
    }

    #[test]
    fn probe_args_carry_cookies() {
        let args = probe_args("https://www.youtube.com/watch?v=abc", Some(Path::new(r"C:\c.txt")));
        assert!(window(&args, ["--cookies", r"C:\c.txt"]), "{args:?}");
        assert_eq!(args.last().unwrap(), "https://www.youtube.com/watch?v=abc");
        assert!(!probe_args("https://youtu.be/a", None).iter().any(|a| a == "--cookies"));
    }

    fn heights(info: &MediaInfo) -> Vec<String> {
        info.options.iter().map(|o| o.label.clone()).collect()
    }

    #[test]
    fn preferred_picks_highest_not_above() {
        let info = parse_probe(SINGLE).unwrap(); // 1080p, 720p, 360p, MP3
        assert_eq!(heights(&info).len(), 4);
        assert_eq!(info.preferred(None), 0);
        assert_eq!(info.preferred(Some(&MediaFormat::Video { max_height: 2160 })), 0);
        assert_eq!(info.preferred(Some(&MediaFormat::Video { max_height: 1080 })), 0);
        assert_eq!(info.preferred(Some(&MediaFormat::Video { max_height: 720 })), 1);
        assert_eq!(info.preferred(Some(&MediaFormat::Video { max_height: 480 })), 2);
    }

    #[test]
    fn preferred_falls_back_to_lowest_video() {
        let info = parse_probe(SINGLE).unwrap();
        assert_eq!(info.preferred(Some(&MediaFormat::Video { max_height: 240 })), 2, "360p is the lowest video");
        let audio_only = parse_probe(r#"{"title":"Song","formats":[{"vcodec":"none","acodec":"opus","filesize":700}]}"#).unwrap();
        assert_eq!(audio_only.preferred(Some(&MediaFormat::Video { max_height: 720 })), 0, "no video at all: the only option");
    }

    #[test]
    fn preferred_mp3() {
        let info = parse_probe(SINGLE).unwrap();
        assert_eq!(info.preferred(Some(&MediaFormat::AudioMp3)), 3);
    }

    #[test]
    fn requests_single_and_playlist() {
        let single = parse_probe(SINGLE).unwrap();
        assert_eq!(
            single.requests("https://youtu.be/x", 3),
            vec![("https://youtu.be/x".to_string(), "Test clip".to_string(), MediaFormat::AudioMp3)]
        );
        assert!(single.requests("https://youtu.be/x", 99).is_empty(), "out of range: nothing");
        let list = parse_probe(r#"{"_type":"playlist","title":"L","entries":[{"url":"https://y/a","title":"A"},{"url":"https://y/b","title":"B"}]}"#).unwrap();
        let reqs = list.requests("https://y/list", 1);
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[1], ("https://y/b".to_string(), "B".to_string(), MediaFormat::Video { max_height: 720 }));
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

#[cfg(test)]
mod meta_tests {
    use super::*;

    #[test]
    fn request_meta_lines_up_with_requests() {
        let entry = |n: u32| Entry { url: format!("https://y/{n}"), title: format!("V{n}"), thumbnail: Some(format!("https://i/{n}.jpg")), duration: Some(n as f64) };
        let video = QualityOption { label: "720p".into(), format: MediaFormat::Video { max_height: 720 }, approx_size: None };
        let list = MediaInfo { title: "L".into(), duration: None, options: vec![video.clone()], entries: vec![entry(1), entry(2)], thumbnail: None };
        assert_eq!(list.request_meta(), vec![(Some("https://i/1.jpg".to_string()), Some(1.0)), (Some("https://i/2.jpg".to_string()), Some(2.0))]);
        assert_eq!(list.request_meta().len(), list.requests("u", 0).len());
        let single = MediaInfo { title: "S".into(), duration: Some(9.0), options: vec![video], entries: vec![], thumbnail: Some("https://i/s.jpg".into()) };
        assert_eq!(single.request_meta(), vec![(Some("https://i/s.jpg".to_string()), Some(9.0))]);
    }
}
