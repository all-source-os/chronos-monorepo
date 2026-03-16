use chronis::infrastructure::config::{ChronisConfig, CoreMode};

#[test]
fn default_config_is_embedded() {
    let config = ChronisConfig::default();
    assert_eq!(config.mode, CoreMode::Embedded);
    assert!(!config.instance_id.is_empty());
    assert!(config.sync.is_none());
    assert!(config.remote_url().is_none());
}

#[test]
fn parse_minimal_config() {
    let toml = "# just a comment\n";
    let config: ChronisConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.mode, CoreMode::Embedded);
    // instance_id should be auto-generated when missing
    assert!(!config.instance_id.is_empty());
}

#[test]
fn parse_embedded_with_sync() {
    let toml = r#"
mode = "embedded"
instance_id = "test-instance-123"

[sync]
remote_url = "http://localhost:3900"
"#;
    let config: ChronisConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.mode, CoreMode::Embedded);
    assert_eq!(config.instance_id, "test-instance-123");
    assert_eq!(config.remote_url(), Some("http://localhost:3900"));
}

#[test]
fn parse_remote_mode() {
    let toml = r#"
mode = "remote"
instance_id = "remote-node"

[sync]
remote_url = "https://core.example.com:3900"
"#;
    let config: ChronisConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.mode, CoreMode::Remote);
    assert_eq!(config.remote_url(), Some("https://core.example.com:3900"));
}

#[test]
fn parse_sync_without_url() {
    let toml = r#"
mode = "embedded"
instance_id = "abc"

[sync]
"#;
    let config: ChronisConfig = toml::from_str(toml).unwrap();
    assert!(config.remote_url().is_none());
}

#[test]
fn config_roundtrips_through_toml() {
    let original = ChronisConfig::default();
    let serialized = toml::to_string_pretty(&original).unwrap();
    let deserialized: ChronisConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.mode, original.mode);
    assert_eq!(deserialized.instance_id, original.instance_id);
}

#[test]
fn load_from_nonexistent_path_returns_default() {
    let config = ChronisConfig::load(std::path::Path::new("/nonexistent/path")).unwrap();
    assert_eq!(config.mode, CoreMode::Embedded);
}

#[test]
fn load_from_temp_dir_with_config() {
    let dir = tempfile::tempdir().unwrap();
    let chronis_dir = dir.path().join(".chronis");
    std::fs::create_dir_all(&chronis_dir).unwrap();
    std::fs::write(
        chronis_dir.join("config.toml"),
        r#"
mode = "remote"
instance_id = "from-file"

[sync]
remote_url = "http://test:3900"
"#,
    )
    .unwrap();

    let config = ChronisConfig::load(dir.path()).unwrap();
    assert_eq!(config.mode, CoreMode::Remote);
    assert_eq!(config.instance_id, "from-file");
    assert_eq!(config.remote_url(), Some("http://test:3900"));
}

#[test]
fn each_default_config_gets_unique_instance_id() {
    let a = ChronisConfig::default();
    let b = ChronisConfig::default();
    assert_ne!(a.instance_id, b.instance_id);
}
