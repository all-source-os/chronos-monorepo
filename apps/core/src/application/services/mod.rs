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
pub mod vector_search;
pub mod webhook;
// v0.14: Durable subscriptions
pub mod consumer;
// v2.0: Advanced features
pub mod exactly_once;
pub mod schema_evolution;

// Re-exports for convenience
pub use analytics::AnalyticsEngine;
pub use audit_logger::{AuditLogger, RequestContext};
pub use consumer::{Consumer, ConsumerRegistry};
pub use creator_coordinator::{ArticlePerformance, CreatorCoordinator, CreatorDashboard};
pub use event_coordinator::{BatchIngestResult, EntityHistory, EntitySnapshot, EventCoordinator};
pub use exactly_once::{ExactlyOnceConfig, ExactlyOnceRegistry, ExactlyOnceStats};
pub use payment_coordinator::{PaymentCoordinator, PurchaseResult};
pub use pipeline::{Pipeline, PipelineConfig, PipelineManager, PipelineOperator, PipelineStats};
pub use projection::{
    CheckpointConfig, EntitySnapshotProjection, EventCounterProjection, Projection,
    ProjectionCheckpoint, ProjectionManager,
};
pub use replay::{ReplayManager, ReplayProgress, StartReplayRequest, StartReplayResponse};
pub use schema::{
    CompatibilityMode, RegisterSchemaRequest, RegisterSchemaResponse, SchemaRegistry,
    SchemaRegistryConfig, ValidateEventRequest, ValidateEventResponse,
};
pub use schema_evolution::{EvolutionAction, SchemaEvolutionManager, SchemaEvolutionStats};
pub use vector_search::{
    BatchIndexResult, IndexEventRequest, IndexStats, SemanticSearchRequest, SemanticSearchResponse,
    SemanticSearchResultItem, VectorSearchConfig, VectorSearchService,
};
pub use webhook::{
    DeliveryStatus, RegisterWebhookRequest, UpdateWebhookRequest, WebhookDelivery, WebhookRegistry,
    WebhookSubscription,
};
