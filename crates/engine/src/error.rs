use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("network error: {0}")]
    Network(String),
    #[error("link expired or access denied (HTTP {0})")]
    LinkExpired(u16),
    #[error("server returned HTTP {0}")]
    Http(u16),
    #[error("server is busy or limiting connections (HTTP {status})")]
    Throttled { status: u16, retry_after: Option<Duration> },
    #[error("another download is already writing to this file")]
    DestinationBusy,
    #[error("server stopped honoring range requests")]
    RangeNotHonored,
    #[error("disk error: {0}")]
    Disk(#[from] std::io::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for EngineError {
    fn from(e: reqwest::Error) -> Self {
        EngineError::Network(e.to_string())
    }
}

impl EngineError {
    pub fn from_status(status: u16) -> Self {
        Self::from_parts(status, None)
    }

    /// Status plus the raw `Retry-After` header value.
    pub fn from_parts(status: u16, retry_after: Option<&str>) -> Self {
        match status {
            403 | 410 => Self::LinkExpired(status),
            429 | 503 => Self::Throttled { status, retry_after: retry_after.and_then(parse_retry_after) },
            s => Self::Http(s),
        }
    }

    pub(crate) fn from_response(resp: &reqwest::Response) -> Self {
        let retry_after = resp.headers().get(reqwest::header::RETRY_AFTER).and_then(|v| v.to_str().ok());
        Self::from_parts(resp.status().as_u16(), retry_after)
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) | Self::Throttled { .. } => true,
            Self::Http(s) => *s >= 500,
            _ => false,
        }
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Throttled { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

/// `Retry-After` in its seconds form, capped at 2 minutes. HTTP-date form is ignored.
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    let secs: u64 = value.trim().parse().ok()?;
    Some(Duration::from_secs(secs.min(120)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_seconds_are_parsed_and_capped() {
        assert_eq!(parse_retry_after("5"), Some(Duration::from_secs(5)));
        assert_eq!(parse_retry_after(" 0 "), Some(Duration::ZERO));
        assert_eq!(parse_retry_after("300"), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), None);
    }

    #[test]
    fn throttling_statuses_are_retryable_and_keep_retry_after() {
        assert!(matches!(
            EngineError::from_parts(429, Some("3")),
            EngineError::Throttled { status: 429, retry_after: Some(d) } if d == Duration::from_secs(3)
        ));
        assert!(matches!(EngineError::from_parts(503, None), EngineError::Throttled { status: 503, retry_after: None }));
        assert!(matches!(EngineError::from_parts(403, Some("3")), EngineError::LinkExpired(403)));
        assert!(EngineError::from_parts(429, None).is_retryable());
    }

    #[test]
    fn maps_status_codes() {
        assert!(matches!(EngineError::from_status(403), EngineError::LinkExpired(403)));
        assert!(matches!(EngineError::from_status(410), EngineError::LinkExpired(410)));
        assert!(matches!(EngineError::from_status(404), EngineError::Http(404)));
    }

    #[test]
    fn only_transient_errors_retry() {
        assert!(EngineError::Network("x".into()).is_retryable());
        assert!(EngineError::Http(503).is_retryable());
        assert!(EngineError::from_status(429).is_retryable());
        assert!(!EngineError::Http(404).is_retryable());
        assert!(!EngineError::LinkExpired(403).is_retryable());
        assert!(!EngineError::RangeNotHonored.is_retryable());
    }
}
