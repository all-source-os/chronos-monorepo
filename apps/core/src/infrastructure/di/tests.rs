//! Tests for the Dependency Injection container
//!
//! Verifies that the container correctly wires up all dependencies
//! and that use cases can be created and used.

use super::{ContainerBuilder, ServiceContainer};
use crate::domain::value_objects::EntityId;
use std::sync::Arc;

#[test]
fn test_builder_with_in_memory_repositories() {
    let container = ContainerBuilder::new()
        .with_in_memory_repositories()
        .build();

    // Verify all repositories are accessible
    let _creator_repo = container.creator_repository();
    let _article_repo = container.article_repository();
    let _transaction_repo = container.transaction_repository();
    let _access_token_repo = container.access_token_repository();
    let _fork_repo = container.fork_repository();
    let _event_stream_repo = container.event_stream_repository();
}

#[test]
fn test_container_is_clone() {
    let container = ContainerBuilder::new()
        .with_in_memory_repositories()
        .build();

    let cloned = container.clone();

    // Both containers should provide working repositories
    let _repo1 = container.creator_repository();
    let _repo2 = cloned.creator_repository();
}

#[test]
fn test_container_is_debug() {
    let container = ContainerBuilder::new()
        .with_in_memory_repositories()
        .build();

    let debug_str = format!("{:?}", container);
    assert!(debug_str.contains("ServiceContainer"));
    assert!(debug_str.contains("creator_repository"));
    assert!(debug_str.contains("event_stream_repository"));
}

#[test]
fn test_use_case_factories() {
    let container = ContainerBuilder::new()
        .with_in_memory_repositories()
        .build();

    // Creator use cases
    let _register_creator = container.register_creator_use_case();
    let _update_creator = container.update_creator_use_case();

    // Article use cases (stateful ones only)
    let _create_article = container.create_article_use_case();
    let _update_article = container.update_article_use_case();

    // Payment use cases
    let _initiate_payment = container.initiate_payment_use_case();
    let _confirm_transaction = container.confirm_transaction_use_case();
    let _refund_transaction = container.refund_transaction_use_case();

    // Access token use cases
    let _grant_free_access = container.grant_free_access_use_case();
    let _validate_token = container.validate_token_use_case();
    let _revoke_access = container.revoke_access_use_case();
    let _check_access = container.check_access_use_case();
    let _cleanup_tokens = container.cleanup_expired_tokens_use_case();

    // Fork use cases
    let _create_fork = container.create_fork_use_case();
    let _update_fork = container.update_fork_use_case();
    let _merge_fork = container.merge_fork_use_case();
    let _discard_fork = container.discard_fork_use_case();
    let _get_fork = container.get_fork_use_case();
    let _append_fork_event = container.append_fork_event_use_case();
    let _query_fork_events = container.query_fork_events_use_case();
    let _branch_fork = container.branch_fork_use_case();
    let _cleanup_forks = container.cleanup_expired_forks_use_case();
}

#[test]
fn test_repositories_are_shared() {
    let container = ContainerBuilder::new()
        .with_in_memory_repositories()
        .build();

    // Get the same repository twice
    let repo1 = container.creator_repository();
    let repo2 = container.creator_repository();

    // They should point to the same underlying instance
    // (Arc pointer equality)
    assert!(Arc::ptr_eq(&repo1, &repo2));
}

#[test]
fn test_try_build_success() {
    let result = ContainerBuilder::new()
        .with_in_memory_repositories()
        .try_build();

    assert!(result.is_ok());
}

#[test]
fn test_try_build_missing_repository() {
    let result = ContainerBuilder::new().try_build();

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("not configured"));
}

#[tokio::test]
async fn test_event_stream_repository_works() {
    let container = ContainerBuilder::new()
        .with_in_memory_repositories()
        .build();

    let repo = container.event_stream_repository();
    let entity_id = EntityId::new("test-entity-1".to_string()).unwrap();

    // Should be able to create a stream
    let stream = repo.get_or_create_stream(&entity_id).await.unwrap();
    assert_eq!(stream.current_version(), 0);

    // Count should be 1
    let count = repo.count_streams().await.unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_container_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ServiceContainer>();
}
