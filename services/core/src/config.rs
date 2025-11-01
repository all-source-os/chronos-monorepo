/// Configuration management for AllSource v1.0
///
/// Features:
/// - Environment-based configuration
/// - TOML file support
/// - Runtime configuration validation
/// - Hot-reloading support (via file watcher)
/// - Secure credential handling

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::error::{AllSourceError, Result};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfigFile,
    pub backup: BackupConfigFile,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfigFile::default(),
            backup: BackupConfigFile::default(),
            metrics: MetricsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub max_connections: usize,
    pub request_timeout_secs: u64,
    pub cors_enabled: bool,
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3900,
            workers: None, // Use number of CPUs
            max_connections: 10_000,
            request_timeout_secs: 30,
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub wal_dir: PathBuf,
    pub batch_size: usize,
    pub compression: CompressionType,
    pub retention_days: Option<u32>,
    pub max_storage_gb: Option<u32>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            wal_dir: PathBuf::from("./wal"),
            batch_size: 1000,
            compression: CompressionType::Lz4,
            retention_days: None,
            max_storage_gb: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    Lz4,
    Gzip,
    Snappy,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub api_key_expiry_days: Option<i64>,
    pub password_min_length: usize,
    pub require_email_verification: bool,
    pub session_timeout_minutes: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "CHANGE_ME_IN_PRODUCTION".to_string(),
            jwt_expiry_hours: 24,
            api_key_expiry_days: Some(90),
            password_min_length: 8,
            require_email_verification: false,
            session_timeout_minutes: 60,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfigFile {
    pub enabled: bool,
    pub default_tier: RateLimitTier,
    pub requests_per_minute: Option<u32>,
    pub burst_size: Option<u32>,
}

impl Default for RateLimitConfigFile {
    fn default() -> Self {
        Self {
            enabled: true,
            default_tier: RateLimitTier::Professional,
            requests_per_minute: None,
            burst_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitTier {
    Free,
    Professional,
    Unlimited,
    Custom,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfigFile {
    pub enabled: bool,
    pub backup_dir: PathBuf,
    pub schedule_cron: Option<String>,
    pub retention_count: usize,
    pub compression_level: u8,
    pub verify_after_backup: bool,
}

impl Default for BackupConfigFile {
    fn default() -> Self {
        Self {
            enabled: false,
            backup_dir: PathBuf::from("./backups"),
            schedule_cron: None, // e.g., "0 2 * * *" for 2am daily
            retention_count: 7,
            compression_level: 6,
            verify_after_backup: true,
        }
    }
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub push_interval_secs: Option<u64>,
    pub push_gateway_url: Option<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
            push_interval_secs: None,
            push_gateway_url: None,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub format: LogFormat,
    pub output: LogOutput,
    pub file_path: Option<PathBuf>,
    pub rotate_size_mb: Option<u64>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
            file_path: None,
            rotate_size_mb: Some(100),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    Stdout,
    Stderr,
    File,
    Both,
}

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| AllSourceError::StorageError(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid config format: {}", e)))
    }

    /// Load configuration from environment variables
    /// Variables are prefixed with ALLSOURCE_
    pub fn from_env() -> Result<Self> {
        let mut config = Config::default();

        // Server
        if let Ok(host) = std::env::var("ALLSOURCE_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("ALLSOURCE_PORT") {
            config.server.port = port.parse()
                .map_err(|_| AllSourceError::ValidationError("Invalid port number".to_string()))?;
        }

        // Storage
        if let Ok(data_dir) = std::env::var("ALLSOURCE_DATA_DIR") {
            config.storage.data_dir = PathBuf::from(data_dir);
        }

        // Auth
        if let Ok(jwt_secret) = std::env::var("ALLSOURCE_JWT_SECRET") {
            config.auth.jwt_secret = jwt_secret;
        }

        Ok(config)
    }

    /// Load configuration with fallback priority:
    /// 1. Config file (if provided)
    /// 2. Environment variables
    /// 3. Defaults
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let mut config = if let Some(path) = config_path {
            if path.exists() {
                tracing::info!("Loading config from: {}", path.display());
                Self::from_file(path)?
            } else {
                tracing::warn!("Config file not found: {}, using defaults", path.display());
                Config::default()
            }
        } else {
            Config::default()
        };

        // Override with environment variables
        if let Ok(env_config) = Self::from_env() {
            config.merge_env(env_config);
        }

        config.validate()?;

        Ok(config)
    }

    /// Merge environment variable overrides
    fn merge_env(&mut self, env_config: Config) {
        // Merge server config
        if env_config.server.host != ServerConfig::default().host {
            self.server.host = env_config.server.host;
        }
        if env_config.server.port != ServerConfig::default().port {
            self.server.port = env_config.server.port;
        }

        // Merge storage config
        if env_config.storage.data_dir != StorageConfig::default().data_dir {
            self.storage.data_dir = env_config.storage.data_dir;
        }

        // Merge auth config
        if env_config.auth.jwt_secret != AuthConfig::default().jwt_secret {
            self.auth.jwt_secret = env_config.auth.jwt_secret;
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate port
        if self.server.port == 0 {
            return Err(AllSourceError::ValidationError(
                "Server port cannot be 0".to_string(),
            ));
        }

        // Validate JWT secret in production
        if self.auth.jwt_secret == "CHANGE_ME_IN_PRODUCTION" {
            tracing::warn!("⚠️  Using default JWT secret - INSECURE for production!");
        }

        // Validate storage paths
        if self.storage.data_dir.as_os_str().is_empty() {
            return Err(AllSourceError::ValidationError(
                "Data directory path cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Save configuration to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml = toml::to_string_pretty(self)
            .map_err(|e| AllSourceError::ValidationError(format!("Failed to serialize config: {}", e)))?;

        fs::write(path.as_ref(), toml)
            .map_err(|e| AllSourceError::StorageError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Generate example configuration file
    pub fn example() -> String {
        toml::to_string_pretty(&Config::default()).unwrap_or_else(|_| String::from("# Failed to generate example config"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8080);
        assert!(config.rate_limit.enabled);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let mut config = Config::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml).unwrap();
        assert_eq!(config.server.port, deserialized.server.port);
    }
}
