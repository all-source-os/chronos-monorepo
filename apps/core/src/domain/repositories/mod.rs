// Core event sourcing repositories
pub mod audit_event_repository;
pub mod event_repository;
pub mod event_stream_repository;
pub mod fork_repository;
pub mod tenant_repository;
pub mod vector_search_repository;

// Paywall domain repositories
pub mod access_token_repository;
pub mod article_repository;
pub mod creator_repository;
pub mod transaction_repository;

// Core re-exports
pub use audit_event_repository::{AuditEventQuery, AuditEventRepository};
pub use event_repository::{EventReader, EventRepository, EventWriter};
pub use event_stream_repository::{EventStreamReader, EventStreamRepository, EventStreamWriter};
pub use fork_repository::{ForkQuery, ForkRepository};
pub use tenant_repository::{TenantQuery, TenantRepository, deep_merge_metadata};
pub use vector_search_repository::{
    SearchResult, VectorEntry, VectorSearchQuery, VectorSearchReader, VectorSearchRepository,
    VectorSearchWriter,
};

// Paywall re-exports
pub use access_token_repository::{AccessTokenQuery, AccessTokenRepository};
pub use article_repository::{ArticleOrderBy, ArticleQuery, ArticleRepository};
pub use creator_repository::{CreatorQuery, CreatorRepository};
pub use transaction_repository::{
    RevenueDataPoint, RevenueGranularity, TransactionQuery, TransactionRepository,
};
