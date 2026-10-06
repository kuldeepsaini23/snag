pub mod download;
pub mod error;
pub mod filename;
pub mod info;
pub mod limiter;
pub mod probe;
pub mod segments;
pub mod state;

pub use download::{DownloadOptions, Outcome, Progress, download};
pub use error::EngineError;
pub use info::RemoteInfo;
pub use limiter::RateLimiter;
pub use probe::probe;
pub use reqwest::Client;
pub use tokio_util::sync::CancellationToken;

use std::time::Duration;

/// `default_client()` plus headers sent with every request (e.g. the browser's Cookie and Referer).
/// reqwest drops Cookie when a redirect leads to another host.
pub fn client_with(headers: reqwest::header::HeaderMap) -> reqwest::Client {
    builder().default_headers(headers).build().expect("static client config is valid")
}

pub fn default_client() -> reqwest::Client {
    builder().build().expect("static client config is valid")
}

fn builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(concat!("RDM/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
}
