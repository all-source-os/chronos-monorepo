use better_auth_core::error::{AuthError, DatabaseError};

#[derive(Debug, thiserror::Error)]
pub enum AllsourceAuthError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Allsource API error ({status}): {message}")]
    Api { status: u16, message: String },
}

impl From<AllsourceAuthError> for AuthError {
    fn from(err: AllsourceAuthError) -> Self {
        AuthError::Database(DatabaseError::Query(err.to_string()))
    }
}
