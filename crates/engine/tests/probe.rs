mod support;

use rdm_engine::{EngineError, default_client, probe};
use support::TestServer;

#[tokio::test]
async fn probes_range_capable_server() {
    let s = TestServer::start().await;
    let info = probe(&default_client(), &s.url("/file/1000000")).await.unwrap();
    assert_eq!(info.size, Some(1_000_000));
    assert!(info.accepts_ranges);
    assert_eq!(info.filename, "test.bin");
    assert_eq!(info.etag.as_deref(), Some("\"v1\""));
    assert_eq!(info.url, s.url("/file/1000000"));
}

#[tokio::test]
async fn probes_server_without_ranges() {
    let s = TestServer::start().await;
    let info = probe(&default_client(), &s.url("/norange/1000")).await.unwrap();
    assert_eq!(info.size, Some(1000));
    assert!(!info.accepts_ranges);
    assert_eq!(info.filename, "1000");
}

#[tokio::test]
async fn probes_zero_byte_file() {
    let s = TestServer::start().await;
    let info = probe(&default_client(), &s.url("/file/0")).await.unwrap();
    assert_eq!(info.size, Some(0));
    assert!(!info.accepts_ranges);
}

#[tokio::test]
async fn forbidden_is_link_expired() {
    let s = TestServer::start().await;
    let err = probe(&default_client(), &s.url("/expired")).await.unwrap_err();
    assert!(matches!(err, EngineError::LinkExpired(403)), "{err:?}");
}
