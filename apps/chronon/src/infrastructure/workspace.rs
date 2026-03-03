use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use allsource_core::embedded::{Config, EmbeddedCore};

use super::{core_task_repo::CoreTaskRepository, projection::TaskProjection};
use crate::domain::error::{ChronError, CoreError};

const CHRONON_DIR: &str = ".chronon";

pub struct Workspace {
    pub root: PathBuf,
    core: Arc<EmbeddedCore>,
}

impl Workspace {
    /// Walk up from `start` looking for a `.chronon/` directory.
    pub fn find_root(start: &Path) -> Option<PathBuf> {
        let mut dir = start.to_path_buf();
        loop {
            if dir.join(CHRONON_DIR).is_dir() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    /// Discover workspace from cwd and open Core with TaskProjection.
    pub async fn open() -> Result<Self, ChronError> {
        let cwd = std::env::current_dir()?;
        let root = Self::find_root(&cwd).ok_or_else(|| ChronError::NoWorkspace(cwd))?;

        let data_dir = root.join(CHRONON_DIR);
        let config = Config::builder()
            .data_dir(&data_dir)
            .single_tenant(true)
            .wal_fsync_interval_ms(100)
            .build()
            .map_err(|e| CoreError(e.to_string()))?;

        let core = EmbeddedCore::open(config)
            .await
            .map_err(|e| CoreError(e.to_string()))?;

        let core = Arc::new(core);

        // Register TaskProjection with backfill so we rebuild state from WAL
        core.inner()
            .register_projection_with_backfill(Arc::new(TaskProjection::new()))
            .map_err(|e| CoreError(e.to_string()))?;

        Ok(Self { root, core })
    }

    /// Build a `CoreTaskRepository` from this workspace's Core instance.
    pub fn repo(&self) -> CoreTaskRepository {
        CoreTaskRepository::new(Arc::clone(&self.core))
    }

    pub async fn shutdown(self) -> Result<(), ChronError> {
        // Arc::into_inner only works if this is the last Arc reference.
        // The repo should be dropped before shutdown is called.
        match Arc::into_inner(self.core) {
            Some(core) => core
                .shutdown()
                .await
                .map_err(|e| CoreError(e.to_string()).into()),
            None => Ok(()),
        }
    }
}

/// Create a new .chronon/ workspace in the given directory.
pub fn init_workspace(path: &Path) -> Result<(), ChronError> {
    let chronon_dir = path.join(CHRONON_DIR);
    if chronon_dir.exists() {
        return Err(ChronError::WorkspaceExists(path.display().to_string()));
    }
    std::fs::create_dir_all(&chronon_dir)?;

    let config_path = chronon_dir.join("config.toml");
    std::fs::write(&config_path, "# Chronon workspace config\n")?;

    println!("Initialized chronon workspace in {}", chronon_dir.display());
    Ok(())
}
