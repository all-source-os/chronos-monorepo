/// Additional configuration tests for comprehensive coverage

use allsource_core::config::{Config, ServerConfig, StorageConfig, AuthConfig};
use std::env;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_config_from_toml() {
    let toml_content = r#"
[server]
host = "0.0.0.0"
port = 9090

[storage]
data_dir = "/tmp/test"
wal_sync_interval_ms = 100
parquet_batch_size = 5000

[auth]
jwt_secret = "test-secret"
token_expiration_hours = 24
api_key_length = 32
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();

    let config = Config::from_file(temp_file.path()).unwrap();

    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 9090);
    assert_eq!(config.storage.data_dir, "/tmp/test");
    assert_eq!(config.storage.wal_sync_interval_ms, 100);
    assert_eq!(config.storage.parquet_batch_size, 5000);
    assert_eq!(config.auth.jwt_secret, "test-secret");
    assert_eq!(config.auth.token_expiration_hours, 24);
    assert_eq!(config.auth.api_key_length, 32);
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert!(config.storage.data_dir.ends_with("data"));
    assert_eq!(config.storage.wal_sync_interval_ms, 1000);
    assert_eq!(config.storage.parquet_batch_size, 10000);
}

#[test]
fn test_config_env_override() {
    // Set environment variables
    env::set_var("ALLSOURCE_HOST", "192.168.1.1");
    env::set_var("ALLSOURCE_PORT", "8888");
    env::set_var("ALLSOURCE_JWT_SECRET", "env-secret");
    env::set_var("ALLSOURCE_DATA_DIR", "/custom/path");

    let config = Config::from_env().unwrap();

    assert_eq!(config.server.host, "192.168.1.1");
    assert_eq!(config.server.port, 8888);
    assert_eq!(config.auth.jwt_secret, "env-secret");
    assert_eq!(config.storage.data_dir, "/custom/path");

    // Clean up
    env::remove_var("ALLSOURCE_HOST");
    env::remove_var("ALLSOURCE_PORT");
    env::remove_var("ALLSOURCE_JWT_SECRET");
    env::remove_var("ALLSOURCE_DATA_DIR");
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();

    // Valid config should pass
    assert!(config.validate().is_ok());

    // Invalid port should fail
    config.server.port = 0;
    assert!(config.validate().is_err());

    config.server.port = 8080;

    // Empty JWT secret should fail
    config.auth.jwt_secret = "".to_string();
    assert!(config.validate().is_err());

    config.auth.jwt_secret = "valid-secret".to_string();

    // Invalid batch size should fail
    config.storage.parquet_batch_size = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_example_generation() {
    let example = Config::example();

    // Should be valid TOML
    assert!(toml::from_str::<toml::Value>(&example).is_ok());

    // Should contain expected sections
    assert!(example.contains("[server]"));
    assert!(example.contains("[storage]"));
    assert!(example.contains("[auth]"));
    assert!(example.contains("[rate_limit]"));
    assert!(example.contains("[backup]"));
    assert!(example.contains("[metrics]"));
    assert!(example.contains("[logging]"));
}

#[test]
fn test_config_load_with_fallback() {
    // Try to load non-existent file, should fall back to defaults
    let result = Config::load(Some("/nonexistent/path/config.toml".into()));

    // Should still return a config (with defaults)
    assert!(result.is_ok());
}

#[test]
fn test_server_config() {
    let config = ServerConfig::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert!(config.cors_enabled);
}

#[test]
fn test_storage_config() {
    let config = StorageConfig::default();

    assert!(config.data_dir.ends_with("data"));
    assert_eq!(config.wal_sync_interval_ms, 1000);
    assert_eq!(config.parquet_batch_size, 10000);
    assert!(config.parquet_compression_enabled);
}

#[test]
fn test_auth_config() {
    let config = AuthConfig::default();

    assert_eq!(config.jwt_secret, "change-me-in-production");
    assert_eq!(config.token_expiration_hours, 24);
    assert_eq!(config.api_key_length, 32);
    assert_eq!(config.password_min_length, 8);
}
