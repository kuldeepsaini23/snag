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

pub struct TestServer {
    pub base: String,
}

impl TestServer {
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/file/{size}", get(|Path(size): Path<usize>, h: HeaderMap| async move { serve(&h, size, "test.bin", None) }))
            .route("/slow/{size}", get(|Path(size): Path<usize>, h: HeaderMap| async move { serve(&h, size, "test.bin", Some(Duration::from_millis(10))) }))
            .route("/named/{name}/{size}", get(|Path((name, size)): Path<(String, usize)>, h: HeaderMap| async move { serve(&h, size, &name, None) }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { base: format!("http://{addr}") }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
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
