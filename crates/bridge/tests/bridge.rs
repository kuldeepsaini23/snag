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
    assert_eq!(get(b.port, "/ping", Some(TOKEN)).await.1["paired"], true);
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
