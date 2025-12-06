//! # AllSource Core - High-Performance Event Store
//!
//! A high-performance event sourcing platform built in Rust, following Clean Architecture principles.
//!
//! ## Architecture Overview
//!
//! The codebase follows a layered Clean Architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Infrastructure Layer                     │
//! │  (HTTP handlers, WebSocket, persistence, security)          │
//! │  infrastructure::web, infrastructure::persistence,          │
//! │  infrastructure::security, infrastructure::repositories     │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    Application Layer                        │
//! │  (Use cases, services, DTOs)                                │
//! │  application::use_cases, application::services,             │
//! │  application::dto                                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │                      Domain Layer                           │
//! │  (Entities, value objects, repository traits)               │
//! │  domain::entities, domain::value_objects,                   │
//! │  domain::repositories                                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! - **domain**: Core business entities, value objects, and repository traits
//! - **application**: Use cases, services, and DTOs that orchestrate domain logic
//! - **infrastructure**: Concrete implementations (web, persistence, security)
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use allsource_core::{EventStore, Event, IngestEventRequest};
//!
//! let store = EventStore::new();
//! let event = Event::from_strings(
//!     "user.created".to_string(),
//!     "user-123".to_string(),
//!     "default".to_string(),
//!     serde_json::json!({"name": "Alice"}),
//!     None,
//! )?;
//! store.ingest(event)?;
//! ```

// Suppress warnings for development
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(deprecated)]
#![allow(unused_must_use)]
// Clippy configuration - allow common patterns in this codebase
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::useless_vec)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::assign_op_pattern)]

// =============================================================================
// Clean Architecture Layers
// =============================================================================

/// Layer 1: Domain Layer - Enterprise Business Rules
///
/// Contains pure business entities, value objects, and repository traits.
/// This layer has ZERO external dependencies.
pub mod domain;

/// Layer 2: Application Layer - Application Business Rules
///
/// Contains use cases that orchestrate domain entities and services.
/// Depends only on the domain layer.
pub mod application;

/// Layer 3: Infrastructure Layer - Interface Adapters
///
/// Contains concrete implementations of abstractions.
/// Depends on domain and application layers.
pub mod infrastructure;

// =============================================================================
// Shared Modules
// =============================================================================

/// Error types for the entire crate
pub mod error;

/// Main EventStore facade
pub mod store;

/// Advanced security module (anomaly detection, encryption, KMS)
pub mod security;

// =============================================================================
// Public API - Commonly Used Types
// =============================================================================

// Domain layer exports
pub use domain::entities;
pub use domain::entities::Event;
pub use domain::repositories;

// Application layer exports
pub use application::dto::{IngestEventRequest, QueryEventsRequest};
pub use application::services::{
    AnalyticsEngine, Pipeline, PipelineConfig, PipelineManager, ProjectionManager, ReplayManager,
    SchemaRegistry, TenantManager,
};

// Infrastructure layer exports
pub use infrastructure::persistence::{
    CompactionConfig, CompactionManager, EventIndex, ParquetStorage, SnapshotConfig,
    SnapshotManager, WALConfig, WriteAheadLog,
};
pub use infrastructure::security::{AuthManager, Permission, RateLimiter, Role};
pub use infrastructure::web::{serve, WebSocketManager};

// Error handling
pub use error::{AllSourceError, Result};

// =============================================================================
// Backward-Compatible Aliases (for binaries and external users)
// =============================================================================

/// Auth module re-export for backward compatibility
pub mod auth {
    pub use crate::infrastructure::security::{AuthManager, Permission, Role};
}

/// Rate limiting module re-export
pub mod rate_limit {
    pub use crate::infrastructure::security::rate_limit::{RateLimitConfig, RateLimiter};
}

/// Tenant module re-export
pub mod tenant {
    pub use crate::application::services::tenant_service::{TenantManager, TenantQuotas};
    pub use crate::domain::entities::Tenant;
}

/// Config module re-export
pub mod config {
    pub use crate::infrastructure::config::*;
}

/// Backup module re-export
pub mod backup {
    pub use crate::infrastructure::persistence::backup::*;
}

/// API v1 module re-export
pub mod api_v1 {
    pub use crate::infrastructure::web::api_v1::*;
}

// Main store facade
pub use store::EventStore;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod security_integration_tests;
