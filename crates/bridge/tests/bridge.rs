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
async fn falls_back_to_next_port() {
    let dir = tempfile::tempdir().unwrap();
    let _taken = std::net::TcpListener::bind("127.0.0.1:48141").unwrap();
    let b = start(manager(dir.path()).await, 48141..=48150).await.unwrap();
    assert_eq!(b.port, 48142);
}
