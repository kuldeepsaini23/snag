use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("network error: {0}")]
    Network(String),
    #[error("link expired or access denied (HTTP {0})")]
    LinkExpired(u16),
    #[error("server returned HTTP {0}")]
    Http(u16),
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
        match status {
            403 | 410 => Self::LinkExpired(status),
            s => Self::Http(s),
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) => true,
            Self::Http(s) => *s == 429 || *s >= 500,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(EngineError::Http(429).is_retryable());
        assert!(!EngineError::Http(404).is_retryable());
        assert!(!EngineError::LinkExpired(403).is_retryable());
        assert!(!EngineError::RangeNotHonored.is_retryable());
    }
}
