//! TDD tests for EmbeddedCore backup/restore API (GitHub issue #102).
//!
//! Run with: cargo test --features embedded --test embedded_backup_restore

#[cfg(feature = "embedded")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
    use serde_json::json;
    use tempfile::TempDir;

    async fn core_with_events(data_dir: &std::path::Path, n: usize) -> EmbeddedCore {
        let core = EmbeddedCore::open(Config::builder().data_dir(data_dir).build().unwrap())
            .await
            .unwrap();

        for i in 0..n {
            core.ingest(IngestEvent {
                entity_id: &format!("entity-{i}"),
                event_type: "test.created",
                payload: json!({"index": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        core
    }

    // =========================================================================
    // event_count
    // =========================================================================

    #[tokio::test]
    async fn event_count_returns_total_events() {
        let tmp = TempDir::new().unwrap();
        let core = core_with_events(tmp.path(), 5).await;
        assert_eq!(core.event_count(), 5);
        core.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn event_count_zero_on_empty_store() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap();
        assert_eq!(core.event_count(), 0);
    }

    // =========================================================================
    // create_backup
    // =========================================================================

    #[tokio::test]
    async fn create_backup_produces_metadata() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();
        let core = core_with_events(tmp.path(), 3).await;

        let metadata = core.create_backup(backup_dir.path()).await.unwrap();
        assert_eq!(metadata.event_count, 3);
        assert!(!metadata.checksum.is_empty());
        assert!(metadata.size_bytes > 0);

        core.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn create_backup_fails_on_empty_store() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap();
        let backup_dir = TempDir::new().unwrap();

        let result = core.create_backup(backup_dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_multiple_backups_listed_newest_first() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();
        let core = core_with_events(tmp.path(), 2).await;

        let m1 = core.create_backup(backup_dir.path()).await.unwrap();
        // Ingest one more event so second backup has more events
        core.ingest(IngestEvent {
            entity_id: "extra",
            event_type: "test.created",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();
        let m2 = core.create_backup(backup_dir.path()).await.unwrap();

        let list = core.list_backups(backup_dir.path()).unwrap();
        assert_eq!(list.len(), 2);
        // Newest first
        assert_eq!(list[0].backup_id, m2.backup_id);
        assert_eq!(list[1].backup_id, m1.backup_id);

        core.shutdown().await.unwrap();
    }

    // =========================================================================
    // restore_from_backup
    // =========================================================================

    #[tokio::test]
    async fn restore_from_backup_recovers_all_events() {
        let data_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        // Create core, ingest events, backup
        let core = core_with_events(data_dir.path(), 5).await;
        let metadata = core.create_backup(backup_dir.path()).await.unwrap();
        core.shutdown().await.unwrap();

        // Restore into a fresh core
        let restore_dir = TempDir::new().unwrap();
        let restored = EmbeddedCore::restore(
            backup_dir.path(),
            &metadata.backup_id,
            Config::builder()
                .data_dir(restore_dir.path())
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        let events = restored.query(Query::new()).await.unwrap();
        assert_eq!(events.len(), 5);

        // Verify event content survived
        let entity_ids: Vec<&str> = events.iter().map(|e| e.entity_id.as_str()).collect();
        assert!(entity_ids.contains(&"entity-0"));
        assert!(entity_ids.contains(&"entity-4"));

        restored.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn restore_preserves_event_payloads() {
        let data_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        let core = core_with_events(data_dir.path(), 1).await;
        let metadata = core.create_backup(backup_dir.path()).await.unwrap();
        core.shutdown().await.unwrap();

        let restore_dir = TempDir::new().unwrap();
        let restored = EmbeddedCore::restore(
            backup_dir.path(),
            &metadata.backup_id,
            Config::builder()
                .data_dir(restore_dir.path())
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        let events = restored
            .query(Query::new().entity_id("entity-0"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["index"], 0);

        restored.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn restore_invalid_backup_id_fails() {
        let backup_dir = TempDir::new().unwrap();
        let restore_dir = TempDir::new().unwrap();

        let result = EmbeddedCore::restore(
            backup_dir.path(),
            "nonexistent_backup",
            Config::builder()
                .data_dir(restore_dir.path())
                .build()
                .unwrap(),
        )
        .await;

        assert!(result.is_err());
    }

    // =========================================================================
    // delete_backup
    // =========================================================================

    #[tokio::test]
    async fn delete_backup_removes_files() {
        let data_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();
        let core = core_with_events(data_dir.path(), 2).await;

        let metadata = core.create_backup(backup_dir.path()).await.unwrap();
        assert_eq!(core.list_backups(backup_dir.path()).unwrap().len(), 1);

        core.delete_backup(backup_dir.path(), &metadata.backup_id)
            .unwrap();
        assert_eq!(core.list_backups(backup_dir.path()).unwrap().len(), 0);

        core.shutdown().await.unwrap();
    }

    // =========================================================================
    // cleanup_old_backups
    // =========================================================================

    #[tokio::test]
    async fn cleanup_keeps_only_n_newest_backups() {
        let data_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();
        let core = core_with_events(data_dir.path(), 1).await;

        // Create 3 backups
        core.create_backup(backup_dir.path()).await.unwrap();
        core.create_backup(backup_dir.path()).await.unwrap();
        core.create_backup(backup_dir.path()).await.unwrap();

        assert_eq!(core.list_backups(backup_dir.path()).unwrap().len(), 3);

        let deleted = core.cleanup_old_backups(backup_dir.path(), 1).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(core.list_backups(backup_dir.path()).unwrap().len(), 1);

        core.shutdown().await.unwrap();
    }
}
