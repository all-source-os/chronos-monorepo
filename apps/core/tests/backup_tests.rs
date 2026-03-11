/// Backup system tests
use allsource_core::{
    backup::{BackupConfig, BackupManager, BackupType},
    domain::entities::Event,
};
use flate2::Compression;
use serde_json::json;
use tempfile::TempDir;

fn create_test_event(entity_id: &str, event_type: &str, i: usize) -> Event {
    Event::from_strings(
        event_type.to_string(),
        entity_id.to_string(),
        "test-tenant".to_string(),
        json!({"index": i, "data": format!("test-{}", i)}),
        None,
    )
    .unwrap()
}

#[test]
fn test_backup_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::default(),
        verify_after_backup: true,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create test events
    let events: Vec<Event> = (0..100)
        .map(|i| create_test_event(&format!("entity-{}", i % 10), &format!("test.event.{i}"), i))
        .collect();

    // Create backup
    let metadata = manager.create_backup(&events).unwrap();

    // Verify metadata
    assert_eq!(metadata.event_count, 100);
    assert!(metadata.size_bytes > 0);
    assert!(!metadata.checksum.is_empty());
    assert!(!metadata.backup_id.is_empty());
    assert!(metadata.compressed);
}

#[test]
fn test_backup_restore() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::default(),
        verify_after_backup: true,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create test events
    let events: Vec<Event> = (0..50)
        .map(|i| create_test_event(&format!("entity-{i}"), "test.event", i))
        .collect();

    // Create backup
    let metadata = manager.create_backup(&events).unwrap();

    // Restore from backup
    let restored = manager.restore_from_backup(&metadata.backup_id).unwrap();

    // Verify restored events
    assert_eq!(restored.len(), 50);
    assert_eq!(restored[0].event_type_str(), "test.event");
}

#[test]
fn test_backup_verification() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::default(),
        verify_after_backup: true,
    };

    let manager = BackupManager::new(config).unwrap();

    let events: Vec<Event> = vec![create_test_event("entity-1", "test.event", 0)];

    // Create backup
    let metadata = manager.create_backup(&events).unwrap();

    // Verify backup
    manager.verify_backup(&metadata).unwrap();
}

#[test]
fn test_backup_compression() {
    let temp_dir = TempDir::new().unwrap();

    // Test with compression
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::best(),
        verify_after_backup: false,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create events with large payloads to show compression effect
    let events: Vec<Event> = (0..10)
        .map(|i| {
            Event::from_strings(
                "test.event".to_string(),
                format!("entity-{i}"),
                "test-tenant".to_string(),
                json!({
                    "data": "x".repeat(1000),
                    "index": i
                }),
                None,
            )
            .unwrap()
        })
        .collect();

    let metadata = manager.create_backup(&events).unwrap();

    // Backup should be compressed
    assert!(metadata.compressed);
    assert!(metadata.size_bytes > 0);
}

#[test]
fn test_list_backups() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::fast(),
        verify_after_backup: false,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create multiple backups
    for i in 0..3 {
        let events: Vec<Event> = vec![create_test_event("entity-1", &format!("test.{i}"), i)];

        manager.create_backup(&events).unwrap();
    }

    // List backups
    let backups = manager.list_backups().unwrap();
    assert_eq!(backups.len(), 3);
}

#[test]
fn test_empty_backup_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::default(),
        verify_after_backup: false,
    };

    let manager = BackupManager::new(config).unwrap();

    // Create backup with no events should fail
    let events: Vec<Event> = vec![];
    let result = manager.create_backup(&events);

    assert!(result.is_err(), "Empty backup should fail");
}

#[test]
fn test_backup_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let config = BackupConfig {
        backup_dir: temp_dir.path().to_path_buf(),
        compression_level: Compression::default(),
        verify_after_backup: true,
    };

    let manager = BackupManager::new(config).unwrap();

    let events: Vec<Event> = (0..10)
        .map(|i| create_test_event("meta-entity", "meta.test", i))
        .collect();

    let metadata = manager.create_backup(&events).unwrap();

    // Verify all metadata fields
    assert!(!metadata.backup_id.is_empty());
    assert_eq!(metadata.event_count, 10);
    assert!(metadata.size_bytes > 0);
    assert!(!metadata.checksum.is_empty());
    assert_eq!(metadata.backup_type, BackupType::Full);
    assert!(metadata.compressed);
}
