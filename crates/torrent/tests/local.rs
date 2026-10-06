//! A real torrent between two engines on this machine (no internet, no DHT, no trackers).

use rdm_torrent::{EngineOptions, Outcome, Progress, Source, TorrentEngine};
use std::time::Duration;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

async fn seeded(size: usize) -> (tempfile::TempDir, Vec<u8>, Vec<u8>, TorrentEngine) {
    let seed_dir = tempfile::tempdir().unwrap();
    let data: Vec<u8> = (0..size).map(|i| (i * 31 % 251) as u8).collect();
    let file = seed_dir.path().join("hello.bin");
    std::fs::write(&file, &data).unwrap();
    let spawner = librqbit::spawn_utils::BlockingSpawner::new(2);
    let created = librqbit::create_torrent(&file, librqbit::CreateTorrentOptions { name: None, trackers: vec![], piece_length: Some(16384) }, &spawner).await.unwrap();
    let bytes = created.as_bytes().unwrap().to_vec();
    let seeder = TorrentEngine::start(seed_dir.path(), EngineOptions { local_only: true, peers: vec![] }).await.unwrap();
    let (tx, _rx) = watch::channel(Progress::default());
    // Already complete on disk: checking it finishes at once.
    let done = seeder.download(&Source::File(bytes.clone()), seed_dir.path(), CancellationToken::new(), &tx).await;
    assert!(matches!(done, Ok(Outcome::Completed(_))), "{done:?}");
    (seed_dir, data, bytes, seeder)
}

#[tokio::test(flavor = "multi_thread")]
async fn downloads_a_torrent_from_a_local_peer() {
    let (_seed_dir, data, torrent, seeder) = seeded(300_000).await;
    // download() paused the seeder after "finishing"; it must share again for this test.
    let resume = seeder.clone();
    let tb = torrent.clone();
    tokio::spawn(async move {
        let (tx, _rx) = watch::channel(Progress::default());
        let _ = resume.share(&Source::File(tb), &tx).await;
    });
    let port = seeder.port().expect("seeder listens");
    let leech_dir = tempfile::tempdir().unwrap();
    let leech = TorrentEngine::start(leech_dir.path(), EngineOptions { local_only: true, peers: vec![([127, 0, 0, 1], port).into()] }).await.unwrap();
    let (tx, rx) = watch::channel(Progress::default());
    let base = leech_dir.path().join("Torrents");
    let got = tokio::time::timeout(Duration::from_secs(60), leech.download(&Source::File(torrent), &base, CancellationToken::new(), &tx)).await.expect("in time");
    let path = match got {
        Ok(Outcome::Completed(p)) => p,
        other => panic!("{other:?}"),
    };
    assert_eq!(path, base.join("hello.bin"));
    assert_eq!(std::fs::read(&path).unwrap(), data);
    assert_eq!(rx.borrow().downloaded, 300_000);
    assert_eq!(rx.borrow().name.as_deref(), Some("hello.bin"));
}
