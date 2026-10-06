mod support;

use rdm_core::{Cookie, Event, Item, ItemId, Kind, Manager, MediaFormat, MediaInfo, Queue, QualityOption, Schedule, Settings, Status};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use support::{TestServer, data, install_fake_ytdlp};
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
    // Partial files went to each item's own temp folder, which is gone now.
    for folder in ["Videos", "Music"] {
        wait_gone(&dir.path().join("dl").join(folder).join(".rdm-parts")).await;
        let names: Vec<_> = std::fs::read_dir(dir.path().join("dl").join(folder)).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 1, "only the finished file is left in {folder}: {names:?}");
    }
}

#[tokio::test]
async fn offer_media_asks_the_ui_to_pick() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let info = rdm_core::MediaInfo { title: "Clip".into(), duration: None, options: vec![], entries: vec![], thumbnail: None };
    m.offer_media("https://youtu.be/x".into(), info.clone()).await;
    let got = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(Event::PickMedia { url, info, .. }) = rx.recv().await {
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
async fn notices_and_focus_requests_reach_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    m.notify("Couldn't read that link".into()).await;
    m.focus().await;
    let mut got = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        while got.len() < 2 {
            match rx.recv().await {
                Ok(Event::Notice(text)) => got.push(text),
                Ok(Event::Focus) => got.push("focus".into()),
                _ => {}
            }
        }
    })
    .await
    .expect("both events");
    assert_eq!(got, vec!["Couldn't read that link".to_string(), "focus".to_string()]);
}

#[tokio::test]
async fn fresh_state_gets_extension_token() {
    let dir = tempfile::tempdir().unwrap();
    let m = Manager::start(dir.path().join("state.json"));
    let token = m.snapshot().await.settings.extension_token;
    assert_eq!(token.len(), 32, "token: {token:?}");
}

#[tokio::test]
async fn redownload_restarts_a_finished_item() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let id = m.add(s.url("/file/300000")).await;
    let first = wait_item(&mut rx, has(id, Status::Done)).await.dest.unwrap();
    std::fs::remove_file(&first).unwrap(); // the user (or something else) deleted the file
    m.redownload(id).await;
    let again = wait_item(&mut rx, has(id, Status::Done)).await;
    assert!(std::fs::read(again.dest.unwrap()).unwrap() == data(300_000), "content mismatch");
}


#[tokio::test]
async fn delete_keeps_a_file_another_item_still_uses() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path().join("video.mp4");
    std::fs::write(&shared, b"the only copy").unwrap();
    let item = |id: u64| {
        serde_json::json!({"id": id, "url": "https://youtu.be/x", "name": "video", "category": "Video", "status": "Done",
            "dest": shared, "downloaded": 13, "total": 13, "queue": 0, "added": 0})
    };
    let state = serde_json::json!({"next_id": 3, "items": [item(1), item(2)]});
    std::fs::write(dir.path().join("state.json"), state.to_string()).unwrap();
    let m = Manager::start(dir.path().join("state.json"));
    let mut rx = m.subscribe();
    m.remove(ItemId(1), true).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while !matches!(rx.recv().await, Ok(Event::Removed(ItemId(1)))) {}
    })
    .await
    .expect("removed");
    assert!(shared.exists(), "item 2 still points at this file");
}

#[tokio::test]
async fn pause_does_not_undo_a_pending_remove() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let id = m.add(s.url("/slow/4194304")).await;
    wait_item(&mut rx, |i| i.id == id && i.status == Status::Running && i.downloaded > 0).await;
    m.remove(id, true).await;
    m.pause(id).await;
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
    .expect("still removed");
    assert!(m.snapshot().await.items.is_empty());
}


/// Waits for the first event `pick` accepts.
async fn wait_event<T>(rx: &mut broadcast::Receiver<Event>, pick: impl Fn(Event) -> Option<T>) -> T {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match rx.recv().await {
                Ok(e) => {
                    if let Some(t) = pick(e) {
                        return t;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(e) => panic!("event channel closed: {e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for event")
}

fn never() -> Schedule {
    Schedule { start: 0, stop: None, days: [false; 7] }
}

fn main_queue() -> Queue {
    Queue { id: 0, name: "Main".into(), max_concurrent: usize::MAX, schedule: None }
}

/// Writes a state.json (downloads into `<dir>/dl`, no category folders) and starts a manager on it.
fn start_with(dir: &Path, items: serde_json::Value, queues: serde_json::Value) -> Manager {
    let state = serde_json::json!({
        "next_id": 10, "items": items, "queues": queues,
        "settings": {"download_dir": dir.join("dl"), "sort_into_folders": false, "extension_token": "t"}
    });
    std::fs::write(dir.join("state.json"), state.to_string()).unwrap();
    Manager::start(dir.join("state.json"))
}

fn media_item(id: u64, url: &str, status: &str, work_dir: Option<&Path>) -> serde_json::Value {
    let mut v = serde_json::json!({"id": id, "url": url, "name": format!("video {id}"), "category": "Video", "status": status,
        "dest": null, "downloaded": 0, "total": null, "queue": 0, "added": 0, "kind": {"Media": {"Video": {"max_height": 480}}}});
    if let Some(w) = work_dir {
        v["work_dir"] = serde_json::json!(w);
    }
    v
}

async fn wait_gone(path: &Path) {
    let gone = tokio::time::timeout(Duration::from_secs(10), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(gone.is_ok(), "{} still exists", path.display());
}

#[tokio::test]
async fn set_queues_keeps_main_and_emits() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let night = Queue { id: 1, name: "Night".into(), max_concurrent: 2, schedule: Some(Schedule { start: 23 * 60, stop: Some(7 * 60), days: [true; 7] }) };
    m.set_queues(vec![night.clone()]).await; // Main left out on purpose
    let queues = wait_event(&mut rx, |e| match e {
        Event::Queues(q) => Some(q),
        _ => None,
    })
    .await;
    assert_eq!(queues, vec![main_queue(), night]);
    assert_eq!(m.snapshot().await.queues, queues);
}

#[tokio::test]
async fn deleting_queue_moves_items_to_main() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let item = serde_json::json!({"id": 1, "url": s.url("/file/200000"), "name": "test.bin", "category": "Other", "status": "Queued",
        "dest": null, "downloaded": 0, "total": null, "queue": 1, "added": 0});
    let queues = serde_json::json!([main_queue(), {"id": 1, "name": "Never", "max_concurrent": 1, "schedule": never()}]);
    let m = start_with(dir.path(), serde_json::json!([item]), queues);
    let mut rx = m.subscribe();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(m.snapshot().await.items[0].status, Status::Queued, "its queue never runs");
    m.set_queues(vec![main_queue()]).await;
    let done = wait_item(&mut rx, has(ItemId(1), Status::Done)).await;
    assert_eq!(done.queue, 0, "moved to Main and downloaded");
}

#[tokio::test]
async fn move_to_queue_and_schedule_holds_it() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.start_immediately = false).await;
    let mut rx = m.subscribe();
    m.set_queues(vec![main_queue(), Queue { id: 1, name: "Never".into(), max_concurrent: 1, schedule: Some(never()) }]).await;
    let id = m.add(s.url("/file/200000")).await;
    m.move_to_queue(id, 99).await; // unknown queue: ignored
    m.move_to_queue(id, 1).await;
    m.resume(id).await;
    wait_item(&mut rx, |i| i.id == id && i.queue == 1 && i.status == Status::Queued).await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(m.snapshot().await.items[0].status, Status::Queued, "held by the queue's schedule");
    m.move_to_queue(id, 0).await;
    wait_item(&mut rx, has(id, Status::Done)).await;
}

fn three_options() -> MediaInfo {
    let video = |h: u32| QualityOption { label: format!("{h}p"), format: MediaFormat::Video { max_height: h }, approx_size: None };
    MediaInfo {
        title: "Clip".into(),
        duration: None,
        options: vec![video(1080), video(720), QualityOption { label: "MP3".into(), format: MediaFormat::AudioMp3, approx_size: None }],
        entries: vec![],
        thumbnail: None,
    }
}

#[tokio::test]
async fn offer_media_asks_with_preferred_choice() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.preferred_quality = Some(MediaFormat::Video { max_height: 720 })).await;
    let mut rx = m.subscribe();
    m.offer_media("https://youtu.be/x".into(), three_options()).await;
    let choice = wait_event(&mut rx, |e| match e {
        Event::PickMedia { choice, .. } => Some(choice),
        _ => None,
    })
    .await;
    assert_eq!(choice, 1, "720p is pre-selected");
}

#[tokio::test]
async fn offer_media_without_asking_adds_items() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| {
        s.ask_quality = false;
        s.start_immediately = false;
        s.preferred_quality = Some(MediaFormat::AudioMp3);
    })
    .await;
    let mut rx = m.subscribe();
    let mut info = three_options();
    info.entries = vec![
        rdm_core::Entry { url: "https://youtu.be/a".into(), title: "A".into(), thumbnail: None, duration: None },
        rdm_core::Entry { url: "https://youtu.be/b".into(), title: "B".into(), thumbnail: None, duration: None },
    ];
    m.offer_media("https://youtube.com/playlist?list=1".into(), info).await;
    wait_event(&mut rx, |e| matches!(e, Event::Notice(_)).then_some(())).await;
    let items = m.snapshot().await.items;
    let got: Vec<_> = items.iter().map(|i| (i.url.as_str(), i.name.as_str(), &i.kind)).collect();
    let mp3 = Kind::Media(MediaFormat::AudioMp3);
    assert_eq!(got, vec![("https://youtu.be/a", "A", &mp3), ("https://youtu.be/b", "B", &mp3)]);
}

#[tokio::test]
async fn old_media_item_without_work_dir_loads() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = start_with(dir.path(), serde_json::json!([media_item(1, "fake://ok", "Paused", None)]), serde_json::json!([main_queue()]));
    let mut rx = m.subscribe();
    assert_eq!(m.snapshot().await.items.len(), 1, "an item from before work_dir existed still loads");
    m.resume(ItemId(1)).await;
    wait_item(&mut rx, has(ItemId(1), Status::Done)).await;
    let parts = dir.path().join("dl").join(".rdm-parts");
    let args = std::fs::read_to_string(dir.path().join("dl").join("fake-args.txt")).unwrap();
    assert!(args.contains(&format!("temp:{}", parts.join("1").display())), "{args}");
    wait_gone(&parts).await; // finished: its partial files are cleaned up
}

#[tokio::test]
async fn removing_media_item_deletes_its_parts_only() {
    let dir = tempfile::tempdir().unwrap();
    let dl = dir.path().join("dl");
    let (one, two) = (dl.join(".rdm-parts").join("1"), dl.join(".rdm-parts").join("2"));
    for d in [&one, &two] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("v.f137.mp4.part"), b"partial").unwrap();
    }
    std::fs::write(dl.join("done.mp4"), b"finished video").unwrap();
    let mut done = media_item(3, "https://youtu.be/c", "Done", None);
    done["dest"] = serde_json::json!(dl.join("done.mp4"));
    let items = serde_json::json!([media_item(1, "https://youtu.be/a", "Paused", Some(&one)), media_item(2, "https://youtu.be/b", "Paused", Some(&two)), done]);
    let m = start_with(dir.path(), items, serde_json::json!([main_queue()]));
    let mut rx = m.subscribe();
    m.remove(ItemId(1), false).await;
    wait_event(&mut rx, |e| matches!(e, Event::Removed(ItemId(1))).then_some(())).await;
    wait_gone(&one).await;
    assert!(two.join("v.f137.mp4.part").exists(), "another item's parts stay");
    assert!(dl.join("done.mp4").exists(), "finished videos stay");
}

#[tokio::test]
async fn http_item_sends_browser_cookies_and_referrer() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    m.remember_cookies(vec![Cookie {
        domain: "127.0.0.1".into(),
        host_only: true,
        path: "/".into(),
        secure: false,
        expiration_date: None,
        name: "sid".into(),
        value: "1".into(),
    }]);
    let id = m.add_with(s.url("/needs-cookie/300000"), Some("https://site.test/page".into())).await;
    let item = wait_item(&mut rx, |i| i.id == id && matches!(i.status, Status::Done | Status::Failed(_))).await;
    assert_eq!(item.status, Status::Done);
    assert_eq!(item.referrer.as_deref(), Some("https://site.test/page"));
    assert!(std::fs::read(item.dest.unwrap()).unwrap() == data(300_000), "content mismatch");
}

#[tokio::test]
async fn media_limit_and_cookies_reach_ytdlp() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path(), |s| s.speed_limit_bps = 100 * 1024).await;
    let mut rx = m.subscribe();
    m.remember_cookies(vec![Cookie {
        domain: ".video.test".into(),
        host_only: false,
        path: "/".into(),
        secure: true,
        expiration_date: None,
        name: "login".into(),
        value: "yes".into(),
    }]);
    let id = m.add_media("https://www.video.test/ok".into(), "Clip".into(), MediaFormat::Video { max_height: 480 }).await;
    wait_item(&mut rx, has(id, Status::Done)).await;
    let args = std::fs::read_to_string(dir.path().join("dl").join("fake-args.txt")).unwrap();
    assert!(args.contains("--limit-rate\n102400"), "{args}");
    assert!(args.contains(".video.test\tTRUE\t/\tTRUE\t0\tlogin\tyes"), "{args}");
    let cookies = dir.path().join("cookies");
    let left = std::fs::read_dir(&cookies).map(|d| d.count()).unwrap_or(0);
    assert_eq!(left, 0, "cookie files are deleted once yt-dlp is done");
}

#[tokio::test]
async fn update_ytdlp_reports_what_it_said() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path(), |_| {}).await;
    assert_eq!(m.update_ytdlp().await, Ok("yt-dlp is up to date (fake)".to_string()));
    assert!(dir.path().join("bin").join("yt-dlp.checked").exists(), "the weekly check starts over");
}


#[tokio::test]
async fn leftover_cookie_files_are_wiped_at_start() {
    let dir = tempfile::tempdir().unwrap();
    let cookies = dir.path().join("cookies");
    std::fs::create_dir_all(&cookies).unwrap();
    std::fs::write(cookies.join("probe-3.txt"), "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t0\tSID\tsecret").unwrap();
    let m = Manager::start(dir.path().join("state.json"));
    m.snapshot().await;
    let left = std::fs::read_dir(&cookies).map(|d| d.count()).unwrap_or(0);
    assert_eq!(left, 0, "cookies from a crashed or killed run must not stay on disk");
}

/// The --limit-rate each running video's yt-dlp got (read from the fake's record in its temp folder).
fn video_limits(dl: &Path) -> Vec<u64> {
    let Ok(dirs) = std::fs::read_dir(dl.join(".rdm-parts")) else { return Vec::new() };
    dirs.filter_map(|d| std::fs::read_to_string(d.ok()?.path().join("fake-args.txt")).ok())
        .filter_map(|args| {
            let lines: Vec<&str> = args.lines().collect();
            let i = lines.iter().position(|l| *l == "--limit-rate")?;
            lines.get(i + 1)?.parse().ok()
        })
        .collect()
}

#[tokio::test]
async fn videos_together_stay_within_the_speed_limit() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let limit = 300 * 1024;
    let m = manager(dir.path(), |s| {
        s.speed_limit_bps = limit;
        s.max_concurrent = 3;
    })
    .await;
    let mut rx = m.subscribe();
    let mut ids = Vec::new();
    for n in 0..3 {
        ids.push(m.add_media(format!("https://v.test/{n}/slow"), format!("clip {n}"), MediaFormat::Video { max_height: 480 }).await);
    }
    for id in &ids {
        wait_item(&mut rx, |i| i.id == *id && i.status == Status::Running).await;
    }
    // Earlier starts are restarted with a smaller share as more videos join.
    let settled = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let limits = video_limits(&dir.path().join("dl"));
            if limits.len() == 3 && limits.iter().sum::<u64>() <= limit {
                return limits;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await;
    assert!(settled.is_ok(), "three videos together exceed {limit} B/s: {:?}", video_limits(&dir.path().join("dl")));
    m.shutdown().await;
}

#[tokio::test]
async fn add_media_to_scheduled_queue_waits() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    m.set_queues(vec![main_queue(), Queue { id: 1, name: "Never".into(), max_concurrent: 1, schedule: Some(never()) }]).await;
    let id = m.add_media_to("https://v.test/1/ok".into(), "Clip".into(), MediaFormat::Video { max_height: 480 }, 1).await;
    let item = wait_event(&mut rx, |e| match e {
        Event::Added(i) if i.id == id => Some(i),
        _ => None,
    })
    .await;
    assert_eq!(item.queue, 1, "placed in its queue before it could start in Main");
    tokio::time::sleep(Duration::from_millis(600)).await;
    let snap = m.snapshot().await;
    assert_eq!(snap.items[0].status, Status::Queued, "held by the queue's schedule");
    // Unknown queue: falls back to Main.
    let other = m.add_media_to("https://v.test/2/ok".into(), "Clip 2".into(), MediaFormat::AudioMp3, 42).await;
    assert_eq!(m.snapshot().await.item(other).unwrap().queue, 0);
    m.shutdown().await;
}

#[tokio::test]
async fn gallery_item_saves_all_images_to_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let id = m.add_gallery("https://www.pinterest.com/pin/ok".into()).await;
    let done = wait_item(&mut rx, has(id, Status::Done)).await;
    assert_eq!(done.category, rdm_core::Category::Image);
    assert_eq!(done.kind, rdm_core::Kind::Gallery);
    let name = rdm_core::model::gallery_name("https://www.pinterest.com/pin/ok");
    assert!(name.starts_with("pinterest.com · ok · "), "{name}");
    assert_eq!(done.name, name);
    let folder = done.dest.clone().expect("the gallery's folder");
    assert_eq!(folder, dir.path().join("dl").join(&name), "sorting is off in these tests");
    assert!(folder.join("3.jpg").exists());
    assert_eq!(done.downloaded, 350, "the folder's real size");
    m.shutdown().await;
}

#[tokio::test]
async fn video_item_keeps_thumbnail_and_duration() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.start_immediately = false).await;
    let id = m.add_media_meta("https://y/1".into(), "Clip".into(), MediaFormat::Video { max_height: 720 }, 0, Some("https://i/1.jpg".into()), Some(61.0)).await;
    let item = m.snapshot().await.items.into_iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.thumbnail.as_deref(), Some("https://i/1.jpg"));
    assert_eq!(item.duration, Some(61.0));
    m.shutdown().await;
    // Saved with the item.
    let again = Manager::start(dir.path().join("state.json"));
    let item = again.snapshot().await.items.into_iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.thumbnail.as_deref(), Some("https://i/1.jpg"));
    again.shutdown().await;
}

#[tokio::test]
async fn refresh_link_continues_with_the_new_url() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    let id = m.add(s.url("/no-such-route")).await;
    wait_item(&mut rx, |i| i.id == id && matches!(i.status, Status::Failed(_))).await;
    m.refresh_url(id, s.url("/file/200000")).await;
    let done = wait_item(&mut rx, has(id, Status::Done)).await;
    assert_eq!(done.url, s.url("/file/200000"));
    assert_eq!(done.downloaded, 200000);
    m.shutdown().await;
}

#[tokio::test]
async fn refresh_link_ignores_bad_links_and_running_items() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path(), |s| s.start_immediately = false).await;
    let id = m.add(s.url("/file/1000")).await;
    m.refresh_url(id, "ftp://nope".into()).await;
    assert_eq!(m.snapshot().await.items[0].url, s.url("/file/1000"), "only http(s) links");
    m.shutdown().await;
}

#[tokio::test]
async fn subtitle_setting_reaches_ytdlp() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path(), |s| {
        s.subtitles = true;
        s.subtitle_langs = "en.*,hi".into();
    })
    .await;
    let mut rx = m.subscribe();
    let id = m.add_media("https://www.video.test/ok".into(), "Clip".into(), MediaFormat::Video { max_height: 480 }).await;
    wait_item(&mut rx, has(id, Status::Done)).await;
    let args = std::fs::read_to_string(dir.path().join("dl").join("fake-args.txt")).unwrap();
    assert!(args.contains("--sub-langs\nen.*,hi") && args.contains("--embed-subs"), "{args}");
    m.shutdown().await;
}

#[tokio::test]
async fn temporary_failures_retry_by_themselves() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = Manager::start_with_retry(dir.path().join("state.json"), Duration::from_millis(50));
    let mut s = m.snapshot().await.settings;
    s.download_dir = dir.path().join("dl");
    s.sort_into_folders = false;
    m.update_settings(s).await;
    let mut rx = m.subscribe();
    let id = m.add_media("https://www.video.test/flaky".into(), "Clip".into(), MediaFormat::Video { max_height: 480 }).await;
    let failed = wait_item(&mut rx, |i| i.id == id && matches!(i.status, Status::Failed(_))).await;
    assert!(failed.retry_at.is_some(), "a retry is pending: {failed:?}");
    assert!(matches!(&failed.status, Status::Failed(e) if !e.contains("retrying")), "the reason stays clean (the UI shows the countdown)");
    wait_item(&mut rx, has(id, Status::Done)).await;
    m.shutdown().await;
}

#[tokio::test]
async fn permanent_failures_and_retry_off_stay_failed() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = Manager::start_with_retry(dir.path().join("state.json"), Duration::from_millis(50));
    let mut s = m.snapshot().await.settings;
    s.download_dir = dir.path().join("dl");
    m.update_settings(s).await;
    let mut rx = m.subscribe();
    let id = m.add_media("https://www.video.test/fail".into(), "Clip".into(), MediaFormat::AudioMp3).await;
    let failed = wait_item(&mut rx, |i| i.id == id && matches!(i.status, Status::Failed(_))).await;
    assert_eq!(failed.status, Status::Failed("Unsupported URL: fake://fail".into()), "no retry for an unsupported link");
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(matches!(m.snapshot().await.items[0].status, Status::Failed(_)));
    m.shutdown().await;
}

#[test]
fn old_rdm_folder_moves_to_snag_once() {
    let appdata = tempfile::tempdir().unwrap();
    let old = appdata.path().join("rdm");
    std::fs::create_dir_all(old.join("bin")).unwrap();
    std::fs::write(old.join("state.json"), "{}").unwrap();
    let dir = rdm_core::store::data_dir(appdata.path());
    assert_eq!(dir, appdata.path().join("Snag"));
    assert!(dir.join("state.json").exists() && dir.join("bin").exists(), "everything moved");
    assert!(!old.exists());
    // Already moved: nothing happens.
    assert_eq!(rdm_core::store::data_dir(appdata.path()), dir);
    // Fresh install: the new folder.
    let fresh = tempfile::tempdir().unwrap();
    assert_eq!(rdm_core::store::data_dir(fresh.path()), fresh.path().join("Snag"));
}

#[test]
fn old_rdm_still_running_keeps_its_folder() {
    let appdata = tempfile::tempdir().unwrap();
    let old = appdata.path().join("rdm");
    std::fs::create_dir_all(&old).unwrap();
    let _running = rdm_core::instance::lock(&old).expect("the old app holds its folder");
    assert_eq!(rdm_core::store::data_dir(appdata.path()), old, "not moved under a running copy");
    assert!(old.exists());
}


#[tokio::test]
async fn pause_stops_a_pending_retry() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = Manager::start_with_retry(dir.path().join("state.json"), Duration::from_millis(300));
    let mut s = m.snapshot().await.settings;
    s.download_dir = dir.path().join("dl");
    m.update_settings(s).await;
    let mut rx = m.subscribe();
    let id = m.add_media("https://www.video.test/busy".into(), "Clip".into(), MediaFormat::AudioMp3).await;
    wait_item(&mut rx, |i| i.id == id && i.retry_at.is_some()).await;
    m.pause(id).await;
    let paused = wait_item(&mut rx, has(id, Status::Paused)).await;
    assert_eq!(paused.retry_at, None);
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(m.snapshot().await.items[0].status, Status::Paused, "the old timer doesn't bring it back");
    m.shutdown().await;
}

#[test]
fn gallery_folders_never_collide() {
    use rdm_core::model::gallery_name;
    let a = gallery_name("https://x.com/alice/status/111/photo/1");
    let b = gallery_name("https://x.com/bob/status/222/photo/1");
    assert_ne!(a, b, "different posts, different folders");
    assert!(a.starts_with("x.com · 111"), "the post id, not 'photo/1': {a}");
    let p1 = gallery_name("https://www.pinterest.com/alice/wallpapers/");
    let p2 = gallery_name("https://www.pinterest.com/bob/wallpapers/");
    assert_ne!(p1, p2);
    assert_eq!(gallery_name("https://imgur.com/a/xyz"), gallery_name("https://imgur.com/a/xyz?utm=1"), "same page, same folder");
    let long = gallery_name(&format!("https://imgur.com/a/{}", "x".repeat(300)));
    assert!(long.chars().count() <= 80, "{}", long.len());
    assert!(!gallery_name("https://imgur.com/a/b:c*d").contains([':', '*']));
}

#[tokio::test]
async fn media_link_from_a_page_keeps_its_referer() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path(), |_| {}).await;
    let mut rx = m.subscribe();
    m.remember_referrer("https://cdn.tv/v/ok".into(), "https://site.tv/watch/1".into());
    let id = m.add_media("https://cdn.tv/v/ok".into(), "Stream".into(), MediaFormat::Video { max_height: 480 }).await;
    let done = wait_item(&mut rx, has(id, Status::Done)).await;
    assert_eq!(done.referrer.as_deref(), Some("https://site.tv/watch/1"));
    let args = std::fs::read_to_string(dir.path().join("dl").join("fake-args.txt")).unwrap();
    assert!(args.contains("--referer\nhttps://site.tv/watch/1"), "{args}");
    m.shutdown().await;
}
