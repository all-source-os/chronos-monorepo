use std::path::{Path, PathBuf};

/// Configuration for [`EmbeddedCore`](super::EmbeddedCore).
///
/// Created via [`Config::builder()`]. Defaults to in-memory, single-tenant mode.
#[derive(Debug, Clone)]
pub struct EmbeddedConfig {
    data_dir: Option<PathBuf>,
    wal_sync_on_write: bool,
    parquet_flush_interval_secs: u64,
    single_tenant: bool,
    node_id: Option<u32>,
}

impl EmbeddedConfig {
    /// Begin building a new configuration.
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Directory for durable storage (WAL + Parquet).
    /// `None` means in-memory only.
    pub fn data_dir(&self) -> Option<&Path> {
        self.data_dir.as_deref()
    }

    /// Whether the WAL syncs to disk on every write.
    pub fn wal_sync_on_write(&self) -> bool {
        self.wal_sync_on_write
    }

    /// Whether single-tenant mode is enabled.
    pub fn single_tenant(&self) -> bool {
        self.single_tenant
    }

    /// Node ID for HLC-based bidirectional sync.
    /// `None` disables sync capabilities (single-node mode).
    pub fn node_id(&self) -> Option<u32> {
        self.node_id
    }

    pub(crate) fn parquet_flush_interval_secs(&self) -> u64 {
        self.parquet_flush_interval_secs
    }
}

/// Builder for [`EmbeddedConfig`](super::Config).
pub struct ConfigBuilder {
    data_dir: Option<PathBuf>,
    wal_sync_on_write: bool,
    parquet_flush_interval_secs: u64,
    single_tenant: bool,
    node_id: Option<u32>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            data_dir: None,
            wal_sync_on_write: true,
            parquet_flush_interval_secs: 300,
            single_tenant: true,
            node_id: None,
        }
    }
}

impl ConfigBuilder {
    /// Set the directory where WAL and Parquet files are stored.
    /// When not called, EmbeddedCore runs in-memory (no durability).
    pub fn data_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.data_dir = Some(path.into());
        self
    }

    /// Control whether WAL syncs to disk on every write.
    /// Default: `true`. Set `false` for higher throughput at the cost of
    /// potential data loss on crash within the last sync window.
    pub fn wal_sync_on_write(mut self, sync: bool) -> Self {
        self.wal_sync_on_write = sync;
        self
    }

    /// How often Parquet files are flushed (in seconds). Default: 300.
    pub fn parquet_flush_interval_secs(mut self, secs: u64) -> Self {
        self.parquet_flush_interval_secs = secs;
        self
    }

    /// Enable or disable single-tenant mode. Default: `true`.
    /// In single-tenant mode, all events automatically use "default" as tenant_id.
    pub fn single_tenant(mut self, enabled: bool) -> Self {
        self.single_tenant = enabled;
        self
    }

    /// Set a node ID for HLC-based bidirectional sync.
    /// Each instance in a sync group must have a unique node ID.
    /// When not set, sync capabilities are disabled (single-node mode).
    pub fn node_id(mut self, id: u32) -> Self {
        self.node_id = Some(id);
        self
    }

    /// Build the configuration. Returns `Result` for forward compatibility.
    pub fn build(self) -> crate::error::Result<EmbeddedConfig> {
        Ok(EmbeddedConfig {
            data_dir: self.data_dir,
            wal_sync_on_write: self.wal_sync_on_write,
            parquet_flush_interval_secs: self.parquet_flush_interval_secs,
            single_tenant: self.single_tenant,
            node_id: self.node_id,
        })
    }
}
