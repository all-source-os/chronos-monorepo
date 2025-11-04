/// Additional backup & restore tests for comprehensive coverage

use allsource_core::backup::{BackupConfig, BackupManager, BackupMetadata};
use allsource_core::event::Event;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn test_backup_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_enabled: true,
        verify_on_create: true,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create test events
    let events: Vec<Event> = (0..100)
        .map(|i| Event {
            id: Uuid::new_v4(),
            event_type: format!("test.event.{}", i),
            entity_id: format!("entity-{}", i % 10),
            tenant_id: "test-tenant".to_string(),
            payload: json!({"index": i}),
            timestamp: chrono::Utc::now(),
            metadata: None,
            version: 1,
        })
        .collect();

    // Create backup
    let metadata = manager.create_backup(&events).unwrap();

    // Verify metadata
    assert_eq!(metadata.event_count, 100);
    assert!(metadata.size_bytes > 0);
    assert!(metadata.compressed_size_bytes > 0);
    assert!(!metadata.checksum.is_empty());
    assert!(!metadata.backup_id.is_empty());
}

#[test]
fn test_backup_restore() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_enabled: true,
        verify_on_create: true,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create test events
    let events: Vec<Event> = (0..50)
        .map(|i| Event {
            id: Uuid::new_v4(),
            event_type: "test.event".to_string(),
            entity_id: format!("entity-{}", i),
            tenant_id: "test-tenant".to_string(),
            payload: json!({"value": i}),
            timestamp: chrono::Utc::now(),
            metadata: None,
            version: 1,
        })
        .collect();

    // Create backup
    let metadata = manager.create_backup(&events).unwrap();

    // Restore from backup
    let restored = manager.restore_from_backup(&metadata.backup_id).unwrap();

    // Verify restored events
    assert_eq!(restored.len(), 50);
    assert_eq!(restored[0].event_type, "test.event");
}

#[test]
fn test_backup_verification() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_enabled: true,
        verify_on_create: true,
    };

    let manager = BackupManager::new(config).unwrap();

    let events: Vec<Event> = vec![Event {
        id: Uuid::new_v4(),
        event_type: "test.event".to_string(),
        entity_id: "entity-1".to_string(),
        tenant_id: "test-tenant".to_string(),
        payload: json!({"test": "data"}),
        timestamp: chrono::Utc::now(),
        metadata: None,
        version: 1,
    }];

    // Create backup
    let metadata = manager.create_backup(&events).unwrap();

    // Verify backup
    manager.verify_backup(&metadata).unwrap();
}

#[test]
fn test_backup_compression() {
    let temp_dir = TempDir::new().unwrap();

    // Test with compression
    let config_compressed = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_enabled: true,
        verify_on_create: false,
    };

    let manager_compressed = BackupManager::new(config_compressed).unwrap();

    // Create large payload
    let large_payload = json!({
        "data": "x".repeat(10000),
    });

    let events: Vec<Event> = vec![Event {
        id: Uuid::new_v4(),
        event_type: "test.event".to_string(),
        entity_id: "entity-1".to_string(),
        tenant_id: "test-tenant".to_string(),
        payload: large_payload.clone(),
        timestamp: chrono::Utc::now(),
        metadata: None,
        version: 1,
    }];

    let metadata = manager_compressed.create_backup(&events).unwrap();

    // Compressed size should be smaller than original
    assert!(metadata.compressed_size_bytes < metadata.size_bytes);
}

#[test]
fn test_list_backups() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_enabled: false,
        verify_on_create: false,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create multiple backups
    for i in 0..3 {
        let events = vec![Event {
            id: Uuid::new_v4(),
            event_type: format!("test.{}", i),
            entity_id: "entity-1".to_string(),
            tenant_id: "test-tenant".to_string(),
            payload: json!({"index": i}),
            timestamp: chrono::Utc::now(),
            metadata: None,
            version: 1,
        }];

        manager.create_backup(&events).unwrap();
    }

    // List backups
    let backups = manager.list_backups().unwrap();
    assert_eq!(backups.len(), 3);
}

#[test]
fn test_empty_backup() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_enabled: false,
        verify_on_create: false,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create backup with no events
    let events: Vec<Event> = vec![];
    let metadata = manager.create_backup(&events).unwrap();

    assert_eq!(metadata.event_count, 0);

    // Restore should return empty vec
    let restored = manager.restore_from_backup(&metadata.backup_id).unwrap();
    assert_eq!(restored.len(), 0);
}
