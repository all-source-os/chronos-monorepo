use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Pagination request parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationRequest {
    /// Number of items to return (default: 20, max: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Number of items to skip (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    20
}

impl PaginationRequest {
    pub fn new(limit: usize, offset: usize) -> Self {
        Self {
            limit: limit.min(100),
            offset,
        }
    }

    /// Clamp limit to max 100
    pub fn clamped_limit(&self) -> usize {
        self.limit.min(100)
    }
}

/// Pagination metadata in responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationResponse {
    /// Total number of items available
    pub total: usize,

    /// Number of items returned in this response
    pub count: usize,

    /// Current offset
    pub offset: usize,

    /// Limit used for this request
    pub limit: usize,

    /// Whether there are more items after this page
    pub has_more: bool,
}

impl PaginationResponse {
    pub fn new(total: usize, count: usize, offset: usize, limit: usize) -> Self {
        Self {
            total,
            count,
            offset,
            limit,
            has_more: offset + count < total,
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

/// Generic sort request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortRequest {
    /// Field to sort by
    pub field: String,

    /// Sort direction
    #[serde(default)]
    pub direction: SortDirection,
}

impl SortRequest {
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            field: field.into(),
            direction,
        }
    }

    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Asc)
    }

    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Desc)
    }
}

/// Time range filter for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeRangeFilter {
    /// Start of time range (inclusive)
    pub since: Option<DateTime<Utc>>,

    /// End of time range (inclusive)
    pub until: Option<DateTime<Utc>>,

    /// Point-in-time query (time-travel)
    pub as_of: Option<DateTime<Utc>>,
}

impl TimeRangeFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }

    pub fn as_of(mut self, as_of: DateTime<Utc>) -> Self {
        self.as_of = Some(as_of);
        self
    }

    /// Check if the filter has any time constraints
    pub fn has_constraints(&self) -> bool {
        self.since.is_some() || self.until.is_some() || self.as_of.is_some()
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

/// Error code categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // Validation errors (400)
    ValidationError,
    InvalidInput,
    MissingField,
    InvalidFormat,

    // Authentication errors (401)
    Unauthorized,
    InvalidToken,
    TokenExpired,

    // Authorization errors (403)
    Forbidden,
    InsufficientPermissions,
    QuotaExceeded,

    // Not found errors (404)
    NotFound,
    EntityNotFound,
    ResourceNotFound,

    // Conflict errors (409)
    Conflict,
    DuplicateEntry,
    ConcurrencyConflict,
    VersionMismatch,

    // Rate limiting (429)
    RateLimited,
    TooManyRequests,

    // Server errors (500)
    InternalError,
    DatabaseError,
    ServiceUnavailable,
}

impl ErrorCode {
    /// Get the HTTP status code for this error code
    pub fn http_status(&self) -> u16 {
        match self {
            ErrorCode::ValidationError
            | ErrorCode::InvalidInput
            | ErrorCode::MissingField
            | ErrorCode::InvalidFormat => 400,

            ErrorCode::Unauthorized | ErrorCode::InvalidToken | ErrorCode::TokenExpired => 401,

            ErrorCode::Forbidden
            | ErrorCode::InsufficientPermissions
            | ErrorCode::QuotaExceeded => 403,

            ErrorCode::NotFound | ErrorCode::EntityNotFound | ErrorCode::ResourceNotFound => 404,

            ErrorCode::Conflict
            | ErrorCode::DuplicateEntry
            | ErrorCode::ConcurrencyConflict
            | ErrorCode::VersionMismatch => 409,

            ErrorCode::RateLimited | ErrorCode::TooManyRequests => 429,

            ErrorCode::InternalError | ErrorCode::DatabaseError | ErrorCode::ServiceUnavailable => {
                500
            }
        }
    }
}

/// Field-level validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// Field name that has the error
    pub field: String,

    /// Error message for this field
    pub message: String,

    /// Error code for this field error
    pub code: Option<String>,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Standard error response DTO for API errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: ErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Additional details about the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Field-level validation errors
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub field_errors: Vec<FieldError>,

    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Timestamp of the error
    pub timestamp: DateTime<Utc>,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            field_errors: Vec::new(),
            request_id: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_field_errors(mut self, errors: Vec<FieldError>) -> Self {
        self.field_errors = errors;
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Create a validation error response
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationError, message)
    }

    /// Create a not found error response
    pub fn not_found(entity: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, format!("{} not found", entity.into()))
    }

    /// Create a conflict error response
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    /// Create an internal error response
    pub fn internal_error() -> Self {
        Self::new(ErrorCode::InternalError, "An internal error occurred")
    }
}

/// Success response wrapper with optional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    /// The response data
    pub data: T,

    /// Optional metadata about the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data, meta: None }
    }

    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Request processing time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_time_ms: Option<u64>,

    /// API version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,

    /// Deprecation warning if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation_warning: Option<String>,
}

impl ResponseMeta {
    pub fn new() -> Self {
        Self {
            processing_time_ms: None,
            api_version: None,
            deprecation_warning: None,
        }
    }

    pub fn with_processing_time(mut self, ms: u64) -> Self {
        self.processing_time_ms = Some(ms);
        self
    }
}

impl Default for ResponseMeta {
    fn default() -> Self {
        Self::new()
    }
}

/// Paginated list response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The list of items
    pub items: Vec<T>,

    /// Pagination metadata
    pub pagination: PaginationResponse,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: usize, offset: usize, limit: usize) -> Self {
        let count = items.len();
        Self {
            items,
            pagination: PaginationResponse::new(total, count, offset, limit),
        }
    }

    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            pagination: PaginationResponse::new(0, 0, 0, 20),
        }
    }
}

/// Batch operation result for individual items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult<T> {
    /// Index of the item in the original batch
    pub index: usize,

    /// Whether this item was processed successfully
    pub success: bool,

    /// The result data if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Error information if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
}

impl<T> BatchItemResult<T> {
    pub fn success(index: usize, data: T) -> Self {
        Self {
            index,
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(index: usize, error: ErrorResponse) -> Self {
        Self {
            index,
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Batch operation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse<T> {
    /// Results for each item in the batch
    pub results: Vec<BatchItemResult<T>>,

    /// Total items in the batch
    pub total: usize,

    /// Number of successful operations
    pub successful: usize,

    /// Number of failed operations
    pub failed: usize,
}

impl<T> BatchResponse<T> {
    pub fn new(results: Vec<BatchItemResult<T>>) -> Self {
        let total = results.len();
        let successful = results.iter().filter(|r| r.success).count();
        let failed = total - successful;

        Self {
            results,
            total,
            successful,
            failed,
        }
    }

    /// Check if all operations succeeded
    pub fn all_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status
    pub status: HealthStatus,

    /// Service version
    pub version: String,

    /// Uptime in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,

    /// Component health checks
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub components: Vec<ComponentHealth>,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,

    /// Component status
    pub status: HealthStatus,

    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Response time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_request_clamping() {
        let pagination = PaginationRequest::new(200, 0);
        assert_eq!(pagination.clamped_limit(), 100);
    }

    #[test]
    fn test_pagination_response_has_more() {
        let response = PaginationResponse::new(100, 20, 0, 20);
        assert!(response.has_more);

        let response = PaginationResponse::new(100, 20, 80, 20);
        assert!(!response.has_more);
    }

    #[test]
    fn test_error_code_http_status() {
        assert_eq!(ErrorCode::ValidationError.http_status(), 400);
        assert_eq!(ErrorCode::Unauthorized.http_status(), 401);
        assert_eq!(ErrorCode::Forbidden.http_status(), 403);
        assert_eq!(ErrorCode::NotFound.http_status(), 404);
        assert_eq!(ErrorCode::Conflict.http_status(), 409);
        assert_eq!(ErrorCode::RateLimited.http_status(), 429);
        assert_eq!(ErrorCode::InternalError.http_status(), 500);
    }

    #[test]
    fn test_error_response_serialization() {
        let error =
            ErrorResponse::validation_error("Invalid email format").with_field_errors(vec![
                FieldError::new("email", "must be a valid email address"),
            ]);

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("VALIDATION_ERROR"));
        assert!(json.contains("email"));
    }

    #[test]
    fn test_time_range_filter() {
        let filter = TimeRangeFilter::new();
        assert!(!filter.has_constraints());

        let filter = filter.since(Utc::now());
        assert!(filter.has_constraints());
    }

    #[test]
    fn test_paginated_response() {
        let items = vec!["a", "b", "c"];
        let response = PaginatedResponse::new(items, 10, 0, 3);

        assert_eq!(response.items.len(), 3);
        assert_eq!(response.pagination.total, 10);
        assert!(response.pagination.has_more);
    }

    #[test]
    fn test_batch_response() {
        let results = vec![
            BatchItemResult::success(0, "item1"),
            BatchItemResult::failure(1, ErrorResponse::validation_error("Invalid")),
            BatchItemResult::success(2, "item3"),
        ];

        let batch = BatchResponse::new(results);
        assert_eq!(batch.total, 3);
        assert_eq!(batch.successful, 2);
        assert_eq!(batch.failed, 1);
        assert!(!batch.all_successful());
    }
}
