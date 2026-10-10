//! Subtitle files a page's player loaded next to its video (the extension's sniffer saw them, or
//! the page lists them as `<track>`s): saved beside the finished video as
//! `<video name>.<language>.<ext>`, which VLC, mpv and MPC pick up by themselves, and, when the
//! user wants subtitles, put inside the video too. None of it can fail the download itself.

use crate::model::{SubtitleLink, is_web_link};
use rdm_engine::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A subtitle file is text: anything bigger is not one.
pub const MAX_BYTES: usize = 5 * 1024 * 1024;
/// At most this many subtitle files per video.
pub const MAX_LINKS: usize = 20;
const TIMEOUT: Duration = Duration::from_secs(30);

/// What the extension sent, made safe: web links only, each once, at most `MAX_LINKS`; the
/// language a short tag (`en`, `pt-BR`) and the label one short line.
pub fn clean(links: Vec<SubtitleLink>) -> Vec<SubtitleLink> {
    let mut out: Vec<SubtitleLink> = Vec::new();
    for link in links {
        let url = link.url.trim().to_string();
        if !is_web_link(&url) || url.len() > 4096 || out.iter().any(|l| l.url == url) {
            continue;
        }
        let lang = link.lang.map(|l| l.trim().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(16).collect::<String>().replace('_', "-"));
        let label = link.label.map(|l| l.trim().chars().filter(|c| !c.is_control()).take(80).collect::<String>());
        out.push(SubtitleLink { url, lang: lang.filter(|l| !l.is_empty()), label: label.filter(|l| !l.is_empty()) });
        if out.len() == MAX_LINKS {
            break;
        }
    }
    out
}

/// The kind of subtitle file `body` is, from its text first (servers often say text/plain), then
/// its link and the server's type. `None`: not a subtitle file (an error page, a playlist…).
pub fn kind_of(body: &[u8], url: &str, content_type: &str) -> Option<&'static str> {
    let head = String::from_utf8_lossy(&body[..body.len().min(4096)]);
    let text = head.trim_start_matches('\u{feff}').trim_start();
    if body[..body.len().min(4096)].contains(&0) || text.starts_with('<') || text.starts_with("#EXTM3U") {
        return None;
    }
    let path = url.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase();
    if text.starts_with("WEBVTT") {
        return Some("vtt");
    }
    if text.starts_with("[Script Info]") {
        return Some(if path.ends_with(".ssa") { "ssa" } else { "ass" });
    }
    if text.contains("-->") {
        // SubRip has commas in its times (00:00:01,000); WebVTT without its header has dots.
        return Some(if path.ends_with(".vtt") || content_type.contains("vtt") { "vtt" } else { "srt" });
    }
    None
}

/// The file names for subtitles of `video`, in order: `clip.en.vtt`; one without a language is
/// numbered (`clip.2.srt`), and a second English one becomes `clip.en-2.vtt`.
pub fn sidecar_paths(video: &Path, subs: &[(Option<String>, &str)]) -> Vec<PathBuf> {
    let dir = video.parent().unwrap_or(Path::new("."));
    let stem = video.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "video".into());
    let mut used: Vec<String> = Vec::new();
    subs.iter()
        .enumerate()
        .map(|(i, (lang, ext))| {
            let base = lang.clone().unwrap_or_else(|| (i + 1).to_string());
            let tag = (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|t| !used.iter().any(|u| u.eq_ignore_ascii_case(t))).expect("a free tag");
            used.push(tag.clone());
            dir.join(format!("{stem}.{tag}.{ext}"))
        })
        .collect()
}

/// One subtitle file: its kind (`vtt`, `srt`, `ass`, `ssa`) and bytes.
pub async fn fetch(client: &Client, url: &str) -> Result<(&'static str, Vec<u8>), String> {
    let get = async {
        let mut resp = client.get(url).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?;
        if resp.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
            return Err("too big for a subtitle file".to_string());
        }
        let content_type = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
        let mut body = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
            body.extend_from_slice(&chunk);
            if body.len() > MAX_BYTES {
                return Err("too big for a subtitle file".to_string());
            }
        }
        let kind = kind_of(&body, url, &content_type).ok_or("not a subtitle file")?;
        Ok((kind, body))
    };
    tokio::time::timeout(TIMEOUT, get).await.map_err(|_| "timed out".to_string())?
}

/// A subtitle saved next to the video.
#[derive(Clone, Debug, PartialEq)]
pub struct Saved {
    pub path: PathBuf,
    pub lang: Option<String>,
    pub label: Option<String>,
}

/// Fetches each subtitle (with its own client: the browser's cookies and the page as Referer) and
/// saves the ones that arrive next to `video`. Failures are skipped.
pub async fn save_all(video: &Path, links: Vec<(SubtitleLink, Client)>) -> Vec<Saved> {
    let mut got = Vec::new();
    for (link, client) in links {
        if let Ok((kind, body)) = fetch(&client, &link.url).await {
            got.push((link, kind, body));
        }
    }
    let names: Vec<(Option<String>, &str)> = got.iter().map(|(l, kind, _)| (l.lang.clone(), *kind)).collect();
    let mut saved = Vec::new();
    for ((link, _, body), path) in got.into_iter().zip(sidecar_paths(video, &names)) {
        if std::fs::write(&path, body).is_ok() {
            saved.push(Saved { path, lang: link.lang, label: link.label });
        }
    }
    saved
}

/// The three-letter code MP4 and MKV files name a subtitle's language with (`en` → `eng`).
pub fn iso639_2(lang: &str) -> Option<&'static str> {
    const CODES: [(&str, &str); 32] = [
        ("en", "eng"), ("es", "spa"), ("fr", "fre"), ("de", "ger"), ("it", "ita"), ("pt", "por"), ("ru", "rus"), ("ja", "jpn"), ("ko", "kor"), ("zh", "chi"), ("ar", "ara"),
        ("hi", "hin"), ("tr", "tur"), ("nl", "dut"), ("pl", "pol"), ("id", "ind"), ("vi", "vie"), ("th", "tha"), ("sv", "swe"), ("uk", "ukr"), ("cs", "cze"), ("el", "gre"),
        ("he", "heb"), ("hu", "hun"), ("ro", "rum"), ("da", "dan"), ("fi", "fin"), ("no", "nor"), ("ms", "may"), ("bn", "ben"), ("ta", "tam"), ("fa", "per"),
    ];
    let base = lang.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    if let Some((_, three)) = CODES.iter().find(|(two, three)| *two == base || *three == base) {
        return Some(three);
    }
    None
}

/// The ffmpeg arguments that put `subs` into `video` as `out`, or `None` when the file kind
/// can't hold subtitles. Picture and sound are copied as they are, never encoded again.
pub fn embed_args(video: &Path, subs: &[Saved], out: &Path) -> Option<Vec<String>> {
    let ext = video.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let codec = match ext.as_str() {
        "mp4" | "m4v" | "mov" => "mov_text",
        "mkv" => "copy",
        "webm" => "webvtt",
        _ => return None,
    };
    let mut args: Vec<String> = ["-hide_banner", "-loglevel", "error", "-y", "-i"].map(String::from).to_vec();
    args.push(video.display().to_string());
    for s in subs {
        args.extend(["-i".into(), s.path.display().to_string()]);
    }
    // The new subtitles first (so their numbers are known), then any the video already had.
    args.extend(["-map", "0:v?", "-map", "0:a?"].map(String::from));
    for i in 1..=subs.len() {
        args.extend(["-map".into(), i.to_string()]);
    }
    args.extend(["-map", "0:s?", "-c", "copy", "-c:s", codec].map(String::from));
    for (i, s) in subs.iter().enumerate() {
        if let Some(code) = s.lang.as_deref().and_then(iso639_2) {
            args.extend([format!("-metadata:s:s:{i}"), format!("language={code}")]);
        }
        if let Some(label) = &s.label {
            args.extend([format!("-metadata:s:s:{i}"), format!("title={label}")]);
        }
    }
    args.push(out.display().to_string());
    Some(args)
}

/// Puts the saved subtitles inside `video` (it is replaced only when ffmpeg succeeded; the
/// subtitle files stay either way).
pub fn embed(ffmpeg: &Path, video: &Path, subs: &[Saved]) -> Result<(), String> {
    let stem = video.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = video.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let out = video.with_file_name(format!("{stem}.subtitles-part.{ext}"));
    let args = embed_args(video, subs, &out).ok_or_else(|| format!(".{ext} files can't hold subtitles"))?;
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let failed = match cmd.output() {
        Ok(o) if o.status.success() && out.is_file() => match std::fs::rename(&out, video) {
            Ok(()) => return Ok(()),
            Err(e) => format!("couldn't replace the video: {e}"),
        },
        Ok(o) => format!("ffmpeg failed: {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("").trim()),
        Err(e) => format!("couldn't run ffmpeg: {e}"),
    };
    let _ = std::fs::remove_file(&out);
    Err(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(url: &str, lang: Option<&str>) -> SubtitleLink {
        SubtitleLink { url: url.into(), lang: lang.map(String::from), label: None }
    }

    #[test]
    fn only_web_links_once_with_tidy_languages() {
        let links = clean(vec![
            link("https://cdn.x/en.vtt", Some(" en ")),
            link("https://cdn.x/en.vtt", Some("fr")),
            link("file:///C:/Windows/win.ini", None),
            link("javascript:alert(1)", None),
            link("https://cdn.x/b.srt", Some("pt_BR")),
            link("https://cdn.x/c.srt", Some("../../evil")),
            SubtitleLink { url: "https://cdn.x/d.vtt".into(), lang: Some(String::new()), label: Some("English\n(CC)".into()) },
        ]);
        let urls: Vec<&str> = links.iter().map(|l| l.url.as_str()).collect();
        assert_eq!(urls, ["https://cdn.x/en.vtt", "https://cdn.x/b.srt", "https://cdn.x/c.srt", "https://cdn.x/d.vtt"]);
        assert_eq!(links[0].lang.as_deref(), Some("en"));
        assert_eq!(links[1].lang.as_deref(), Some("pt-BR"));
        assert_eq!(links[2].lang.as_deref(), Some("evil"), "never a path");
        assert_eq!(links[3].lang, None);
        assert_eq!(links[3].label.as_deref(), Some("English(CC)"));
        let many = clean((0..50).map(|i| link(&format!("https://cdn.x/{i}.vtt"), None)).collect());
        assert_eq!(many.len(), MAX_LINKS);
    }

    #[test]
    fn subtitle_kinds_are_read_from_the_text() {
        assert_eq!(kind_of(b"WEBVTT\n\n00:01.000 --> 00:02.000\nHi", "https://x/s", "text/plain"), Some("vtt"));
        assert_eq!(kind_of("\u{feff}WEBVTT".as_bytes(), "https://x/s.txt", ""), Some("vtt"), "a byte order mark first");
        assert_eq!(kind_of(b"1\r\n00:00:01,000 --> 00:00:02,000\r\nHi", "https://x/subs?id=1", "application/octet-stream"), Some("srt"));
        assert_eq!(kind_of(b"[Script Info]\nTitle: x", "https://x/a.ass", ""), Some("ass"));
        assert_eq!(kind_of(b"[Script Info]\nTitle: x", "https://x/a.SSA?v=1", ""), Some("ssa"));
        assert_eq!(kind_of(b"00:01.000 --> 00:02.000\nHi", "https://x/a.vtt", ""), Some("vtt"), "WebVTT missing its header");
        assert_eq!(kind_of(b"<!doctype html><title>404</title>", "https://x/a.vtt", "text/html"), None, "an error page");
        assert_eq!(kind_of(b"#EXTM3U\n#EXT-X-TARGETDURATION:6", "https://x/subs.m3u8", ""), None);
        assert_eq!(kind_of(b"\x00\x01binary-->", "https://x/a.srt", ""), None);
        assert_eq!(kind_of(b"just some words", "https://x/a.srt", ""), None);
    }

    #[test]
    fn sidecars_are_named_after_the_video() {
        let video = Path::new("dl").join("Video").join("Sintel [abc].mp4");
        let paths = sidecar_paths(&video, &[(Some("en".into()), "vtt"), (None, "srt"), (Some("EN".into()), "vtt"), (Some("de".into()), "ass")]);
        let names: Vec<String> = paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, ["Sintel [abc].en.vtt", "Sintel [abc].2.srt", "Sintel [abc].EN-2.vtt", "Sintel [abc].de.ass"]);
        assert!(paths.iter().all(|p| p.parent() == video.parent()), "next to the video");
    }

    #[test]
    fn embedding_copies_picture_and_sound() {
        let subs = [
            Saved { path: "v.en.vtt".into(), lang: Some("en".into()), label: Some("English".into()) },
            Saved { path: "v.2.srt".into(), lang: None, label: None },
        ];
        let args = embed_args(Path::new("v.mp4"), &subs, Path::new("out.mp4")).unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("-i v.mp4 -i v.en.vtt -i v.2.srt"), "{joined}");
        assert!(joined.contains("-map 0:v? -map 0:a? -map 1 -map 2 -map 0:s? -c copy -c:s mov_text"), "{joined}");
        assert!(joined.contains("-metadata:s:s:0 language=eng -metadata:s:s:0 title=English"), "{joined}");
        assert!(!joined.contains("s:s:1"), "nothing known about the second: {joined}");
        assert!(!args.iter().any(|a| a == "-c:v" || a == "-c:a"), "never encoded again");
        assert!(embed_args(Path::new("v.mkv"), &subs, Path::new("o.mkv")).unwrap().join(" ").contains("-c:s copy"));
        assert!(embed_args(Path::new("v.webm"), &subs, Path::new("o.webm")).unwrap().join(" ").contains("-c:s webvtt"));
        assert_eq!(embed_args(Path::new("v.ts"), &subs, Path::new("o.ts")), None);
        assert_eq!(iso639_2("pt-BR"), Some("por"));
        assert_eq!(iso639_2("eng"), Some("eng"));
        assert_eq!(iso639_2("xx"), None);
    }

    #[test]
    fn a_failed_embed_leaves_the_video_alone() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("v.mp4");
        std::fs::write(&video, b"video").unwrap();
        let fake = dir.path().join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&fake, b"").unwrap();
        let subs = [Saved { path: dir.path().join("v.en.vtt"), lang: None, label: None }];
        assert!(embed(&fake, &video, &subs).is_err());
        assert_eq!(std::fs::read(&video).unwrap(), b"video");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2, "no half-written file left");
    }
}

/// Real ffmpeg from PATH. Run with: cargo test -p rdm-core --lib real_ffmpeg -- --ignored
#[cfg(test)]
mod real {
    use super::*;

    #[test]
    #[ignore]
    fn real_ffmpeg_embeds_subtitles() {
        let ffmpeg = PathBuf::from("ffmpeg");
        let dir = tempfile::tempdir().unwrap();
        for ext in ["mp4", "mkv"] {
            let clip = dir.path().join(format!("clip.{ext}"));
            let made = std::process::Command::new(&ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc=duration=2:size=320x180", "-f", "lavfi", "-i", "sine=duration=2", "-shortest"])
                .arg(&clip)
                .status()
                .unwrap();
            assert!(made.success());
            let vtt = dir.path().join("clip.en.vtt");
            std::fs::write(&vtt, "WEBVTT\n\n00:00.000 --> 00:01.000\nHello\n").unwrap();
            let ass = dir.path().join("clip.de.ass");
            std::fs::write(&ass, "[Script Info]\nScriptType: v4.00+\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,Hallo\n").unwrap();
            let subs = [Saved { path: vtt, lang: Some("en".into()), label: Some("English".into()) }, Saved { path: ass, lang: Some("de".into()), label: None }];
            embed(&ffmpeg, &clip, &subs).unwrap();
            let probe = std::process::Command::new("ffprobe").args(["-v", "error", "-select_streams", "s", "-show_entries", "stream=codec_name:stream_tags=language", "-of", "csv=p=0"]).arg(&clip).output().unwrap();
            let listed = String::from_utf8_lossy(&probe.stdout).to_string();
            assert_eq!(listed.lines().count(), 2, "{ext}: {listed}");
            assert!(listed.contains("eng") && listed.contains("ger"), "{ext}: {listed}");
            let video = std::process::Command::new("ffprobe").args(["-v", "error", "-select_streams", "v", "-show_entries", "stream=codec_name", "-of", "csv=p=0"]).arg(&clip).output().unwrap();
            assert_eq!(String::from_utf8_lossy(&video.stdout).trim(), "h264", "{ext}: the picture is copied");
        }
    }
}
