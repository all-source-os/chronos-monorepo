// Application services layer
// Contains business logic orchestration and domain service implementations

pub mod analytics;
pub mod audit_logger;
pub mod pipeline;
pub mod projection;
pub mod replay;
pub mod schema;
pub mod tenant_service;
pub mod vector_search;

// Re-exports for convenience
pub use analytics::AnalyticsEngine;
pub use audit_logger::{AuditLogger, RequestContext};
pub use pipeline::{Pipeline, PipelineConfig, PipelineManager, PipelineOperator, PipelineStats};
pub use projection::{
    EntitySnapshotProjection, EventCounterProjection, Projection, ProjectionManager,
};
pub use replay::{ReplayManager, ReplayProgress, StartReplayRequest, StartReplayResponse};
pub use schema::{
    CompatibilityMode, RegisterSchemaRequest, RegisterSchemaResponse, SchemaRegistry,
    SchemaRegistryConfig, ValidateEventRequest, ValidateEventResponse,
};
pub use tenant_service::{Tenant, TenantManager, TenantQuotas, TenantUsage};
pub use vector_search::{
    BatchIndexResult, IndexEventRequest, IndexStats, SemanticSearchRequest,
    SemanticSearchResponse, SemanticSearchResultItem, VectorSearchConfig, VectorSearchService,
};
