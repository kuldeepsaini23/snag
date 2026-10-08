#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};
use futures_util::StreamExt;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

const CHUNK: usize = 16 * 1024;

pub fn data(size: usize) -> Vec<u8> {
    data_with_seed(size, 0)
}

pub fn data_with_seed(size: usize, seed: usize) -> Vec<u8> {
    (0..size).map(|i| ((i + seed) % 251) as u8).collect()
}

pub fn seed_for(tag: &str) -> usize {
    tag.bytes().map(usize::from).sum()
}

#[derive(Clone, Default)]
struct Counters {
    flaky: Arc<AtomicUsize>,
    ranges_once: Arc<AtomicUsize>,
    limited_active: Arc<AtomicUsize>,
    busy: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

pub struct TestServer {
    pub base: String,
}

impl TestServer {
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/file/{size}", get(file))
            .route("/etag/{tag}/{size}", get(etag_file))
            .route("/slow/{size}", get(slow))
            .route("/norange/{size}", get(norange))
            .route("/flaky/{size}", get(flaky))
            .route("/ranges-once/{size}", get(ranges_once))
            .route("/limited/{size}", get(limited))
            .route("/busy/{size}", get(busy))
            .route("/drops/{size}", get(drops))
            .route("/wrong-range/{size}", get(wrong_range))
            .route("/expired", get(|| async { StatusCode::FORBIDDEN }))
            .route("/needs-cookie/{size}", get(needs_cookie))
            .with_state(Counters::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { base: format!("http://{addr}") }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}

/// Returns (start, end_exclusive) for `Range: bytes=a-b` / `bytes=a-`.
fn parse_range(headers: &HeaderMap, len: usize) -> Option<(usize, usize)> {
    let v = headers.get(header::RANGE)?.to_str().ok()?.strip_prefix("bytes=")?;
    let (a, b) = v.split_once('-')?;
    let start: usize = a.parse().ok()?;
    let end = if b.is_empty() { len } else { b.parse::<usize>().ok()? + 1 };
    Some((start, end.min(len)))
}

fn stream_body(bytes: Vec<u8>, delay: Option<Duration>, fail_after: Option<usize>) -> Body {
    let chunks: Vec<Vec<u8>> = bytes.chunks(CHUNK).map(<[u8]>::to_vec).collect();
    let stream = futures_util::stream::iter(chunks.into_iter().enumerate()).then(move |(i, c)| async move {
        if let Some(d) = delay {
            tokio::time::sleep(d).await;
        }
        if fail_after.is_some_and(|n| i * CHUNK >= n) {
            return Err(std::io::Error::other("simulated connection drop"));
        }
        Ok::<_, std::io::Error>(c)
    });
    Body::from_stream(stream)
}

fn ranged(headers: &HeaderMap, body: Vec<u8>, etag: &str, delay: Option<Duration>) -> Response {
    let len = body.len();
    let builder = Response::builder()
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, etag)
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"test.bin\"");
    match parse_range(headers, len) {
        Some((start, _)) if start >= len => builder
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{len}"))
            .body(Body::empty())
            .unwrap(),
        Some((start, end)) => builder
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{}/{len}", end - 1))
            .header(header::CONTENT_LENGTH, end - start)
            .body(stream_body(body[start..end].to_vec(), delay, None))
            .unwrap(),
        None => builder
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, len)
            .body(stream_body(body, delay, None))
            .unwrap(),
    }
}

fn full_200(size: usize) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, size)
        .body(stream_body(data(size), None, None))
        .unwrap()
}

/// Like a logged-in download: every request (probe and each range) must carry the browser's cookie and referrer.
async fn needs_cookie(Path(size): Path<usize>, headers: HeaderMap) -> Response {
    let has = |name: header::HeaderName, value: &str| headers.get(name).and_then(|v| v.to_str().ok()) == Some(value);
    if !(has(header::COOKIE, "session=abc; theme=dark") && has(header::REFERER, "https://site.test/page")) {
        return Response::builder().status(StatusCode::FORBIDDEN).body(Body::empty()).unwrap();
    }
    ranged(&headers, data(size), "\"c\"", None)
}

async fn file(Path(size): Path<usize>, headers: HeaderMap) -> Response {
    ranged(&headers, data(size), "\"v1\"", None)
}

async fn etag_file(Path((tag, size)): Path<(String, usize)>, headers: HeaderMap) -> Response {
    ranged(&headers, data_with_seed(size, seed_for(&tag)), &format!("\"{tag}\""), Some(Duration::from_millis(10)))
}

async fn slow(Path(size): Path<usize>, headers: HeaderMap) -> Response {
    // 64 chunks per connection × 40 ms ≈ 2.6 s, well past the engine's 1 s sidecar save on any OS timer.
    ranged(&headers, data(size), "\"v1\"", Some(Duration::from_millis(40)))
}

async fn norange(Path(size): Path<usize>) -> Response {
    full_200(size)
}

async fn flaky(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    let full = data(size);
    let Some((start, end)) = parse_range(&headers, size) else {
        return ranged(&headers, full, "\"v1\"", None);
    };
    if end - start > 1 && c.flaky.fetch_add(1, Ordering::SeqCst) < 2 {
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::ETAG, "\"v1\"")
            .header(header::CONTENT_RANGE, format!("bytes {start}-{}/{size}", end - 1))
            .header(header::CONTENT_LENGTH, end - start)
            .body(stream_body(full[start..end].to_vec(), None, Some((end - start) / 2)))
            .unwrap();
    }
    ranged(&headers, full, "\"v1\"", None)
}

async fn ranges_once(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    if c.ranges_once.fetch_add(1, Ordering::SeqCst) == 0 {
        ranged(&headers, data(size), "\"v1\"", None)
    } else {
        full_200(size)
    }
}

/// Allows at most 2 concurrent ranged transfers; extra ones get 429 + Retry-After: 0.
async fn limited(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    let full = data(size);
    let Some((start, end)) = parse_range(&headers, size) else {
        return ranged(&headers, full, "\"v1\"", None);
    };
    if end - start <= 1 {
        return ranged(&headers, full, "\"v1\"", None);
    }
    if c.limited_active.fetch_add(1, Ordering::SeqCst) >= 2 {
        c.limited_active.fetch_sub(1, Ordering::SeqCst);
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::RETRY_AFTER, "0")
            .body(Body::empty())
            .unwrap();
    }
    let guard = ActiveGuard(c.limited_active.clone());
    let chunks: Vec<Vec<u8>> = full[start..end].chunks(CHUNK).map(<[u8]>::to_vec).collect();
    let stream = futures_util::stream::iter(chunks).then(move |chunk| {
        let _held = &guard;
        async move {
            tokio::time::sleep(Duration::from_millis(2)).await;
            Ok::<_, std::io::Error>(chunk)
        }
    });
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::ETAG, "\"v1\"")
        .header(header::CONTENT_RANGE, format!("bytes {start}-{}/{size}", end - 1))
        .header(header::CONTENT_LENGTH, end - start)
        .body(Body::from_stream(stream))
        .unwrap()
}

struct ActiveGuard(Arc<AtomicUsize>);

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The first 2 requests (including the probe) get 503 + Retry-After: 0.
async fn busy(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    if c.busy.fetch_add(1, Ordering::SeqCst) < 2 {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(header::RETRY_AFTER, "0")
            .body(Body::empty())
            .unwrap();
    }
    ranged(&headers, data(size), "\"v1\"", None)
}

/// Every ranged transfer drops after 64 KiB, 10 times in total, always making progress.
async fn drops(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    let full = data(size);
    let Some((start, end)) = parse_range(&headers, size) else {
        return ranged(&headers, full, "\"v1\"", None);
    };
    if end - start > 64 * 1024 && c.drops.fetch_add(1, Ordering::SeqCst) < 10 {
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::ETAG, "\"v1\"")
            .header(header::CONTENT_RANGE, format!("bytes {start}-{}/{size}", end - 1))
            .header(header::CONTENT_LENGTH, end - start)
            .body(stream_body(full[start..end].to_vec(), Some(Duration::from_millis(2)), Some(64 * 1024)))
            .unwrap();
    }
    ranged(&headers, full, "\"v1\"", None)
}

/// Answers every ranged request (except the 1-byte probe) with the file's *first*
/// bytes, honestly labelled as `bytes 0-…`. Without a Range header: plain 200.
async fn wrong_range(Path(size): Path<usize>, headers: HeaderMap) -> Response {
    let full = data(size);
    let Some((start, end)) = parse_range(&headers, size) else {
        return full_200(size);
    };
    if end - start <= 1 {
        return ranged(&headers, full, "\"v1\"", None);
    }
    let len = end - start;
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::ETAG, "\"v1\"")
        .header(header::CONTENT_RANGE, format!("bytes 0-{}/{size}", len - 1))
        .header(header::CONTENT_LENGTH, len)
        .body(stream_body(full[..len].to_vec(), None, None))
        .unwrap()
}
