//! Local HTTP bridge for the browser extension: 127.0.0.1 only, token-protected.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use rdm_core::{Manager, MediaFormat, MediaInfo};
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
                let app = Router::new().route("/ping", get(ping)).route("/add", post(add)).with_state(manager);
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
}

async fn add(State(manager): State<Manager>, headers: HeaderMap, Json(req): Json<AddRequest>) -> (StatusCode, Json<Value>) {
    if !authorized(&manager, &headers).await {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "not paired: paste the pairing code from RDM settings" })));
    }
    if !(req.url.starts_with("http://") || req.url.starts_with("https://")) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "only http(s) links can be downloaded" })));
    }
    let media = match req.kind.as_deref() {
        Some("media") => true,
        Some("file") => false,
        _ => rdm_media::is_media_url(&req.url),
    };
    if media {
        // Reading a video page takes a few seconds; answer now, add when ready.
        tokio::spawn(add_media(manager, req.url));
        return (StatusCode::ACCEPTED, Json(json!({ "queued": true })));
    }
    let id = manager.add(req.url).await;
    (StatusCode::OK, Json(json!({ "id": id.0 })))
}

async fn add_media(manager: Manager, url: String) {
    let Ok(info) = manager.probe_media(url.clone()).await else { return };
    let format = default_format(&info);
    if info.entries.is_empty() {
        manager.add_media(url, info.title, format).await;
    } else {
        for entry in info.entries {
            manager.add_media(entry.url, entry.title, format.clone()).await;
        }
    }
}

/// Without a picker: best video up to 1080p, else whatever is offered first (MP3 for audio-only links).
fn default_format(info: &MediaInfo) -> MediaFormat {
    info.options
        .iter()
        .map(|o| &o.format)
        .find(|f| matches!(f, MediaFormat::Video { max_height } if *max_height <= 1080))
        .or(info.options.first().map(|o| &o.format))
        .cloned()
        .unwrap_or(MediaFormat::Video { max_height: 1080 })
}
