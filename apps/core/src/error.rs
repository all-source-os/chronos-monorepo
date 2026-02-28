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

impl AllSourceError {
    /// Returns true for transient errors that may succeed on retry
    /// (storage I/O, concurrency conflicts, queue pressure).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AllSourceError::StorageError(_)
                | AllSourceError::ConcurrencyError(_)
                | AllSourceError::QueueFull(_)
        )
    }
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

impl From<crate::infrastructure::persistence::SimdJsonError> for AllSourceError {
    fn from(err: crate::infrastructure::persistence::SimdJsonError) -> Self {
        AllSourceError::SerializationError(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        )))
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::Error> for AllSourceError {
    fn from(err: sqlx::Error) -> Self {
        AllSourceError::StorageError(format!("Database error: {err}"))
    }
}

/// Custom Result type for AllSource operations
pub type Result<T> = std::result::Result<T, AllSourceError>;

#[cfg(feature = "server")]
mod axum_impl {
    use super::AllSourceError;
    use axum::{
        http::StatusCode,
        response::{IntoResponse, Response},
    };

    /// Implement IntoResponse for axum error handling
    impl IntoResponse for AllSourceError {
        fn into_response(self) -> Response {
            let (status, error_message) = match self {
                AllSourceError::EventNotFound(_)
                | AllSourceError::EntityNotFound(_)
                | AllSourceError::TenantNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "server")]
    use axum::{http::StatusCode, response::IntoResponse};

    #[test]
    fn test_error_display() {
        let err = AllSourceError::EventNotFound("event-123".to_string());
        assert_eq!(err.to_string(), "Event not found: event-123");

        let err = AllSourceError::EntityNotFound("entity-456".to_string());
        assert_eq!(err.to_string(), "Entity not found: entity-456");

        let err = AllSourceError::TenantAlreadyExists("tenant-1".to_string());
        assert_eq!(err.to_string(), "Tenant already exists: tenant-1");

        let err = AllSourceError::TenantNotFound("tenant-2".to_string());
        assert_eq!(err.to_string(), "Tenant not found: tenant-2");
    }

    #[test]
    fn test_error_variants() {
        let errors: Vec<AllSourceError> = vec![
            AllSourceError::InvalidEvent("bad event".to_string()),
            AllSourceError::InvalidQuery("bad query".to_string()),
            AllSourceError::InvalidInput("bad input".to_string()),
            AllSourceError::StorageError("storage failed".to_string()),
            AllSourceError::ArrowError("arrow failed".to_string()),
            AllSourceError::IndexError("index failed".to_string()),
            AllSourceError::ValidationError("validation failed".to_string()),
            AllSourceError::ConcurrencyError("conflict".to_string()),
            AllSourceError::QueueFull("queue full".to_string()),
            AllSourceError::InternalError("internal error".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_into_response_not_found() {
        let err = AllSourceError::EventNotFound("event-123".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let err = AllSourceError::EntityNotFound("entity-456".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let err = AllSourceError::TenantNotFound("tenant-1".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_into_response_bad_request() {
        let err = AllSourceError::InvalidEvent("bad event".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = AllSourceError::InvalidQuery("bad query".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = AllSourceError::InvalidInput("bad input".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = AllSourceError::ValidationError("validation failed".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_into_response_conflict() {
        let err = AllSourceError::TenantAlreadyExists("tenant-1".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let err = AllSourceError::ConcurrencyError("conflict".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_into_response_service_unavailable() {
        let err = AllSourceError::QueueFull("queue is full".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_into_response_internal_error() {
        let err = AllSourceError::StorageError("storage error".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let err = AllSourceError::ArrowError("arrow error".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let err = AllSourceError::IndexError("index error".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let err = AllSourceError::InternalError("internal error".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_from_arrow_error() {
        let arrow_err = arrow::error::ArrowError::InvalidArgumentError("test".to_string());
        let err: AllSourceError = arrow_err.into();
        assert!(matches!(err, AllSourceError::ArrowError(_)));
    }

    #[test]
    fn test_from_parquet_error() {
        let parquet_err = parquet::errors::ParquetError::General("test".to_string());
        let err: AllSourceError = parquet_err.into();
        assert!(matches!(err, AllSourceError::StorageError(_)));
    }

    #[test]
    fn test_error_debug() {
        let err = AllSourceError::EventNotFound("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("EventNotFound"));
    }

    #[test]
    fn test_is_retryable() {
        // Retryable errors
        assert!(AllSourceError::StorageError("io".to_string()).is_retryable());
        assert!(AllSourceError::ConcurrencyError("conflict".to_string()).is_retryable());
        assert!(AllSourceError::QueueFull("backpressure".to_string()).is_retryable());

        // Non-retryable errors
        assert!(!AllSourceError::EventNotFound("e1".to_string()).is_retryable());
        assert!(!AllSourceError::InvalidEvent("bad".to_string()).is_retryable());
        assert!(!AllSourceError::InvalidQuery("bad".to_string()).is_retryable());
        assert!(!AllSourceError::ValidationError("bad".to_string()).is_retryable());
        assert!(!AllSourceError::TenantNotFound("t1".to_string()).is_retryable());
        assert!(!AllSourceError::TenantAlreadyExists("t1".to_string()).is_retryable());
        assert!(!AllSourceError::InternalError("bug".to_string()).is_retryable());
    }
}
