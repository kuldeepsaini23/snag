#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};
use futures_util::StreamExt;
use std::time::Duration;

const CHUNK: usize = 16 * 1024;

pub fn data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 251) as u8).collect()
}

const PAGE: &str = "<!doctype html><html><head><title>Rust: Saved Page</title><link rel=stylesheet href=/style.css></head><body><h1>Hello</h1><img src=/dot.png></body></html>";

/// A 1×1 PNG.
const DOT_PNG: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
    0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

pub struct TestServer {
    pub base: String,
}

impl TestServer {
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/file/{size}", get(|Path(size): Path<usize>, h: HeaderMap| async move { serve(&h, size, "test.bin", None) }))
            .route("/slow/{size}", get(|Path(size): Path<usize>, h: HeaderMap| async move { serve(&h, size, "test.bin", Some(Duration::from_millis(10))) }))
            .route("/named/{name}/{size}", get(|Path((name, size)): Path<(String, usize)>, h: HeaderMap| async move { serve(&h, size, &name, None) }))
            .route("/needs-cookie/{size}", get(|Path(size): Path<usize>, h: HeaderMap| async move { needs_cookie(&h, size) }))
            // A small web page with a stylesheet and an image, for "save page".
            .route("/page", get(|| async { ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], PAGE) }))
            .route("/style.css", get(|| async { ([(header::CONTENT_TYPE, "text/css")], "body { color: rgb(1, 2, 3); }") }))
            .route("/dot.png", get(|| async { ([(header::CONTENT_TYPE, "image/png")], DOT_PNG.to_vec()) }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { base: format!("http://{addr}") }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}

/// A logged-in download: every request must carry the browser's cookie and referrer.
fn needs_cookie(headers: &HeaderMap, size: usize) -> Response {
    let has = |name: header::HeaderName, value: &str| headers.get(name).and_then(|v| v.to_str().ok()) == Some(value);
    if !(has(header::COOKIE, "sid=1") && has(header::REFERER, "https://site.test/page")) {
        return Response::builder().status(StatusCode::FORBIDDEN).body(Body::empty()).unwrap();
    }
    serve(headers, size, "private.bin", None)
}

/// Puts the test stand-in for yt-dlp where the manager looks for it (`<data dir>/bin/yt-dlp.exe`).
/// It is std-only, so it is compiled straight with rustc once per test run.
pub fn install_fake_ytdlp(data_dir: &std::path::Path) {
    static FAKE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let fake = FAKE.get_or_init(|| {
        let out = std::env::temp_dir().join(format!("rdm-core-fake-ytdlp-{}.exe", std::process::id()));
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../media/tests/fake_ytdlp.rs");
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let status = std::process::Command::new(rustc).args(["--edition", "2024", "-o"]).arg(&out).arg(&src).status().unwrap();
        assert!(status.success(), "couldn't compile the fake yt-dlp");
        out
    });
    let bin = data_dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::copy(fake, bin.join("yt-dlp.exe")).unwrap();
    // A stand-in ffmpeg, so tests never fetch the real one (yt-dlp here is fake anyway).
    std::fs::write(bin.join("ffmpeg.exe"), b"").unwrap();
    // The same fake plays gallery-dl (it switches on the `-D <dir>` argument).
    std::fs::copy(fake, bin.join("gallery-dl.exe")).unwrap();
}

/// Range-capable file of `size` deterministic bytes, optionally slowed per 16 KiB chunk.
fn serve(headers: &HeaderMap, size: usize, name: &str, delay: Option<Duration>) -> Response {
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("bytes="))
        .and_then(|v| v.split_once('-'))
        .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, if b.is_empty() { size } else { b.parse::<usize>().ok()? + 1 })));
    let (status, start, end) = match range {
        Some((s, e)) => (StatusCode::PARTIAL_CONTENT, s, e.min(size)),
        None => (StatusCode::OK, 0, size),
    };
    let chunks: Vec<Vec<u8>> = data(size)[start..end].chunks(CHUNK).map(<[u8]>::to_vec).collect();
    let stream = futures_util::stream::iter(chunks).then(move |c| async move {
        if let Some(d) = delay {
            tokio::time::sleep(d).await;
        }
        Ok::<_, std::io::Error>(c)
    });
    let mut b = Response::builder()
        .status(status)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, "\"v1\"")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\""))
        .header(header::CONTENT_LENGTH, end - start);
    if status == StatusCode::PARTIAL_CONTENT {
        b = b.header(header::CONTENT_RANGE, format!("bytes {start}-{}/{size}", end - 1));
    }
    b.body(Body::from_stream(stream)).unwrap()
}
