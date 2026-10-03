use crate::{error::EngineError, filename::filename_from, info::RemoteInfo};
use reqwest::{Client, StatusCode, header};

/// One `Range: bytes=0-0` GET tells us size, range support, filename and validators.
pub async fn probe(client: &Client, url: &str) -> Result<RemoteInfo, EngineError> {
    let resp = client.get(url).header(header::RANGE, "bytes=0-0").send().await?;
    let headers = resp.headers();
    let get = |name: header::HeaderName| {
        headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string)
    };
    let (size, accepts_ranges) = match resp.status() {
        StatusCode::PARTIAL_CONTENT => (get(header::CONTENT_RANGE).and_then(|v| parse_total(&v)), true),
        StatusCode::RANGE_NOT_SATISFIABLE => (Some(0), false),
        s if s.is_success() => (resp.content_length(), false),
        s => return Err(EngineError::from_status(s.as_u16())),
    };
    Ok(RemoteInfo {
        url: url.to_string(),
        size,
        accepts_ranges: accepts_ranges && size.is_some(),
        filename: filename_from(get(header::CONTENT_DISPOSITION).as_deref(), resp.url().as_str()),
        etag: get(header::ETAG),
        last_modified: get(header::LAST_MODIFIED),
    })
}

/// `bytes 0-0/12345` → 12345; `bytes 0-0/*` → None.
fn parse_total(content_range: &str) -> Option<u64> {
    content_range.rsplit('/').next()?.trim().parse().ok()
}
