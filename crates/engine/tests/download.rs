mod support;

use rdm_engine::state::{DownloadState, part_path, state_path};
use rdm_engine::{CancellationToken, DownloadOptions, EngineError, Outcome, Progress, default_client, download};
use std::path::Path;
use std::time::Duration;
use support::{TestServer, data};
use tokio::sync::watch;

fn opts(connections: usize) -> DownloadOptions {
    DownloadOptions {
        connections,
        min_split: 64 * 1024,
        retry_base: Duration::from_millis(10),
        ..Default::default()
    }
}

async fn run(url: &str, dest: &Path, o: &DownloadOptions, cancel: CancellationToken) -> (Result<Outcome, EngineError>, Progress) {
    let (tx, rx) = watch::channel(Progress::default());
    let r = download(&default_client(), url, dest, o, cancel, &tx).await;
    let last = rx.borrow().clone();
    (r, last)
}

fn cancel_after(ms: u64) -> CancellationToken {
    let t = CancellationToken::new();
    let t2 = t.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        t2.cancel();
    });
    t
}

#[tokio::test]
async fn downloads_byte_exact_with_many_connections_into_new_dir() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("nested").join("out.bin");
    let size = 5 * 1024 * 1024 + 123;
    let (r, last) = run(&s.url(&format!("/file/{size}")), &dest, &opts(8), CancellationToken::new()).await;
    assert_eq!(r.unwrap(), Outcome::Completed(dest.clone()));
    assert_eq!(std::fs::read(&dest).unwrap(), data(size));
    assert!(!part_path(&dest).exists());
    assert!(!state_path(&dest).exists());
    assert_eq!(last.downloaded, size as u64);
}

#[tokio::test]
async fn single_connection_works() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("one.bin");
    let (r, _) = run(&s.url("/file/700000"), &dest, &opts(1), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(700_000));
}

#[tokio::test]
async fn pause_then_resume_keeps_progress() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("big.bin");
    let size = 4 * 1024 * 1024;
    let url = s.url(&format!("/slow/{size}"));

    let (r, _) = run(&url, &dest, &opts(4), cancel_after(500)).await;
    assert_eq!(r.unwrap(), Outcome::Paused);
    assert!(!dest.exists());
    assert!(part_path(&dest).exists());
    let saved = DownloadState::load(&state_path(&dest)).await.unwrap().expect("sidecar written");
    assert!(saved.downloaded() > 0 && saved.downloaded() < size as u64, "saved {}", saved.downloaded());

    let (r, _) = run(&url, &dest, &opts(4), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(size));
    assert!(!state_path(&dest).exists());
}

#[tokio::test]
async fn retries_dropped_connections() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("flaky.bin");
    let size = 2 * 1024 * 1024;
    let (r, _) = run(&s.url(&format!("/flaky/{size}")), &dest, &opts(2), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(size));
}

#[tokio::test]
async fn range_not_honored_fails_cleanly() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("cdn.bin");
    let (r, _) = run(&s.url("/ranges-once/2000000"), &dest, &opts(4), CancellationToken::new()).await;
    assert!(matches!(r, Err(EngineError::RangeNotHonored)), "{r:?}");
    assert!(!dest.exists());
}

#[tokio::test]
async fn expired_link_is_reported() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (r, _) = run(&s.url("/expired"), &dir.path().join("x"), &opts(4), CancellationToken::new()).await;
    assert!(matches!(r, Err(EngineError::LinkExpired(403))), "{r:?}");
}
