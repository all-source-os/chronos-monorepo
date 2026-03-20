use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::error::ChronError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CoreMode {
    #[default]
    Embedded,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Remote Core URL used as sync target (embedded mode) or primary backend (remote mode).
    pub remote_url: Option<String>,
    /// Optional API key for authenticating with a remote Core instance.
    pub api_key: Option<String>,
}

fn default_instance_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronisConfig {
    #[serde(default)]
    pub mode: CoreMode,
    /// Unique identifier for this chronis instance (auto-generated on init).
    #[serde(default = "default_instance_id")]
    pub instance_id: String,
    #[serde(default)]
    pub sync: Option<SyncConfig>,
}

impl ChronisConfig {
    /// Load config from `.chronis/config.toml`, falling back to defaults.
    pub fn load(workspace_root: &Path) -> Result<Self, ChronError> {
        let config_path = workspace_root.join(".chronis/config.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content).map_err(|e| ChronError::Sync(format!("invalid config.toml: {e}")))
    }

    /// The remote Core URL, if configured.
    pub fn remote_url(&self) -> Option<&str> {
        self.sync.as_ref().and_then(|s| s.remote_url.as_deref())
    }

    /// The remote Core API key, if configured.
    pub fn api_key(&self) -> Option<&str> {
        self.sync.as_ref().and_then(|s| s.api_key.as_deref())
    }
}

impl Default for ChronisConfig {
    fn default() -> Self {
        Self {
            mode: CoreMode::Embedded,
            instance_id: uuid::Uuid::new_v4().to_string(),
            sync: None,
        }
    }
}
