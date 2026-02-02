// Application services layer
// Contains business logic orchestration and domain service implementations

pub mod analytics;
pub mod audit_logger;
pub mod creator_coordinator;
pub mod event_coordinator;
pub mod payment_coordinator;
pub mod pipeline;
pub mod projection;
pub mod replay;
pub mod schema;
pub mod tenant_service;
pub mod vector_search;

// Re-exports for convenience
pub use analytics::AnalyticsEngine;
pub use audit_logger::{AuditLogger, RequestContext};
pub use creator_coordinator::{ArticlePerformance, CreatorCoordinator, CreatorDashboard};
pub use event_coordinator::{
    BatchIngestResult, EntityHistory, EntitySnapshot, EventCoordinator,
};
pub use payment_coordinator::{PaymentCoordinator, PurchaseResult};
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
