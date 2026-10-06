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
    let info = probe(&fake(), "fake://probe", None, None).await.unwrap();
    assert_eq!(info.title, "Fake clip");
    assert_eq!(info.options[0].label, "480p");
}

#[tokio::test]
async fn self_update_reports_last_line() {
    assert_eq!(self_update(&fake()).await, Ok("yt-dlp is up to date (fake)".to_string()));
}

/// Real yt-dlp + network: --limit-rate keeps the speed near the limit.
/// Run with: cargo test -p rdm-media --test run real_limit -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn real_limit_rate_holds() {
    let ytdlp = PathBuf::from(std::env::var("APPDATA").unwrap()).join(r"rdm\bin\yt-dlp.exe");
    let dir = tempfile::tempdir().unwrap();
    let (tx, mut rx) = watch::channel(MediaProgress::default());
    let cancel = CancellationToken::new();
    let limit = 512 * 1024;
    let opts = MediaOptions { limit_bps: limit, temp_dir: Some(dir.path().join("parts")), ..Default::default() };
    let watcher = tokio::spawn(async move {
        let started = Instant::now();
        let mut speeds = Vec::new();
        while rx.changed().await.is_ok() {
            let p = rx.borrow_and_update().clone();
            if started.elapsed() > Duration::from_secs(4) && p.speed_bps > 0 {
                speeds.push(p.speed_bps);
            }
        }
        speeds
    });
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(12)).await;
        stop.cancel();
    });
    let r = download(&ytdlp, "https://www.youtube.com/watch?v=aqz-KE-bpKQ", &MediaFormat::Video { max_height: 1080 }, dir.path(), &opts, cancel, &tx).await;
    drop(tx);
    let speeds = watcher.await.unwrap();
    assert_eq!(r, Ok(MediaOutcome::Paused));
    let avg = speeds.iter().sum::<u64>() / speeds.len().max(1) as u64;
    println!("samples={} avg={avg} max={:?}", speeds.len(), speeds.iter().max());
    assert!(!speeds.is_empty());
    assert!(avg < limit * 13 / 10, "average {avg} B/s over a {limit} B/s limit");
    assert!(dir.path().join("parts").exists(), "partial files are in the temp folder");
    let in_home: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).filter(|n| n != "parts").collect();
    assert!(in_home.is_empty(), "nothing partial in the output folder: {in_home:?}");
}

fn count(image: &str) -> usize {
    let out = std::process::Command::new("tasklist").args(["/FO", "CSV", "/NH", "/FI", &format!("IMAGENAME eq {image}")]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.starts_with('"')).count()
}

/// Real yt-dlp (PyInstaller) + network. Run with: cargo test -p rdm-media --test run real_cancel -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn real_cancel_leaves_no_processes_behind() {
    let ytdlp = PathBuf::from(std::env::var("APPDATA").unwrap()).join("Snag").join("bin").join("yt-dlp.exe");
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

#[tokio::test]
async fn gallery_downloads_every_image_into_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("Images").join("pin");
    let (tx, rx) = watch::channel(MediaProgress::default());
    let r = rdm_media::gallery::download(&fake(), "fake://ok", &out, &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Completed(out.clone())));
    assert_eq!(rx.borrow().downloaded, 350, "new and already-there images both count");
    assert!(out.join("3.jpg").exists());
}

#[tokio::test]
async fn gallery_survives_output_that_is_not_utf8() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("X post");
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let r = rdm_media::gallery::download(&fake(), "fake://codepage", &out, &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Completed(out.clone())));
}

#[tokio::test]
async fn gallery_reports_errors_and_empty_pages() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let r = rdm_media::gallery::download(&fake(), "fake://nope", dir.path(), &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Err("No suitable extractor found for 'fake://nope'".to_string()));
    let r = rdm_media::gallery::download(&fake(), "fake://empty", dir.path(), &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Err("No images found on that page".to_string()));
}

#[tokio::test]
async fn gallery_pause_kills_it() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        stop.cancel();
    });
    let r = tokio::time::timeout(std::time::Duration::from_secs(10), rdm_media::gallery::download(&fake(), "fake://slow", dir.path(), &MediaOptions::default(), cancel, &tx)).await;
    assert_eq!(r, Ok(Ok(MediaOutcome::Paused)));
}

#[tokio::test]
async fn refused_subtitles_dont_fail_a_saved_video() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let r = download(&fake(), "fake://subfail", &MediaFormat::Video { max_height: 720 }, dir.path(), &MediaOptions::default(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Completed(PathBuf::from(r"C:\out\clip.mp4"))));
}

#[tokio::test]
async fn stopping_a_live_recording_keeps_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        stop.cancel();
    });
    let r = download(&fake(), "fake://live", &MediaFormat::Live { max_height: 720 }, dir.path(), &MediaOptions::default(), cancel, &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Completed(dir.path().join("Launch [live].mp4"))), "stop = finished recording, not paused");
}
