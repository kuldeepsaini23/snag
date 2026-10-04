use rdm_media::{MediaFormat, MediaOutcome, MediaProgress, download, probe};
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
    let r = download(&fake(), "fake://ok", &MediaFormat::Video { max_height: 720 }, dir.path(), CancellationToken::new(), &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Completed(PathBuf::from(r"C:\out\clip.mp4"))));
    assert_eq!(rx.borrow().downloaded, 1000);
    assert_eq!(rx.borrow().total, Some(1000));
}

#[tokio::test]
async fn download_failure_returns_last_error_line() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(MediaProgress::default());
    let r = download(&fake(), "fake://fail", &MediaFormat::AudioMp3, dir.path(), CancellationToken::new(), &tx).await;
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
    let r = download(&fake(), "fake://slow", &MediaFormat::Video { max_height: 480 }, dir.path(), cancel, &tx).await;
    assert_eq!(r, Ok(MediaOutcome::Paused));
    assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
}

#[tokio::test]
async fn probe_parses_tool_output() {
    let info = probe(&fake(), "fake://probe").await.unwrap();
    assert_eq!(info.title, "Fake clip");
    assert_eq!(info.options[0].label, "480p");
}
