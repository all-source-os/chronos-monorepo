//! Service Container implementation
//!
//! Holds all repository instances and provides factory methods for use cases.
//!
//! Note: Stateless use cases (unit structs with no constructor) are not included
//! in the container - they should be used directly via their static methods.

use crate::application::use_cases::{
    // Creator use cases (with state)
    RegisterCreatorUseCase, UpdateCreatorUseCase,
    // Article use cases (with state)
    CreateArticleUseCase, UpdateArticleUseCase,
    // Payment use cases (with state)
    InitiatePaymentUseCase, ConfirmTransactionUseCase, RefundTransactionUseCase,
    // Access use cases (with state)
    GrantFreeAccessUseCase, ValidateTokenUseCase, RevokeAccessUseCase, CheckAccessUseCase,
    CleanupExpiredTokensUseCase,
    // Fork use cases (with state)
    CreateForkUseCase, UpdateForkUseCase, MergeForkUseCase, DiscardForkUseCase,
    GetForkUseCase, AppendForkEventUseCase, QueryForkEventsUseCase, BranchForkUseCase,
    CleanupExpiredForksUseCase,
};
use crate::domain::repositories::{
    AccessTokenRepository, ArticleRepository, CreatorRepository, EventStreamRepository,
    ForkRepository, TransactionRepository,
};
use std::sync::Arc;

/// Service container that holds all repository instances and provides factory methods for use cases.
///
/// The container uses `Arc` for thread-safe sharing of repository instances across use cases.
/// All repositories are stored as trait objects, allowing for easy substitution with mock
/// implementations during testing.
///
/// # Design Principles
///
/// 1. **Singleton Repositories**: Repository instances are created once and shared across all use cases
/// 2. **Factory Methods**: Use cases are created on-demand with their dependencies injected
/// 3. **Thread Safety**: All repositories are wrapped in `Arc` for concurrent access
/// 4. **Testability**: Repositories can be swapped with mocks via the builder
///
/// # Stateless Use Cases
///
/// Some use cases are stateless (unit structs with static methods). These are NOT included
/// in the container and should be used directly:
/// - `PublishArticleUseCase::execute(article)`
/// - `ArchiveArticleUseCase::execute(article)`
/// - `DeleteArticleUseCase::execute(article)`
/// - `RestoreArticleUseCase::execute(article)`
/// - `RecordArticlePurchaseUseCase::execute(article, amount)`
/// - `ExtendAccessUseCase::execute(token, days)`
/// - And others...
///
/// # Example
///
/// ```rust,ignore
/// let container = ContainerBuilder::new()
///     .with_in_memory_repositories()
///     .build();
///
/// // Get use cases with dependencies injected
/// let register_creator = container.register_creator_use_case();
/// let initiate_payment = container.initiate_payment_use_case();
/// ```
#[derive(Clone)]
pub struct ServiceContainer {
    // Paywall domain repositories
    creator_repository: Arc<dyn CreatorRepository>,
    article_repository: Arc<dyn ArticleRepository>,
    transaction_repository: Arc<dyn TransactionRepository>,
    access_token_repository: Arc<dyn AccessTokenRepository>,
    fork_repository: Arc<dyn ForkRepository>,
    // Core event sourcing repository
    event_stream_repository: Arc<dyn EventStreamRepository>,
}

impl ServiceContainer {
    /// Creates a new service container with the provided repositories.
    ///
    /// Prefer using `ContainerBuilder` for a more fluent API.
    pub fn new(
        creator_repository: Arc<dyn CreatorRepository>,
        article_repository: Arc<dyn ArticleRepository>,
        transaction_repository: Arc<dyn TransactionRepository>,
        access_token_repository: Arc<dyn AccessTokenRepository>,
        fork_repository: Arc<dyn ForkRepository>,
        event_stream_repository: Arc<dyn EventStreamRepository>,
    ) -> Self {
        Self {
            creator_repository,
            article_repository,
            transaction_repository,
            access_token_repository,
            fork_repository,
            event_stream_repository,
        }
    }

    // =========================================================================
    // Repository Accessors
    // =========================================================================

    /// Returns the creator repository instance.
    pub fn creator_repository(&self) -> Arc<dyn CreatorRepository> {
        self.creator_repository.clone()
    }

    /// Returns the article repository instance.
    pub fn article_repository(&self) -> Arc<dyn ArticleRepository> {
        self.article_repository.clone()
    }

    /// Returns the transaction repository instance.
    pub fn transaction_repository(&self) -> Arc<dyn TransactionRepository> {
        self.transaction_repository.clone()
    }

    /// Returns the access token repository instance.
    pub fn access_token_repository(&self) -> Arc<dyn AccessTokenRepository> {
        self.access_token_repository.clone()
    }

    /// Returns the fork repository instance.
    pub fn fork_repository(&self) -> Arc<dyn ForkRepository> {
        self.fork_repository.clone()
    }

    /// Returns the event stream repository instance.
    pub fn event_stream_repository(&self) -> Arc<dyn EventStreamRepository> {
        self.event_stream_repository.clone()
    }

    // =========================================================================
    // Creator Use Case Factories
    // =========================================================================

    /// Creates a new `RegisterCreatorUseCase` with injected dependencies.
    pub fn register_creator_use_case(&self) -> RegisterCreatorUseCase {
        RegisterCreatorUseCase::new(self.creator_repository.clone())
    }

    /// Creates a new `UpdateCreatorUseCase` with injected dependencies.
    pub fn update_creator_use_case(&self) -> UpdateCreatorUseCase {
        UpdateCreatorUseCase::new(self.creator_repository.clone())
    }

    // =========================================================================
    // Article Use Case Factories
    // =========================================================================

    /// Creates a new `CreateArticleUseCase` with injected dependencies.
    pub fn create_article_use_case(&self) -> CreateArticleUseCase {
        CreateArticleUseCase::new(self.article_repository.clone())
    }

    /// Creates a new `UpdateArticleUseCase` with injected dependencies.
    pub fn update_article_use_case(&self) -> UpdateArticleUseCase {
        UpdateArticleUseCase::new(self.article_repository.clone())
    }

    // Note: PublishArticleUseCase, ArchiveArticleUseCase, DeleteArticleUseCase,
    // RestoreArticleUseCase, and RecordArticlePurchaseUseCase are stateless.
    // Use them directly: e.g., PublishArticleUseCase::execute(article)

    // =========================================================================
    // Payment Use Case Factories
    // =========================================================================

    /// Creates a new `InitiatePaymentUseCase` with injected dependencies.
    pub fn initiate_payment_use_case(&self) -> InitiatePaymentUseCase {
        InitiatePaymentUseCase::new(
            self.transaction_repository.clone(),
            self.article_repository.clone(),
            self.creator_repository.clone(),
        )
    }

    /// Creates a new `ConfirmTransactionUseCase` with injected dependencies.
    pub fn confirm_transaction_use_case(&self) -> ConfirmTransactionUseCase {
        ConfirmTransactionUseCase::new(
            self.transaction_repository.clone(),
            self.access_token_repository.clone(),
            self.article_repository.clone(),
            self.creator_repository.clone(),
        )
    }

    /// Creates a new `RefundTransactionUseCase` with injected dependencies.
    pub fn refund_transaction_use_case(&self) -> RefundTransactionUseCase {
        RefundTransactionUseCase::new(
            self.transaction_repository.clone(),
            self.access_token_repository.clone(),
        )
    }

    // Note: FailTransactionUseCase, DisputeTransactionUseCase, ResolveDisputeUseCase,
    // and ListTransactionsUseCase are stateless. Use them directly.

    // =========================================================================
    // Access Token Use Case Factories
    // =========================================================================

    /// Creates a new `GrantFreeAccessUseCase` with injected dependencies.
    pub fn grant_free_access_use_case(&self) -> GrantFreeAccessUseCase {
        GrantFreeAccessUseCase::new(
            self.access_token_repository.clone(),
            self.article_repository.clone(),
        )
    }

    /// Creates a new `ValidateTokenUseCase` with injected dependencies.
    pub fn validate_token_use_case(&self) -> ValidateTokenUseCase {
        ValidateTokenUseCase::new(self.access_token_repository.clone())
    }

    /// Creates a new `RevokeAccessUseCase` with injected dependencies.
    pub fn revoke_access_use_case(&self) -> RevokeAccessUseCase {
        RevokeAccessUseCase::new(self.access_token_repository.clone())
    }

    /// Creates a new `CheckAccessUseCase` with injected dependencies.
    pub fn check_access_use_case(&self) -> CheckAccessUseCase {
        CheckAccessUseCase::new(self.access_token_repository.clone())
    }

    /// Creates a new `CleanupExpiredTokensUseCase` with injected dependencies.
    pub fn cleanup_expired_tokens_use_case(&self) -> CleanupExpiredTokensUseCase {
        CleanupExpiredTokensUseCase::new(self.access_token_repository.clone())
    }

    // Note: ExtendAccessUseCase, RecordAccessUseCase, and ListAccessTokensUseCase
    // are stateless. Use them directly.

    // =========================================================================
    // Fork Use Case Factories
    // =========================================================================

    /// Creates a new `CreateForkUseCase` with injected dependencies.
    pub fn create_fork_use_case(&self) -> CreateForkUseCase {
        CreateForkUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `UpdateForkUseCase` with injected dependencies.
    pub fn update_fork_use_case(&self) -> UpdateForkUseCase {
        UpdateForkUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `MergeForkUseCase` with injected dependencies.
    pub fn merge_fork_use_case(&self) -> MergeForkUseCase {
        MergeForkUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `DiscardForkUseCase` with injected dependencies.
    pub fn discard_fork_use_case(&self) -> DiscardForkUseCase {
        DiscardForkUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `GetForkUseCase` with injected dependencies.
    pub fn get_fork_use_case(&self) -> GetForkUseCase {
        GetForkUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `AppendForkEventUseCase` with injected dependencies.
    pub fn append_fork_event_use_case(&self) -> AppendForkEventUseCase {
        AppendForkEventUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `QueryForkEventsUseCase` with injected dependencies.
    pub fn query_fork_events_use_case(&self) -> QueryForkEventsUseCase {
        QueryForkEventsUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `BranchForkUseCase` with injected dependencies.
    pub fn branch_fork_use_case(&self) -> BranchForkUseCase {
        BranchForkUseCase::new(self.fork_repository.clone())
    }

    /// Creates a new `CleanupExpiredForksUseCase` with injected dependencies.
    pub fn cleanup_expired_forks_use_case(&self) -> CleanupExpiredForksUseCase {
        CleanupExpiredForksUseCase::new(self.fork_repository.clone())
    }

    // Note: ListForksUseCase is stateless. Use it directly.
}

impl std::fmt::Debug for ServiceContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceContainer")
            .field("creator_repository", &"Arc<dyn CreatorRepository>")
            .field("article_repository", &"Arc<dyn ArticleRepository>")
            .field("transaction_repository", &"Arc<dyn TransactionRepository>")
            .field("access_token_repository", &"Arc<dyn AccessTokenRepository>")
            .field("fork_repository", &"Arc<dyn ForkRepository>")
            .field("event_stream_repository", &"Arc<dyn EventStreamRepository>")
            .finish()
    }
}
