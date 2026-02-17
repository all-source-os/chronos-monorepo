use serde_json::Value;

/// Errors returned by the AllSource SDK.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The AllSource API returned a non-2xx status code.
    #[error("AllSource API error: {status} — {message}")]
    Api {
        status: u16,
        message: String,
        body: Option<Value>,
    },

    /// Configuration error (e.g., missing base URL).
    #[error("configuration error: {0}")]
    Config(String),

    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Circuit breaker is open — backend is unavailable.
    #[error("circuit breaker open: backend unavailable, retry after {retry_after_secs}s")]
    CircuitOpen { retry_after_secs: u64 },
}

impl Error {
    /// Returns true if this is a 401 Unauthorized error.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Error::Api { status: 401, .. })
    }

    /// Returns true if this is a 429 Too Many Requests error.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Error::Api { status: 429, .. })
    }

    /// Returns true if this is a 404 Not Found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::Api { status: 404, .. })
    }

    /// Returns true if this is a server error (5xx).
    pub fn is_server_error(&self) -> bool {
        matches!(self, Error::Api { status, .. } if *status >= 500)
    }

    /// Returns true if the circuit breaker is open.
    pub fn is_circuit_open(&self) -> bool {
        matches!(self, Error::CircuitOpen { .. })
    }

    /// Returns true if this is a transient error that might succeed on retry.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Api { status, .. } => matches!(status, 408 | 429 | 500 | 502 | 503 | 504),
            Error::Http(_) => true,
            _ => false,
        }
    }
}
