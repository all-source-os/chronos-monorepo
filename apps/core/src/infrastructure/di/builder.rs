//! Container Builder - Fluent API for constructing ServiceContainer
//!
//! Provides a builder pattern for wiring up repositories and creating
//! a fully configured ServiceContainer.

use super::ServiceContainer;
use crate::{
    application::services::consumer::ConsumerRegistry,
    domain::repositories::{
        AccessTokenRepository, ArticleRepository, AuditEventRepository, CreatorRepository,
        EventStreamRepository, ForkRepository, TenantRepository, TransactionRepository,
    },
    infrastructure::{
        persistence::{SystemMetadataStore, SystemRepositories},
        repositories::{
            EventSourcedConfigRepository, InMemoryAccessTokenRepository, InMemoryArticleRepository,
            InMemoryCreatorRepository, InMemoryEventStreamRepository, InMemoryForkRepository,
            InMemoryTransactionRepository,
        },
    },
};
use std::sync::Arc;

/// Builder for constructing a ServiceContainer with all dependencies.
///
/// # Example
///
/// ```rust,ignore
/// // Development with in-memory repositories
/// let container = ContainerBuilder::new()
///     .with_in_memory_repositories()
///     .build();
///
/// // Testing with custom mock repositories
/// let container = ContainerBuilder::new()
///     .with_creator_repository(Arc::new(mock_creator_repo))
///     .with_article_repository(Arc::new(mock_article_repo))
///     .with_transaction_repository(Arc::new(mock_transaction_repo))
///     .with_access_token_repository(Arc::new(mock_access_repo))
///     .with_fork_repository(Arc::new(mock_fork_repo))
///     .with_event_stream_repository(Arc::new(mock_event_stream_repo))
///     .build();
/// ```
#[derive(Default)]
pub struct ContainerBuilder {
    creator_repository: Option<Arc<dyn CreatorRepository>>,
    article_repository: Option<Arc<dyn ArticleRepository>>,
    transaction_repository: Option<Arc<dyn TransactionRepository>>,
    access_token_repository: Option<Arc<dyn AccessTokenRepository>>,
    fork_repository: Option<Arc<dyn ForkRepository>>,
    event_stream_repository: Option<Arc<dyn EventStreamRepository>>,

    // System metadata (event-sourced)
    tenant_repository: Option<Arc<dyn TenantRepository>>,
    audit_repository: Option<Arc<dyn AuditEventRepository>>,
    config_repository: Option<Arc<EventSourcedConfigRepository>>,
    system_store: Option<Arc<SystemMetadataStore>>,
    consumer_registry: Option<Arc<ConsumerRegistry>>,
}

impl ContainerBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure all repositories with in-memory implementations.
    ///
    /// This is the recommended setup for development and testing.
    /// For production, use specific repository setters with persistent implementations.
    pub fn with_in_memory_repositories(mut self) -> Self {
        self.creator_repository = Some(Arc::new(InMemoryCreatorRepository::new()));
        self.article_repository = Some(Arc::new(InMemoryArticleRepository::new()));
        self.transaction_repository = Some(Arc::new(InMemoryTransactionRepository::new()));
        self.access_token_repository = Some(Arc::new(InMemoryAccessTokenRepository::new()));
        self.fork_repository = Some(Arc::new(InMemoryForkRepository::new()));
        self.event_stream_repository = Some(Arc::new(InMemoryEventStreamRepository::new()));
        self
    }

    /// Set a custom creator repository.
    pub fn with_creator_repository(mut self, repository: Arc<dyn CreatorRepository>) -> Self {
        self.creator_repository = Some(repository);
        self
    }

    /// Set a custom article repository.
    pub fn with_article_repository(mut self, repository: Arc<dyn ArticleRepository>) -> Self {
        self.article_repository = Some(repository);
        self
    }

    /// Set a custom transaction repository.
    pub fn with_transaction_repository(
        mut self,
        repository: Arc<dyn TransactionRepository>,
    ) -> Self {
        self.transaction_repository = Some(repository);
        self
    }

    /// Set a custom access token repository.
    pub fn with_access_token_repository(
        mut self,
        repository: Arc<dyn AccessTokenRepository>,
    ) -> Self {
        self.access_token_repository = Some(repository);
        self
    }

    /// Set a custom fork repository.
    pub fn with_fork_repository(mut self, repository: Arc<dyn ForkRepository>) -> Self {
        self.fork_repository = Some(repository);
        self
    }

    /// Set a custom event stream repository.
    pub fn with_event_stream_repository(
        mut self,
        repository: Arc<dyn EventStreamRepository>,
    ) -> Self {
        self.event_stream_repository = Some(repository);
        self
    }

    /// Wire event-sourced system repositories from a `SystemRepositories` instance.
    ///
    /// This replaces in-memory metadata storage with durable, event-sourced
    /// repositories backed by AllSource's own WAL. Call this after
    /// `SystemBootstrap::initialize()` succeeds.
    ///
    /// If not called, the container will have no system repositories (backward-compatible).
    pub fn with_system_repositories(mut self, repos: SystemRepositories) -> Self {
        self.system_store = Some(repos.system_store);
        self.tenant_repository = Some(repos.tenant_repository);
        self.audit_repository = Some(repos.audit_repository);
        self.config_repository = Some(repos.config_repository);
        self.consumer_registry = Some(repos.consumer_registry);
        self
    }

    /// Set a custom tenant repository.
    pub fn with_tenant_repository(mut self, repository: Arc<dyn TenantRepository>) -> Self {
        self.tenant_repository = Some(repository);
        self
    }

    /// Set a custom audit repository.
    pub fn with_audit_repository(mut self, repository: Arc<dyn AuditEventRepository>) -> Self {
        self.audit_repository = Some(repository);
        self
    }

    /// Build the ServiceContainer.
    ///
    /// # Panics
    ///
    /// Panics if any required repository has not been set.
    /// Use `with_in_memory_repositories()` to set all repositories at once,
    /// or set each one individually.
    pub fn build(self) -> ServiceContainer {
        let mut container = ServiceContainer::new(
            self.creator_repository
                .expect("CreatorRepository not configured. Use with_in_memory_repositories() or with_creator_repository()"),
            self.article_repository
                .expect("ArticleRepository not configured. Use with_in_memory_repositories() or with_article_repository()"),
            self.transaction_repository
                .expect("TransactionRepository not configured. Use with_in_memory_repositories() or with_transaction_repository()"),
            self.access_token_repository
                .expect("AccessTokenRepository not configured. Use with_in_memory_repositories() or with_access_token_repository()"),
            self.fork_repository
                .expect("ForkRepository not configured. Use with_in_memory_repositories() or with_fork_repository()"),
            self.event_stream_repository
                .expect("EventStreamRepository not configured. Use with_in_memory_repositories() or with_event_stream_repository()"),
        );

        container.tenant_repository = self.tenant_repository;
        container.audit_repository = self.audit_repository;
        container.config_repository = self.config_repository;
        container.system_store = self.system_store;
        container.consumer_registry = self.consumer_registry;

        container
    }

    /// Try to build the ServiceContainer, returning an error if any repository is missing.
    ///
    /// This is a safer alternative to `build()` that doesn't panic.
    pub fn try_build(self) -> Result<ServiceContainer, ContainerBuilderError> {
        let mut container = ServiceContainer::new(
            self.creator_repository
                .ok_or(ContainerBuilderError::MissingRepository(
                    "CreatorRepository",
                ))?,
            self.article_repository
                .ok_or(ContainerBuilderError::MissingRepository(
                    "ArticleRepository",
                ))?,
            self.transaction_repository
                .ok_or(ContainerBuilderError::MissingRepository(
                    "TransactionRepository",
                ))?,
            self.access_token_repository
                .ok_or(ContainerBuilderError::MissingRepository(
                    "AccessTokenRepository",
                ))?,
            self.fork_repository
                .ok_or(ContainerBuilderError::MissingRepository("ForkRepository"))?,
            self.event_stream_repository
                .ok_or(ContainerBuilderError::MissingRepository(
                    "EventStreamRepository",
                ))?,
        );

        container.tenant_repository = self.tenant_repository;
        container.audit_repository = self.audit_repository;
        container.config_repository = self.config_repository;
        container.system_store = self.system_store;
        container.consumer_registry = self.consumer_registry;

        Ok(container)
    }
}

/// Error returned when building a container fails.
#[derive(Debug, Clone)]
pub enum ContainerBuilderError {
    /// A required repository was not configured.
    MissingRepository(&'static str),
}

impl std::fmt::Display for ContainerBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerBuilderError::MissingRepository(name) => {
                write!(f, "{} not configured", name)
            }
        }
    }
}

impl std::error::Error for ContainerBuilderError {}
