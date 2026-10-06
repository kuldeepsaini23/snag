//! Torrents through the manager. Its own test binary: the torrent engine's threads must not
//! slow down the timing-sensitive tests in manager.rs.

use rdm_core::{Manager, Status};
use std::time::Duration;
use tokio::sync::broadcast;

async fn manager(dir: &std::path::Path) -> Manager {
    let m = Manager::start(dir.join("state.json"));
    let mut s = m.snapshot().await.settings;
    s.download_dir = dir.join("dl");
    s.sort_into_folders = false;
    m.update_settings(s).await;
    m
}

async fn wait_done(rx: &mut broadcast::Receiver<rdm_core::Event>, id: rdm_core::ItemId) -> rdm_core::Item {
    loop {
        if let Ok(rdm_core::Event::Updated(i)) = rx.recv().await
            && i.id == id
            && i.status == Status::Done
        {
            return i;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn magnet_link_downloads_from_a_peer() {
    use rdm_torrent::{EngineOptions, Progress as TorrentProgress, Source, TorrentEngine};
    // A seeder on this machine with a 200 KB file.
    let seed_dir = tempfile::tempdir().unwrap();
    let data: Vec<u8> = (0..200_000).map(|i| (i * 7 % 253) as u8).collect();
    let file = seed_dir.path().join("movie.mkv");
    std::fs::write(&file, &data).unwrap();
    let spawner = librqbit::spawn_utils::BlockingSpawner::new(2);
    let created = librqbit::create_torrent(&file, librqbit::CreateTorrentOptions { name: None, trackers: vec![], piece_length: Some(16384) }, &spawner).await.unwrap();
    let torrent = created.as_bytes().unwrap().to_vec();
    let seeder = TorrentEngine::start(seed_dir.path(), EngineOptions { local_only: true, peers: vec![] }).await.unwrap();
    let (tx, _rx) = tokio::sync::watch::channel(TorrentProgress::default());
    seeder.share(&Source::File(torrent), &tx).await.unwrap();
    let peer: std::net::SocketAddr = ([127, 0, 0, 1], seeder.port().unwrap()).into();

    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    m.use_torrent_peers(vec![peer]);
    let mut rx = m.subscribe();
    let magnet = format!("magnet:?xt=urn:btih:{}&dn=movie.mkv", created.info_hash().as_string());
    let id = m.add_torrent(magnet).await;
    let added = m.snapshot().await.items.into_iter().find(|i| i.id == id).unwrap();
    assert_eq!(added.kind, rdm_core::Kind::Torrent);
    assert_eq!(added.name, "movie.mkv", "named from the magnet link straight away");
    let done = tokio::time::timeout(Duration::from_secs(90), wait_done(&mut rx, id)).await.expect("in time");
    let dest = done.dest.expect("the file");
    assert_eq!(dest, dir.path().join("dl").join("movie.mkv"), "sorting is off in these tests");
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    assert_eq!(done.category, rdm_core::Category::Video);
    m.shutdown().await;
}
