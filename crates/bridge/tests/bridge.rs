use rdm_bridge::start;
use rdm_core::Manager;
use rdm_engine::Client;
use serde_json::{Value, json};
use std::path::Path;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

async fn manager(dir: &Path) -> Manager {
    let m = Manager::start(dir.join("state.json"));
    let mut s = m.snapshot().await.settings;
    s.extension_token = TOKEN.into();
    s.start_immediately = false; // keep the test offline: items are added paused
    s.download_dir = dir.join("dl");
    m.update_settings(s).await;
    m
}

async fn get(port: u16, path: &str, token: Option<&str>) -> (u16, Value) {
    let mut req = Client::new().get(format!("http://127.0.0.1:{port}{path}"));
    if let Some(t) = token {
        req = req.header("X-RDM-Token", t);
    }
    let resp = req.send().await.unwrap();
    let status = resp.status().as_u16();
    (status, serde_json::from_str(&resp.text().await.unwrap()).unwrap_or(Value::Null))
}

async fn post(port: u16, body: Value, token: &str) -> (u16, Value) {
    let resp = Client::new()
        .post(format!("http://127.0.0.1:{port}/add"))
        .header("X-RDM-Token", token)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    (status, serde_json::from_str(&resp.text().await.unwrap()).unwrap_or(Value::Null))
}

#[tokio::test]
async fn ping_reports_pairing() {
    let dir = tempfile::tempdir().unwrap();
    let b = start(manager(dir.path()).await, 48101..=48110).await.unwrap();
    let (status, body) = get(b.port, "/ping", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["app"], "rdm");
    assert_eq!(body["paired"], false);
    let paired = get(b.port, "/ping", Some(TOKEN)).await.1;
    assert_eq!(paired["paired"], true);
    assert_eq!(paired["accent"], "#ff9f0a", "the extension follows Snag's colour");
    assert!(body.get("accent").is_none(), "only paired extensions get the settings");
}

#[tokio::test]
async fn add_with_cookies_and_referrer_reaches_item() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48171..=48180).await.unwrap();
    // Exactly what chrome.cookies.getAll returns (extra fields included), plus one broken entry.
    let cookies = json!([
        {"domain": ".site.test", "hostOnly": false, "path": "/", "secure": true, "httpOnly": true, "sameSite": "lax",
         "session": false, "storeId": "0", "expirationDate": 4000000000.5, "name": "sid", "value": "abc"},
        {"domain": 42}
    ]);
    let body = json!({"url": "https://dl.site.test/private.zip", "kind": "file", "referrer": "https://site.test/page", "cookies": cookies});
    let (status, reply) = post(b.port, body, TOKEN).await;
    assert_eq!(status, 200, "{reply}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].referrer.as_deref(), Some("https://site.test/page"));
}

#[tokio::test]
async fn add_requires_valid_token() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48111..=48120).await.unwrap();
    let (status, _) = post(b.port, json!({"url": "https://example.com/a.zip"}), "wrong").await;
    assert_eq!(status, 401);
    assert!(m.snapshot().await.items.is_empty());
}

#[tokio::test]
async fn add_file_link_creates_item() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48121..=48130).await.unwrap();
    let (status, body) = post(b.port, json!({"url": "https://example.com/setup.exe", "kind": "file"}), TOKEN).await;
    assert_eq!(status, 200, "{body}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].url, "https://example.com/setup.exe");
    assert_eq!(body["id"], items[0].id.0);
}

#[tokio::test]
async fn rejects_non_http_url() {
    let dir = tempfile::tempdir().unwrap();
    let b = start(manager(dir.path()).await, 48131..=48140).await.unwrap();
    assert_eq!(post(b.port, json!({"url": "javascript:alert(1)"}), TOKEN).await.0, 400);
    assert_eq!(post(b.port, json!({"url": "blob:https://x/y"}), TOKEN).await.0, 400);
}

#[tokio::test]
async fn focus_request_brings_the_window_forward() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48151..=48160).await.unwrap();
    let mut rx = m.subscribe();
    let send = |token: &'static str| Client::new().post(format!("http://127.0.0.1:{}/focus", b.port)).header("X-RDM-Token", token).send();
    assert_eq!(send("wrong").await.unwrap().status().as_u16(), 401);
    assert_eq!(send(TOKEN).await.unwrap().status().as_u16(), 200);
    let got = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Ok(rdm_core::Event::Focus) = rx.recv().await {
                return true;
            }
        }
    })
    .await;
    assert_eq!(got, Ok(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn second_instance_can_bring_the_first_forward() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48161..=48170).await.unwrap();
    let mut rx = m.subscribe();
    let port = b.port;
    let answered = tokio::task::spawn_blocking(move || rdm_bridge::focus_running(port..=port, TOKEN)).await.unwrap();
    assert!(answered);
    assert!(matches!(rx.recv().await, Ok(rdm_core::Event::Focus)));
    assert!(!tokio::task::spawn_blocking(move || rdm_bridge::focus_running(port..=port, "wrong")).await.unwrap());
}

#[tokio::test]
async fn falls_back_to_next_port() {
    let dir = tempfile::tempdir().unwrap();
    let _taken = std::net::TcpListener::bind("127.0.0.1:48141").unwrap();
    let b = start(manager(dir.path()).await, 48141..=48150).await.unwrap();
    assert_eq!(b.port, 48142);
}

#[tokio::test(flavor = "multi_thread")]
async fn quit_request_asks_the_running_app_to_quit() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48171..=48180).await.unwrap();
    let mut rx = m.subscribe();
    let port = b.port;
    assert!(!tokio::task::spawn_blocking(move || rdm_bridge::quit_running(port..=port, "wrong")).await.unwrap(), "needs the pairing token");
    let answered = tokio::task::spawn_blocking(move || rdm_bridge::quit_running(port..=port, TOKEN)).await.unwrap();
    assert!(answered);
    assert!(matches!(rx.recv().await, Ok(rdm_core::Event::Quit)));
}

#[tokio::test]
async fn image_page_from_browser_becomes_a_gallery() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48181..=48190).await.unwrap();
    // A page link (right-click, no kind) on an image site goes to gallery-dl.
    let (status, body) = post(b.port, json!({"url": "https://www.pinterest.com/pin/123/"}), TOKEN).await;
    assert_eq!(status, 200, "{body}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, rdm_core::Kind::Gallery);
    assert_eq!(items[0].category, rdm_core::Category::Image);
    // A real file the browser was downloading (the extension already cancelled it there) stays a file.
    let (status, body) = post(b.port, json!({"url": "https://i.imgur.com/archive.zip", "kind": "file"}), TOKEN).await;
    assert_eq!(status, 200, "{body}");
    let items = m.snapshot().await.items;
    assert_eq!(items[1].kind, rdm_core::Kind::Http, "never lost to an extractor that may not know it");
}

#[tokio::test]
async fn add_batch_adds_each_link_as_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48191..=48200).await.unwrap();
    let send = |body: Value, token: &'static str| {
        Client::new()
            .post(format!("http://127.0.0.1:{}/add-batch", b.port))
            .header("X-RDM-Token", token)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
    };
    let body = json!({"urls": ["https://x.test/a.zip", "https://x.test/b.pdf", "ftp://x.test/c", "https://x.test/a.zip"], "referrer": "https://x.test/page"});
    assert_eq!(send(body.clone(), "wrong").await.unwrap().status().as_u16(), 401);
    let resp = send(body, TOKEN).await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let reply: Value = serde_json::from_str(&resp.text().await.unwrap()).unwrap();
    assert_eq!(reply["added"], 2, "web links only, each once: {reply}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.kind == rdm_core::Kind::Http && i.referrer.as_deref() == Some("https://x.test/page")));
    let too_many: Vec<String> = (0..1001).map(|i| format!("https://x.test/{i}.zip")).collect();
    assert_eq!(send(json!({ "urls": too_many }), TOKEN).await.unwrap().status().as_u16(), 400);
}

#[tokio::test]
async fn magnet_and_torrent_links_become_torrents() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48201..=48210).await.unwrap();
    let magnet = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=Ubuntu";
    let (status, body) = post(b.port, json!({ "url": magnet }), TOKEN).await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = post(b.port, json!({ "url": "https://site.org/debian.iso.torrent", "kind": "file" }), TOKEN).await;
    assert_eq!(status, 200, "{body}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.kind == rdm_core::Kind::Torrent), "{items:?}");
    assert_eq!(items[0].name, "Ubuntu");
}

#[tokio::test]
async fn phone_page_needs_the_token_and_adds_shared_links() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    // Bound to localhost here; in the app it binds the home network only while switched on.
    let phone = rdm_bridge::phone::start(m.clone(), ([127, 0, 0, 1], 0).into()).await.unwrap();
    let base = format!("http://127.0.0.1:{}", phone.port);
    let get = |path: String| async move { Client::new().get(path).send().await.unwrap() };
    assert_eq!(get(format!("{base}/m")).await.status().as_u16(), 401, "no token, no page");
    assert_eq!(get(format!("{base}/m?t=wrong")).await.status().as_u16(), 401);
    let page = get(format!("{base}/m?t={TOKEN}")).await;
    assert_eq!(page.status().as_u16(), 200);
    assert!(page.text().await.unwrap().contains("<form"), "a page to paste links into");
    // Shared text from a phone app often wraps the link in words.
    let shared = get(format!("{base}/share?t={TOKEN}&text=Look%20at%20this%20https%3A%2F%2Fx.test%2Fa.zip%20cool")).await;
    assert_eq!(shared.status().as_u16(), 200);
    assert_eq!(get(format!("{base}/share?t=wrong&url=https%3A%2F%2Fx.test%2Fb.zip")).await.status().as_u16(), 401);
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].url, "https://x.test/a.zip");
    phone.stop();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(Client::new().get(format!("{base}/m?t={TOKEN}")).send().await.is_err(), "switched off: nothing listens");
}

#[test]
fn first_link_in_shared_text() {
    use rdm_bridge::phone::first_link;
    assert_eq!(first_link("Watch https://youtu.be/abc?t=1 now"), Some("https://youtu.be/abc?t=1".to_string()));
    assert_eq!(first_link("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x"), Some("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x".to_string()));
    assert_eq!(first_link("(see https://x.test/a.zip)."), Some("https://x.test/a.zip".to_string()), "trailing punctuation dropped");
    assert_eq!(first_link("no links here"), None);
}

#[tokio::test]
async fn extension_pairs_with_one_click_after_the_user_allows() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48211..=48220).await.unwrap();
    let pair = |origin: &'static str| Client::new().post(format!("http://127.0.0.1:{}/pair", b.port)).header("Origin", origin).send();
    // A website can't even ask.
    assert_eq!(pair("https://evil.example").await.unwrap().status().as_u16(), 403);
    // An extension asks; the app shows the question; the user allows.
    let mut rx = m.subscribe();
    let asking = tokio::spawn(pair("chrome-extension://abcdefghijklmnop"));
    let id = loop {
        if let Ok(rdm_core::Event::PairRequest { id, .. }) = rx.recv().await {
            break id;
        }
    };
    m.answer_pair(id, true).await;
    let resp = asking.await.unwrap().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = serde_json::from_str(&resp.text().await.unwrap()).unwrap();
    assert_eq!(body["token"], TOKEN);
    // Denied: no code.
    let asking = tokio::spawn(pair("moz-extension://1234-5678"));
    let id = loop {
        if let Ok(rdm_core::Event::PairRequest { id, .. }) = rx.recv().await {
            break id;
        }
    };
    m.answer_pair(id, false).await;
    let resp = asking.await.unwrap().unwrap();
    assert_eq!(resp.status().as_u16(), 403);
    assert!(!resp.text().await.unwrap().contains(TOKEN));
}

#[tokio::test]
async fn a_second_connection_request_is_refused_while_one_waits() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48371..=48380).await.unwrap();
    let pair = |origin: &'static str| Client::new().post(format!("http://127.0.0.1:{}/pair", b.port)).header("Origin", origin).send();
    let mut rx = m.subscribe();
    let first = tokio::spawn(pair("chrome-extension://abcdefghijklmnop"));
    // The question names who asks, so the user can tell it apart from another extension.
    let (id, origin) = loop {
        if let Ok(rdm_core::Event::PairRequest { id, origin }) = rx.recv().await {
            break (id, origin);
        }
    };
    assert_eq!(origin, "chrome-extension://abcdefghijklmnop");
    // Another caller can't swap its own question in while the user reads this one.
    let second = pair("chrome-extension://zyxwvutsrqponmlk").await.unwrap();
    assert_eq!(second.status().as_u16(), 409);
    assert!(second.text().await.unwrap().contains("another connection request is waiting"));
    while let Ok(event) = rx.try_recv() {
        assert!(!matches!(event, rdm_core::Event::PairRequest { .. }), "only one question at a time");
    }
    m.answer_pair(id, true).await;
    assert_eq!(first.await.unwrap().unwrap().status().as_u16(), 200);
    // Answered: the next request may ask again.
    let again = tokio::spawn(pair("chrome-extension://zyxwvutsrqponmlk"));
    let id = loop {
        if let Ok(rdm_core::Event::PairRequest { id, .. }) = rx.recv().await {
            break id;
        }
    };
    m.answer_pair(id, false).await;
    assert_eq!(again.await.unwrap().unwrap().status().as_u16(), 403);
}

/// The fake yt-dlp from the media crate, as bin/yt-dlp.exe in `data_dir`.
fn install_fake_ytdlp(data_dir: &Path) {
    static FAKE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let fake = FAKE.get_or_init(|| {
        let out = std::env::temp_dir().join(format!("rdm-bridge-fake-ytdlp-{}.exe", std::process::id()));
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../media/tests/fake_ytdlp.rs");
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let status = std::process::Command::new(rustc).args(["--edition", "2024", "-o"]).arg(&out).arg(&src).status().unwrap();
        assert!(status.success(), "couldn't compile the fake yt-dlp");
        out
    });
    std::fs::create_dir_all(data_dir.join("bin")).unwrap();
    std::fs::copy(fake, data_dir.join("bin").join("yt-dlp.exe")).unwrap();
}

#[tokio::test]
async fn unknown_video_site_falls_back_to_the_stream_the_page_played() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48221..=48230).await.unwrap();
    let mut rx = m.subscribe();
    // A page yt-dlp can't read ("fail"), but the extension saw an MP4 playing on it.
    let body = json!({ "url": "https://videos.example/watch/fail", "referrer": "https://videos.example/watch/fail", "fallback": "https://cdn.example/v/movie.mp4" });
    let (status, reply) = post(b.port, body, TOKEN).await;
    assert_eq!(status, 202, "read in the background: {reply}");
    let item = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            if let Ok(rdm_core::Event::Added(i)) = rx.recv().await {
                return i;
            }
        }
    })
    .await
    .expect("the fallback was added");
    assert_eq!(item.url, "https://cdn.example/v/movie.mp4");
    assert_eq!(item.kind, rdm_core::Kind::Http);
    assert_eq!(item.referrer.as_deref(), Some("https://videos.example/watch/fail"));
}

async fn post_to(port: u16, path: &str, body: Value, token: &str) -> (u16, Value) {
    let resp = Client::new()
        .post(format!("http://127.0.0.1:{port}{path}"))
        .header("X-RDM-Token", token)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    (status, serde_json::from_str(&resp.text().await.unwrap()).unwrap_or(Value::Null))
}

#[tokio::test]
async fn extension_picks_the_quality_itself() {
    let dir = tempfile::tempdir().unwrap();
    install_fake_ytdlp(dir.path());
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48231..=48240).await.unwrap();
    // Not paired: nothing.
    assert_eq!(post_to(b.port, "/probe", json!({ "url": "https://videos.example/watch/clip" }), "wrong").await.0, 401);
    let (status, info) = post_to(b.port, "/probe", json!({ "url": "https://videos.example/watch/clip" }), TOKEN).await;
    assert_eq!(status, 200, "{info}");
    assert_eq!(info["title"], "Fake clip");
    let options = info["options"].as_array().expect("options");
    assert!(options.iter().any(|o| o["label"] == "480p"), "{info}");
    let mp3 = options.iter().find(|o| o["label"] == "Audio only (MP3)").expect("mp3 offered");
    // The extension sends back the option it picked.
    let (status, reply) = post_to(b.port, "/add-media", json!({ "url": "https://videos.example/watch/clip", "title": info["title"], "format": mp3["format"] }), TOKEN).await;
    assert_eq!(status, 200, "{reply}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, rdm_core::Kind::Media(rdm_core::MediaFormat::AudioMp3));
    // A page Snag can't read: the error, for the extension to show (and fall back).
    let (status, reply) = post_to(b.port, "/probe", json!({ "url": "https://videos.example/watch/fail" }), TOKEN).await;
    assert_eq!(status, 422);
    assert!(reply["error"].as_str().unwrap().contains("Unsupported URL"), "{reply}");
}

#[tokio::test]
async fn a_caught_stream_starts_at_once_with_its_player_as_referer() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48241..=48250).await.unwrap();
    // The extension's "NOW" row: no reading first, the best quality, the embedded player as Referer.
    let choice = json!({
        "url": "https://cdn.example/hls/master.m3u8?sig=1",
        "title": "Some video",
        "format": { "Video": { "max_height": 4320 } },
        "referrer": "https://player.example/e/42",
    });
    let (status, reply) = post_to(b.port, "/add-media", choice, TOKEN).await;
    assert_eq!(status, 200, "{reply}");
    let items = m.snapshot().await.items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Some video");
    assert_eq!(items[0].referrer.as_deref(), Some("https://player.example/e/42"));
}

#[tokio::test]
async fn a_github_repo_page_downloads_its_code_as_zip() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48251..=48260).await.unwrap();
    let (status, reply) = post(b.port, json!({ "url": "https://github.com/rust-lang/rustlings" }), TOKEN).await;
    assert_eq!(status, 200, "{reply}");
    let items = m.snapshot().await.items;
    assert_eq!(items[0].url, "https://github.com/rust-lang/rustlings/archive/HEAD.zip");
    assert_eq!(items[0].kind, rdm_core::Kind::Http);
}

#[tokio::test]
async fn the_extension_can_save_a_page() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48261..=48270).await.unwrap();
    let (status, reply) = post(b.port, json!({ "url": "https://example.com/article", "kind": "page" }), TOKEN).await;
    assert_eq!(status, 200, "{reply}");
    let items = m.snapshot().await.items;
    assert_eq!(items[0].kind, rdm_core::Kind::Page);
    assert_eq!(items[0].url, "https://example.com/article");
}

#[tokio::test]
async fn only_web_links_reach_the_video_tools() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48271..=48280).await.unwrap();
    for bad in [r"--config-locations=\host\share\cfg", "file:///C:/Windows/win.ini", "-o x"] {
        let (status, _) = post_to(b.port, "/probe", json!({ "url": bad }), TOKEN).await;
        assert_eq!(status, 400, "probe {bad}");
        let (status, _) = post_to(b.port, "/add-media", json!({ "url": bad, "title": "x", "format": "AudioMp3" }), TOKEN).await;
        assert_eq!(status, 400, "add-media {bad}");
    }
    assert!(m.snapshot().await.items.is_empty());
}

#[tokio::test]
async fn only_web_pictures_are_taken_as_thumbnails() {
    let dir = tempfile::tempdir().unwrap();
    let m = manager(dir.path()).await;
    let b = start(m.clone(), 48381..=48390).await.unwrap();
    // A file: "thumbnail" would make Snag read a local file, or a network share (sending the
    // Windows login to whoever runs it).
    let thumbs = [r"file:\\host\share\a.png", "file:///C:/x.png", r"\\host\share\b.png", "https://i.example/ok.jpg"];
    for (n, thumbnail) in thumbs.iter().enumerate() {
        let choice = json!({ "url": format!("https://videos.example/watch/{n}"), "title": "Clip", "format": "AudioMp3", "thumbnail": thumbnail });
        let (status, reply) = post_to(b.port, "/add-media", choice, TOKEN).await;
        assert_eq!(status, 200, "{reply}");
    }
    let items = m.snapshot().await.items;
    let kept: Vec<_> = items.iter().map(|i| i.thumbnail.as_deref()).collect();
    assert_eq!(kept, vec![None, None, None, Some("https://i.example/ok.jpg")]);
}
