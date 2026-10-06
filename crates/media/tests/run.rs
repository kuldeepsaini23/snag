use rdm_media::{MediaFormat, MediaOptions, MediaOutcome, MediaProgress, download, probe, self_update};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-ytdlp"))
}

#[tokio::test]
async fn download_reports_progress_and_final_path() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = watch::channel(MediaProgress::default());
    let r = download(&fake(), "fake://ok", &MediaFormat::Video { max_height: 720 }, dir.path(), &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Completed(PathBuf::from(r"C:\out\clip.mp4"))));
    assert_eq!(rx.borrow().downloaded, 1000);
    assert_eq!(rx.borrow().total, Some(1000));
}

#[tokio::test]
async fn download_failure_returns_last_error_line() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let r = download(&fake(), "fake://fail", &MediaFormat::AudioMp3, dir.path(), &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Err("Unsupported URL: fake://fail".to_string()));
}

#[tokio::test]
async fn cancel_kills_process_and_pauses() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        trigger.cancel();
    });
    let started = Instant::now();
    let r = download(&fake(), "fake://slow", &MediaFormat::Video { max_height: 480 }, dir.path(), &MediaOptions::default(), cancel, &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Paused));
    assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
}

#[tokio::test]
async fn probe_parses_tool_output() {
    let info = probe(&fake(), "fake://probe", None).await.unwrap();
    assert_eq!(info.title, "Fake clip");
    assert_eq!(info.options[0].label, "480p");
}

#[tokio::test]
async fn self_update_reports_last_line() {
    assert_eq!(self_update(&fake()).await, Ok("yt-dlp is up to date (fake)".to_string()));
}

fn count(image: &str) -> usize {
    let out = std::process::Command::new("tasklist").args(["/FO", "CSV", "/NH", "/FI", &format!("IMAGENAME eq {image}")]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.starts_with('"')).count()
}

/// Real yt-dlp (PyInstaller) + network. Run with: cargo test -p rdm-media --test run real_cancel -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn real_cancel_leaves_no_processes_behind() {
    let ytdlp = PathBuf::from(std::env::var("APPDATA").unwrap()).join("rdm").join("bin").join("yt-dlp.exe");
    let before = (count("yt-dlp.exe"), count("ffmpeg.exe"));
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = watch::channel(MediaProgress::default());
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        trigger.cancel();
    });
    let r = download(&ytdlp, "https://www.youtube.com/watch?v=aqz-KE-bpKQ", &MediaFormat::Video { max_height: 1080 }, dir.path(), &MediaOptions::default(), cancel, &tx).await;
    println!("outcome {r:?}, downloaded {} bytes", rx.borrow().downloaded);
    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = (count("yt-dlp.exe"), count("ffmpeg.exe"));
    println!("yt-dlp/ffmpeg processes before {before:?}, after {after:?}");
    assert_eq!(r, Ok(MediaOutcome::Paused));
    assert_eq!(after, before, "pausing left processes running");
}
