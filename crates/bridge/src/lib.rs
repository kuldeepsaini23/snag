//! Local HTTP bridge for the browser extension: 127.0.0.1 only, token-protected.

pub mod phone;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use rdm_core::{Cookie, Manager};
use serde::Deserialize;
use serde_json::{Value, json};
use std::ops::RangeInclusive;

pub const PORTS: RangeInclusive<u16> = 47321..=47326;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bridge {
    pub port: u16,
}

/// Serves on the first free port in `ports` until the runtime shuts down.
pub async fn start(manager: Manager, ports: RangeInclusive<u16>) -> Result<Bridge, String> {
    let mut last_error = String::from("no ports to try");
    for port in ports {
        match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => {
                let app = Router::new().route("/ping", get(ping)).route("/add", post(add)).route("/focus", post(focus)).route("/quit", post(quit)).route("/add-batch", post(add_batch)).with_state(manager);
                tokio::spawn(async move {
                    let _ = axum::serve(listener, app).await;
                });
                return Ok(Bridge { port });
            }
            Err(e) => last_error = format!("127.0.0.1:{port}: {e}"),
        }
    }
    Err(format!("couldn't open the extension bridge ({last_error})"))
}

/// The request carries the pairing token currently in settings (read fresh, so a
/// regenerated token takes effect at once).
async fn authorized(manager: &Manager, headers: &HeaderMap) -> bool {
    let token = manager.snapshot().await.settings.extension_token;
    !token.is_empty() && headers.get("x-rdm-token").and_then(|v| v.to_str().ok()) == Some(token.as_str())
}

async fn ping(State(manager): State<Manager>, headers: HeaderMap) -> Json<Value> {
    Json(json!({ "app": "rdm", "version": env!("CARGO_PKG_VERSION"), "paired": authorized(&manager, &headers).await }))
}

#[derive(Deserialize)]
struct AddRequest {
    url: String,
    /// "file" or "media"; guessed from the URL when absent.
    #[serde(default)]
    kind: Option<String>,
    /// The page the link was found on (sent as Referer).
    #[serde(default)]
    referrer: Option<String>,
    /// The browser's cookies for the link (`chrome.cookies.getAll`), for logged-in downloads.
    /// Entries that don't parse are skipped rather than failing the whole request.
    #[serde(default)]
    cookies: Vec<Value>,
}

async fn add(State(manager): State<Manager>, headers: HeaderMap, Json(req): Json<AddRequest>) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired: paste the pairing code from Snag settings" })));
    }
    let torrent = rdm_core::is_torrent_link(&req.url);
    if !(req.url.starts_with("http://") || req.url.starts_with("https://") || torrent) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "only http(s) and magnet links can be downloaded" })));
    }
    let cookies: Vec<Cookie> = req.cookies.into_iter().filter_map(|c| serde_json::from_value(c).ok()).collect();
    if !cookies.is_empty() {
        manager.remember_cookies(cookies);
    }
    // Magnet links and .torrent files (also .torrent downloads the browser caught) are torrents.
    if torrent {
        let id = manager.add_torrent(req.url).await;
        return (StatusCode::OK, Json(json!({ "id": id.0 })));
    }
    // Page links on image sites go to gallery-dl (they'd otherwise save HTML). A real file the
    // browser was downloading (kind "file": already cancelled there) always stays a file download.
    if req.kind.as_deref() != Some("file") && rdm_media::gallery::is_gallery_url(&req.url) {
        let id = manager.add_gallery(req.url).await;
        return (StatusCode::OK, Json(json!({ "id": id.0 })));
    }
    let media = match req.kind.as_deref() {
        Some("media") => true,
        Some("file") => false,
        _ => rdm_media::is_media_url(&req.url),
    };
    if media {
        // A stream found on a page (the extension's media sniffer) needs that page as Referer.
        if let Some(page) = req.referrer.clone().filter(|r| r.starts_with("http")) {
            manager.remember_referrer(req.url.clone(), page);
        }
        // Reading a video page takes a few seconds; answer now, add when ready.
        tokio::spawn(add_media(manager, req.url));
        return (StatusCode::ACCEPTED, Json(json!({ "queued": true })));
    }
    let id = manager.add_with(req.url, req.referrer.filter(|r| r.starts_with("http"))).await;
    (StatusCode::OK, Json(json!({ "id": id.0 })))
}

/// Reads the page, then lets the user choose the quality in the app window.
async fn add_media(manager: Manager, url: String) {
    match manager.probe_media(url.clone()).await {
        Ok(info) => manager.offer_media(url, info).await,
        // A photo post (no video in it): save its images instead.
        Err(e) if rdm_media::gallery::is_photo_post(&url, &e) => {
            manager.add_gallery(url).await;
        }
        Err(e) => manager.notify(format!("Couldn't read the link from the browser: {e}")).await,
    }
}

async fn focus(State(manager): State<Manager>, headers: HeaderMap) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired" })));
    }
    manager.focus().await;
    (StatusCode::OK, Json(json!({ "ok": true })))
}

/// One request should never add more than this ("Grab all" on a huge page).
const BATCH_MAX: usize = 1000;

#[derive(Deserialize)]
struct BatchRequest {
    urls: Vec<String>,
    #[serde(default)]
    referrer: Option<String>,
    #[serde(default)]
    cookies: Vec<Value>,
}

/// "Grab all" from the extension: direct links from one page, each added as a file download.
async fn add_batch(State(manager): State<Manager>, headers: HeaderMap, Json(req): Json<BatchRequest>) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired: paste the pairing code from Snag settings" })));
    }
    if req.urls.len() > BATCH_MAX {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": format!("at most {BATCH_MAX} links at once") })));
    }
    let cookies: Vec<Cookie> = req.cookies.into_iter().filter_map(|c| serde_json::from_value(c).ok()).collect();
    if !cookies.is_empty() {
        manager.remember_cookies(cookies);
    }
    let referrer = req.referrer.filter(|r| r.starts_with("http"));
    let mut seen = std::collections::HashSet::new();
    let mut added = 0;
    for url in req.urls {
        let web = url.starts_with("http://") || url.starts_with("https://");
        if web && seen.insert(url.clone()) {
            manager.add_with(url, referrer.clone()).await;
            added += 1;
        }
    }
    (StatusCode::OK, Json(json!({ "added": added })))
}

async fn quit(State(manager): State<Manager>, headers: HeaderMap) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired" })));
    }
    manager.request_quit().await;
    (StatusCode::OK, Json(json!({ "ok": true })))
}

/// For a second RDM that is about to quit: asks the running one to show its window.
/// Plain blocking HTTP so it works before any runtime exists. Returns whether one answered.
pub fn focus_running(ports: RangeInclusive<u16>, token: &str) -> bool {
    signal_running(ports, token, "/focus")
}

/// `rdm --quit` / the uninstaller: asks the running RDM to pause, save and quit.
pub fn quit_running(ports: RangeInclusive<u16>, token: &str) -> bool {
    signal_running(ports, token, "/quit")
}

fn signal_running(ports: RangeInclusive<u16>, token: &str, path: &str) -> bool {
    use std::io::{Read, Write};
    use std::time::Duration;
    for port in ports {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let Ok(mut stream) = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300)) else { continue };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-RDM-Token: {token}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        if stream.write_all(request.as_bytes()).is_err() {
            continue;
        }
        let mut reply = String::new();
        let _ = stream.read_to_string(&mut reply);
        if reply.starts_with("HTTP/1.1 200") {
            return true;
        }
    }
    false
}
