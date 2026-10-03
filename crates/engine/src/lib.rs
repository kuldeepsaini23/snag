pub mod error;
pub mod filename;

pub use error::EngineError;

use std::time::Duration;

pub fn default_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("RDM/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .expect("static client config is valid")
}
