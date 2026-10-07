//! Local HTTP bridge for the browser extension: 127.0.0.1 only, token-protected.

pub mod phone;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use rdm_core::route::Route;
use rdm_core::{Cookie, Manager, PairAnswer};
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
                let app = Router::new().route("/ping", get(ping)).route("/add", post(add)).route("/focus", post(focus)).route("/quit", post(quit)).route("/add-batch", post(add_batch)).route("/pair", post(pair)).route("/probe", post(probe)).route("/add-media", post(add_media_choice)).with_state(manager);
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
    let paired = authorized(&manager, &headers).await;
    let mut body = json!({ "app": "rdm", "version": env!("CARGO_PKG_VERSION"), "paired": paired });
    if paired {
        // The extension's button and popup follow Snag's accent colour.
        body["accent"] = json!(manager.snapshot().await.settings.accent);
    }
    Json(body)
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
    /// The best stream the extension saw playing on the page: used if yt-dlp can't read the page.
    #[serde(default)]
    fallback: Option<String>,
}

async fn add(State(manager): State<Manager>, headers: HeaderMap, Json(mut req): Json<AddRequest>) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired: paste the pairing code from Snag settings" })));
    }
    // A GitHub repository's page: its code as a ZIP.
    if let Some(zip) = rdm_core::route::github_zip(&req.url) {
        req.url = zip;
        req.kind = Some("file".into());
    }
    let torrent = rdm_core::is_torrent_link(&req.url);
    if !(req.url.starts_with("http://") || req.url.starts_with("https://") || torrent) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "only http(s) and magnet links can be downloaded" })));
    }
    let cookies: Vec<Cookie> = req.cookies.into_iter().filter_map(|c| serde_json::from_value(c).ok()).collect();
    if !cookies.is_empty() {
        manager.remember_cookies(cookies);
    }
    let referrer = req.referrer.filter(|r| r.starts_with("http"));
    // "Save page with Snag": the page itself, as one offline file.
    if req.kind.as_deref() == Some("page") {
        let id = manager.save_page(req.url).await;
        return (StatusCode::OK, Json(json!({ "id": id.0 })));
    }
    // A file the browser was downloading (already cancelled there) stays a file; anything else
    // goes where its link belongs.
    let route = match (req.kind.as_deref(), rdm_core::route::route(&req.url)) {
        (_, Route::Torrent) => Route::Torrent,
        (Some("file"), _) => Route::File,
        (Some("media"), _) => Route::Media,
        (_, route) => route,
    };
    match route {
        Route::Torrent => {
            let id = manager.add_torrent(req.url).await;
            (StatusCode::OK, Json(json!({ "id": id.0 })))
        }
        Route::Gallery => {
            let id = manager.add_gallery(req.url).await;
            (StatusCode::OK, Json(json!({ "id": id.0 })))
        }
        Route::Media => {
            // A stream found on a page (the extension's media sniffer) needs that page as Referer.
            if let Some(page) = referrer.clone() {
                manager.remember_referrer(req.url.clone(), page);
            }
            // Reading a video page takes a few seconds; answer now, add when ready.
            tokio::spawn(add_media(manager, req.url, req.fallback.filter(|f| f.starts_with("http")), referrer));
            (StatusCode::ACCEPTED, Json(json!({ "queued": true })))
        }
        Route::File => {
            let id = manager.add_with(req.url, referrer).await;
            (StatusCode::OK, Json(json!({ "id": id.0 })))
        }
    }
}

/// Reads the page, then lets the user choose the quality in the app window.
/// Reads the page with yt-dlp. If it can't (a site it doesn't know), falls back to the stream
/// the extension saw playing on the page.
async fn add_media(manager: Manager, url: String, fallback: Option<String>, referrer: Option<String>) {
    match manager.probe_media(url.clone()).await {
        Ok(info) => manager.offer_media(url, info).await,
        // A photo post (no video in it): save its images instead.
        Err(e) if rdm_media::gallery::is_photo_post(&url, &e) => {
            manager.add_gallery(url).await;
        }
        Err(e) => match fallback {
            Some(stream) if rdm_core::route::route(&stream) == Route::File => {
                manager.add_with(stream, referrer).await;
            }
            Some(stream) => {
                if let Some(page) = referrer {
                    manager.remember_referrer(stream.clone(), page);
                }
                match manager.probe_media(stream.clone()).await {
                    Ok(info) => manager.offer_media(stream, info).await,
                    Err(e) => manager.notify(format!("Couldn't read the video on that page: {e}")).await,
                }
            }
            None => manager.notify(format!("Couldn't find a video on that page: {e}")).await,
        },
    }
}

async fn focus(State(manager): State<Manager>, headers: HeaderMap) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired" })));
    }
    manager.focus().await;
    (StatusCode::OK, Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ProbeRequest {
    url: String,
    #[serde(default)]
    referrer: Option<String>,
    #[serde(default)]
    cookies: Vec<Value>,
}

/// Reads a video page for the extension's own quality menu: title, preview, and the options
/// (each with the `format` to send back to `/add-media`).
async fn probe(State(manager): State<Manager>, headers: HeaderMap, Json(req): Json<ProbeRequest>) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired" })));
    }
    let cookies: Vec<Cookie> = req.cookies.into_iter().filter_map(|c| serde_json::from_value(c).ok()).collect();
    if !cookies.is_empty() {
        manager.remember_cookies(cookies);
    }
    if let Some(page) = req.referrer.filter(|r| r.starts_with("http") && *r != req.url) {
        manager.remember_referrer(req.url.clone(), page);
    }
    match manager.probe_media(req.url).await {
        Ok(info) => {
            let options: Vec<Value> = info.options.iter().map(|o| json!({ "label": o.label, "size": o.approx_size, "format": o.format })).collect();
            let body = json!({
                "title": info.title,
                "thumbnail": info.thumbnail,
                "duration": info.duration,
                "live": info.live,
                "playlist": info.entries.len(),
                "options": options,
            });
            (StatusCode::OK, Json(body))
        }
        Err(e) => (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "error": e }))),
    }
}

#[derive(Deserialize)]
struct AddMediaRequest {
    url: String,
    title: String,
    format: rdm_core::MediaFormat,
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    /// The page (or embedded player) the video plays on: sent as Referer.
    #[serde(default)]
    referrer: Option<String>,
    #[serde(default)]
    cookies: Vec<Value>,
}

/// The quality the user picked in the extension: added straight away, no window in between.
async fn add_media_choice(State(manager): State<Manager>, headers: HeaderMap, Json(req): Json<AddMediaRequest>) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired" })));
    }
    let cookies: Vec<Cookie> = req.cookies.into_iter().filter_map(|c| serde_json::from_value(c).ok()).collect();
    if !cookies.is_empty() {
        manager.remember_cookies(cookies);
    }
    if let Some(page) = req.referrer.filter(|r| r.starts_with("http")) {
        manager.remember_referrer(req.url.clone(), page);
    }
    // Only a web picture: never a local or network-share path.
    let thumbnail = req.thumbnail.filter(|t| rdm_core::model::is_web_link(t));
    let id = manager.add_media_meta(req.url, req.title, req.format, 0, thumbnail, req.duration).await;
    (StatusCode::OK, Json(json!({ "id": id.0 })))
}

/// One-click pairing: a browser extension asks, the user allows it in Snag, the extension gets
/// the code. Only extensions can ask (websites send their own origin), and without CORS
/// headers no website could read the answer anyway. The question names the asking extension,
/// and while it is up no other request can replace it.
async fn pair(State(manager): State<Manager>, headers: HeaderMap) -> (StatusCode, Json<Value>) {
    let origin = headers.get("origin").and_then(|v| v.to_str().ok()).unwrap_or("");
    let extension = ["chrome-extension://", "moz-extension://", "safari-web-extension://"].iter().any(|p| origin.starts_with(p));
    if !extension {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "only the Snag browser extension can connect" })));
    }
    match manager.request_pair(origin.to_string()).await {
        PairAnswer::Allowed => {
            let token = manager.snapshot().await.settings.extension_token;
            (StatusCode::OK, Json(json!({ "token": token })))
        }
        PairAnswer::Refused => (StatusCode::FORBIDDEN, Json(json!({ "error": "not allowed in Snag" }))),
        PairAnswer::Busy => (StatusCode::CONFLICT, Json(json!({ "error": "another connection request is waiting in Snag" }))),
    }
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
