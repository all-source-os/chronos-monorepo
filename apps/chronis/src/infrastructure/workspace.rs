use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use allsource_core::embedded::{Config, EmbeddedCore};

use super::{
    backend::CoreBackend,
    config::{ChronisConfig, CoreMode},
    core_task_repo::CoreTaskRepository,
    http_core_client::HttpCoreClient,
    projection::TaskProjection,
};
use crate::domain::error::{ChronError, CoreError};

const CHRONIS_DIR: &str = ".chronis";

pub struct Workspace {
    pub root: PathBuf,
    pub config: ChronisConfig,
    backend: Arc<CoreBackend>,
}

impl Workspace {
    /// Walk up from `start` looking for a `.chronis/` directory.
    pub fn find_root(start: &Path) -> Option<PathBuf> {
        let mut dir = start.to_path_buf();
        loop {
            if dir.join(CHRONIS_DIR).is_dir() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    /// Discover workspace from cwd and open the appropriate backend.
    pub async fn open() -> Result<Self, ChronError> {
        let cwd = std::env::current_dir()?;
        let root = Self::find_root(&cwd).ok_or(ChronError::NoWorkspace(cwd))?;
        let config = ChronisConfig::load(&root)?;

        let backend = match config.mode {
            CoreMode::Embedded => {
                let data_dir = root.join(CHRONIS_DIR);
                let core_config = Config::builder()
                    .data_dir(&data_dir)
                    .single_tenant(true)
                    .wal_fsync_interval_ms(100)
                    .build()
                    .map_err(|e| CoreError(e.to_string()))?;

                let core = EmbeddedCore::open(core_config)
                    .await
                    .map_err(|e| CoreError(e.to_string()))?;

                let core = Arc::new(core);

                core.inner()
                    .register_projection_with_backfill(
                        &(Arc::new(TaskProjection::new())
                            as Arc<dyn allsource_core::application::Projection>),
                    )
                    .map_err(|e| CoreError(e.to_string()))?;

                CoreBackend::Embedded(core)
            }
            CoreMode::Remote => {
                let url = config.remote_url().ok_or_else(|| {
                    ChronError::Sync(
                        "mode = \"remote\" requires [sync] remote_url in config.toml".into(),
                    )
                })?;
                let client = HttpCoreClient::new(url).with_api_key(config.api_key());
                client.health().await?;

                // Create an in-memory Core for local projection cache
                let local_config = Config::builder()
                    .single_tenant(true)
                    .build()
                    .map_err(|e| CoreError(e.to_string()))?;
                let local = EmbeddedCore::open(local_config)
                    .await
                    .map_err(|e| CoreError(e.to_string()))?;
                let local = Arc::new(local);

                local
                    .inner()
                    .register_projection_with_backfill(
                        &(Arc::new(TaskProjection::new())
                            as Arc<dyn allsource_core::application::Projection>),
                    )
                    .map_err(|e| CoreError(e.to_string()))?;

                // Replay all remote events into local Core for projections
                let events = client.query_events(None).await?;
                for ev in &events {
                    local
                        .ingest(allsource_core::embedded::IngestEvent {
                            entity_id: &ev.entity_id,
                            event_type: &ev.event_type,
                            payload: ev.payload.clone(),
                            metadata: ev.metadata.clone(),
                            tenant_id: Some(&ev.tenant_id),
                        })
                        .await
                        .map_err(|e| CoreError(e.to_string()))?;
                }

                CoreBackend::Remote { client, local }
            }
        };

        Ok(Self {
            root,
            config,
            backend: Arc::new(backend),
        })
    }

    /// Build a `CoreTaskRepository` from this workspace's backend.
    pub fn repo(&self) -> CoreTaskRepository {
        CoreTaskRepository::new(Arc::clone(&self.backend))
    }

    pub fn backend(&self) -> &CoreBackend {
        &self.backend
    }

    pub async fn shutdown(self) -> Result<(), ChronError> {
        let Some(backend) = Arc::into_inner(self.backend) else {
            // Other references still alive (e.g., leaked repo) — can't take ownership.
            // Not an error: the EmbeddedCore will flush on drop anyway.
            return Ok(());
        };
        match backend {
            CoreBackend::Embedded(core) => match Arc::into_inner(core) {
                Some(core) => core
                    .shutdown()
                    .await
                    .map_err(|e| CoreError(e.to_string()).into()),
                None => Ok(()),
            },
            CoreBackend::Remote { .. } => Ok(()),
        }
    }
}

/// Create a new .chronis/ workspace in the given directory.
pub fn init_workspace(path: &Path) -> Result<(), ChronError> {
    init_workspace_inner(path, None, None)
}

/// Create a new .chronis/ workspace pre-configured for remote sync.
pub fn init_workspace_with_remote(
    path: &Path,
    remote_url: &str,
    api_key: Option<&str>,
) -> Result<(), ChronError> {
    init_workspace_inner(path, Some(remote_url), api_key)
}

fn init_workspace_inner(
    path: &Path,
    remote_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<(), ChronError> {
    let chronis_dir = path.join(CHRONIS_DIR);
    if chronis_dir.exists() {
        return Err(ChronError::WorkspaceExists(path.display().to_string()));
    }
    std::fs::create_dir_all(&chronis_dir)?;

    let sync = remote_url.map(|url| super::config::SyncConfig {
        remote_url: Some(url.to_string()),
        api_key: api_key.map(String::from),
    });

    let config = ChronisConfig {
        sync,
        ..ChronisConfig::default()
    };

    let config_content = toml::to_string_pretty(&config)
        .map_err(|e| ChronError::Sync(format!("serialize config: {e}")))?;

    let config_path = chronis_dir.join("config.toml");
    std::fs::write(&config_path, config_content)?;

    // .gitignore: track config, ignore data and sync state
    let gitignore_path = chronis_dir.join(".gitignore");
    std::fs::write(&gitignore_path, "wal/\nstorage/\nsync/\n")?;

    println!("Initialized chronis workspace in {}", chronis_dir.display());
    if remote_url.is_some() {
        println!("Remote sync configured. Run `cn sync` to push/pull events.");
    }
    Ok(())
}
