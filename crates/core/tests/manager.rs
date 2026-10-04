mod support;

use rdm_core::{Event, Item, ItemId, Manager, Settings, Status};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use support::{TestServer, data};
use tokio::sync::broadcast;

async fn manager(dir: &Path, tweak: impl FnOnce(&mut Settings)) -> Manager {
    let m = Manager::start(dir.join("state.json"));
    let mut s = m.snapshot().await.settings;
    s.download_dir = dir.join("dl");
    s.sort_into_folders = false;
    tweak(&mut s);
    m.update_settings(s).await;
    m
}

/// Waits for the first item update matching `pred`.
async fn wait_item(rx: &mut broadcast::Receiver<Event>, pred: impl Fn(&Item) -> bool) -> Item {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match rx.recv().await {
                Ok(Event::Updated(item)) if pred(&item) => return item,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(e) => panic!("event channel closed: {e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for item update")
}

fn has(id: ItemId, status: Status) -> impl Fn(&Item) -> bool {
    move |i| i.id == id && i.status == status
}

#[tokio::test]
async fn add_downloads_to_done() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let id = m.add(s.url("/file/300000")).await;
    let item = wait_item(&mut rx, has(id, Status::Done)).await;
    assert_eq!(item.name, "test.bin");
    let dest = item.dest.expect("dest set");
    assert_eq!(dest, dir.path().join("dl").join("test.bin"));
    assert!(std::fs::read(&dest).unwrap() == data(300_000), "content mismatch");
    assert_eq!(item.downloaded, 300_000);
}

#[tokio::test]
async fn sorts_into_category_folder() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.sort_into_folders = true).await;
    let mut rx = m.subscribe();
    let id = m.add(s.url("/named/lecture.mp4/200000")).await;
    let item = wait_item(&mut rx, has(id, Status::Done)).await;
    assert_eq!(item.dest.unwrap(), dir.path().join("dl").join("Videos").join("lecture.mp4"));
}

#[tokio::test]
async fn pause_then_resume_completes() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let size = 4 * 1024 * 1024;
    let id = m.add(s.url(&format!("/slow/{size}"))).await;
    wait_item(&mut rx, |i| i.id == id && i.status == Status::Running && i.downloaded > 0).await;
    m.pause(id).await;
    wait_item(&mut rx, has(id, Status::Paused)).await;
    m.resume(id).await;
    let item = wait_item(&mut rx, has(id, Status::Done)).await;
    assert!(std::fs::read(item.dest.unwrap()).unwrap() == data(size), "content mismatch");
}

#[tokio::test]
async fn respects_max_concurrent() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.max_concurrent = 1).await;
    let mut rx = m.subscribe();
    let a = m.add(s.url("/slow/1048576")).await;
    let b = m.add(s.url("/slow/1048577")).await;
    let mut running = HashSet::new();
    let mut done = HashSet::new();
    let mut peak = 0;
    tokio::time::timeout(Duration::from_secs(30), async {
        while done.len() < 2 {
            if let Ok(Event::Updated(i)) = rx.recv().await {
                if i.status == Status::Running { running.insert(i.id); } else { running.remove(&i.id); }
                if i.status == Status::Done { done.insert(i.id); }
                peak = peak.max(running.len());
            }
        }
    })
    .await
    .expect("both downloads finish");
    assert_eq!(peak, 1, "never more than one running");
    assert_eq!(done, [a, b].into());
}

#[tokio::test]
async fn state_survives_restart() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let size = 4 * 1024 * 1024;
    let id = {
        let m = manager(dir.path(), |_| {}).await;
        let mut rx = m.subscribe();
        let id = m.add(s.url(&format!("/slow/{size}"))).await;
        wait_item(&mut rx, |i| i.id == id && i.status == Status::Running && i.downloaded > 0).await;
        m.shutdown().await;
        id
    };
    let m = Manager::start(dir.path().join("state.json"));
    let item = m.snapshot().await.items.into_iter().find(|i| i.id == id).expect("item persisted");
    assert_eq!(item.status, Status::Paused, "running items come back paused");
    assert!(item.dest.is_some());
    let mut rx = m.subscribe();
    m.resume(id).await;
    let item = wait_item(&mut rx, has(id, Status::Done)).await;
    assert!(std::fs::read(item.dest.unwrap()).unwrap() == data(size), "content mismatch");
}

#[tokio::test]
async fn remove_deletes_partial_files() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let id = m.add(s.url("/slow/4194304")).await;
    wait_item(&mut rx, |i| i.id == id && i.status == Status::Running && i.downloaded > 0).await;
    m.remove(id, true).await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(Event::Removed(r)) = rx.recv().await
                && r == id
            {
                break;
            }
        }
    })
    .await
    .expect("removed");
    assert!(m.snapshot().await.items.is_empty());
    let left: Vec<_> = std::fs::read_dir(dir.path().join("dl")).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert!(left.is_empty(), "leftover files: {left:?}");
}

/// Real network + real yt-dlp/ffmpeg. Run with: cargo test -p rdm-core --test manager -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn real_media_download_mp4_and_mp3() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.sort_into_folders = true).await;
    let url = "https://www.youtube.com/watch?v=aqz-KE-bpKQ".to_string();
    let info = m.probe_media(url.clone()).await.expect("probe");
    println!("title={:?} options={:?}", info.title, info.options.iter().map(|o| &o.label).collect::<Vec<_>>());
    let mut rx = m.subscribe();
    let video = m.add_media(url.clone(), info.title.clone(), rdm_core::MediaFormat::Video { max_height: 360 }).await;
    let audio = m.add_media(url, info.title.clone(), rdm_core::MediaFormat::AudioMp3).await;
    let mut done = std::collections::HashMap::new();
    tokio::time::timeout(Duration::from_secs(300), async {
        while done.len() < 2 {
            if let Ok(Event::Updated(i)) = rx.recv().await {
                match &i.status {
                    Status::Done => { done.insert(i.id, i.dest.clone().unwrap()); }
                    Status::Failed(e) => panic!("{} failed: {e}", i.name),
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("both finish");
    for (id, path) in &done {
        let len = std::fs::metadata(path).unwrap().len();
        println!("{id:?} -> {} ({len} bytes)", path.display());
        assert!(len > 10_000);
    }
    assert_eq!(done[&video].extension().unwrap(), "mp4");
    assert_eq!(done[&audio].extension().unwrap(), "mp3");
    assert!(done[&video].starts_with(dir.path().join("dl").join("Videos")));
    assert!(done[&audio].starts_with(dir.path().join("dl").join("Music")));
}

#[tokio::test]
async fn offer_media_asks_the_ui_to_pick() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let info = rdm_core::MediaInfo { title: "Clip".into(), duration: None, options: vec![], entries: vec![] };
    m.offer_media("https://youtu.be/x".into(), info.clone()).await;
    let got = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(Event::PickMedia { url, info }) = rx.recv().await {
                return (url, info);
            }
        }
    })
    .await
    .expect("pick event");
    assert_eq!(got, ("https://youtu.be/x".to_string(), info));
    assert!(m.snapshot().await.items.is_empty(), "nothing is added until the user picks");
}

#[tokio::test]
async fn fresh_state_gets_extension_token() {
    let dir = tempfile::tempdir().unwrap();
    let m = Manager::start(dir.path().join("state.json"));
    let token = m.snapshot().await.settings.extension_token;
    assert_eq!(token.len(), 32, "token: {token:?}");
}
