use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// AllSource error types
#[derive(Debug, thiserror::Error)]
pub enum AllSourceError {
    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Tenant already exists: {0}")]
    TenantAlreadyExists(String),

    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Arrow error: {0}")]
    ArrowError(String),

    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Concurrency error: {0}")]
    ConcurrencyError(String),

    #[error("Queue full: {0}")]
    QueueFull(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

// Alias for domain layer convenience
pub use AllSourceError as Error;

impl From<arrow::error::ArrowError> for AllSourceError {
    fn from(err: arrow::error::ArrowError) -> Self {
        AllSourceError::ArrowError(err.to_string())
    }
}

impl From<parquet::errors::ParquetError> for AllSourceError {
    fn from(err: parquet::errors::ParquetError) -> Self {
        AllSourceError::StorageError(err.to_string())
    }
}

/// Custom Result type for AllSource operations
pub type Result<T> = std::result::Result<T, AllSourceError>;

/// Implement IntoResponse for axum error handling
impl IntoResponse for AllSourceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AllSourceError::EventNotFound(_)
            | AllSourceError::EntityNotFound(_)
            | AllSourceError::TenantNotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            AllSourceError::InvalidEvent(_)
            | AllSourceError::InvalidQuery(_)
            | AllSourceError::InvalidInput(_)
            | AllSourceError::ValidationError(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            AllSourceError::TenantAlreadyExists(_)
            | AllSourceError::ConcurrencyError(_) => {
                (StatusCode::CONFLICT, self.to_string())
            }
            AllSourceError::QueueFull(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            AllSourceError::StorageError(_)
            | AllSourceError::ArrowError(_)
            | AllSourceError::IndexError(_)
            | AllSourceError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AllSourceError::SerializationError(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
        };

        let body = serde_json::json!({
            "error": error_message,
        });

        (status, axum::Json(body)).into_response()
    }
}
