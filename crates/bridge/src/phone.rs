//! "Send from your phone": a tiny page on the home network, only while the user switches it
//! on, and only with the pairing code in the link (the QR code in Settings carries it).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use rdm_core::Manager;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::sync::oneshot;

/// The phone server; dropping or `stop()` closes it.
pub struct Phone {
    pub port: u16,
    stop: Option<oneshot::Sender<()>>,
}

impl Phone {
    pub fn stop(mut self) {
        if let Some(tx) = self.stop.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for Phone {
    fn drop(&mut self) {
        if let Some(tx) = self.stop.take() {
            let _ = tx.send(());
        }
    }
}

/// Serves the phone page on `addr` (port 0 = any free port).
pub async fn start(manager: Manager, addr: SocketAddr) -> Result<Phone, String> {
    start_on(&tokio::runtime::Handle::current(), manager, addr)
}

/// `start`, callable from any thread: binds now, serves on `runtime`.
pub fn start_on(runtime: &tokio::runtime::Handle, manager: Manager, addr: SocketAddr) -> Result<Phone, String> {
    let std_listener = std::net::TcpListener::bind(addr).map_err(|e| format!("couldn't open phone sharing on {addr}: {e}"))?;
    std_listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = std_listener.local_addr().map_err(|e| e.to_string())?.port();
    let app = Router::new().route("/m", get(page)).route("/share", get(share)).with_state(manager);
    let (tx, rx) = oneshot::channel::<()>();
    runtime.spawn(async move {
        let Ok(listener) = tokio::net::TcpListener::from_std(std_listener) else { return };
        let _ = axum::serve(listener, app).with_graceful_shutdown(async { let _ = rx.await; }).await;
    });
    Ok(Phone { port, stop: Some(tx) })
}

/// The phone page link with the pairing code (what the QR code holds).
pub fn page_url(ip: std::net::IpAddr, port: u16, token: &str) -> String {
    format!("http://{ip}:{port}/m?t={token}")
}

/// Ports tried on the home network, so the phone's bookmark keeps working.
pub const PORTS: std::ops::RangeInclusive<u16> = 47330..=47335;

/// This PC's address on the home network (no packet is sent: it only asks the routing table).
pub fn lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.168.0.1", 9)).ok()?;
    socket.local_addr().ok().map(|a| a.ip()).filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
}

/// The first web or magnet link in text shared from a phone app.
pub fn first_link(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|w| w.trim_start_matches(['(', '[', '<', '"', '\'']))
        .find(|w| w.starts_with("http://") || w.starts_with("https://") || w.starts_with("magnet:?"))
        .map(|w| w.trim_end_matches(['.', ',', ')', ']', '>', '"', '\'', '!', '?', ';']).to_string())
}

#[derive(Deserialize)]
struct Params {
    #[serde(default)]
    t: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    text: String,
}

async fn token_ok(manager: &Manager, given: &str) -> bool {
    let token = manager.settings().await.phone_token;
    !token.is_empty() && given == token
}

const STYLE: &str = "body{margin:0;font:16px/1.4 system-ui,sans-serif;background:#1a1816;color:#ffffffe5;padding:24px}\
h1{font-size:20px}input{width:100%;box-sizing:border-box;font-size:16px;padding:12px;border-radius:10px;border:1px solid #ffffff26;background:#211f1c;color:#fff}\
button{margin-top:12px;width:100%;padding:14px;font-size:16px;font-weight:600;border:0;border-radius:10px;background:#ff9f0a;color:#1a1816}\
p{color:#ffffffa6}";

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

async fn page(State(manager): State<Manager>, Query(p): Query<Params>) -> (StatusCode, Html<String>) {
    if !token_ok(&manager, &p.t).await {
        return (StatusCode::UNAUTHORIZED, Html("<h1>Scan the QR code in Snag → Settings → Extension</h1>".into()));
    }
    let token = escape(&p.t);
    (
        StatusCode::OK,
        Html(format!(
            "<!doctype html><html><head><meta charset=utf-8><meta name=viewport content='width=device-width,initial-scale=1'><title>Snag</title><style>{STYLE}</style></head>\
<body><h1>Send to Snag</h1><p>Paste a link (video, file, torrent…). It downloads on your PC.</p>\
<form action=/share method=get><input type=hidden name=t value=\"{token}\"><input name=text placeholder='Paste a link' autofocus>\
<button>Download on PC</button></form><p>Tip: add this page to your home screen.</p></body></html>"
        )),
    )
}

async fn share(State(manager): State<Manager>, Query(p): Query<Params>) -> (StatusCode, Html<String>) {
    if !token_ok(&manager, &p.t).await {
        return (StatusCode::UNAUTHORIZED, Html("<h1>Not paired</h1>".into()));
    }
    let Some(link) = first_link(&p.url).or_else(|| first_link(&p.text)) else {
        return (StatusCode::BAD_REQUEST, Html(format!("<style>{STYLE}</style><h1>No link found</h1><p><a href='/m?t={}'>Back</a></p>", escape(&p.t))));
    };
    if rdm_core::is_torrent_link(&link) {
        manager.add_torrent(link.clone()).await;
    } else if rdm_media::is_media_url(&link) {
        let m = manager.clone();
        let url = link.clone();
        tokio::spawn(async move {
            match m.probe_media(url.clone()).await {
                Ok(info) => m.offer_media(url, info).await,
                Err(e) => m.notify(format!("Couldn't read the link from your phone: {e}")).await,
            }
        });
    } else {
        manager.add(link.clone()).await;
    }
    (
        StatusCode::OK,
        Html(format!(
            "<!doctype html><meta name=viewport content='width=device-width,initial-scale=1'><style>{STYLE}</style><h1>Sent to Snag ✓</h1><p>{}</p><p><a style='color:#ff9f0a' href='/m?t={}'>Send another</a></p>",
            escape(&link),
            escape(&p.t)
        )),
    )
}
