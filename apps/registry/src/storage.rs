use allframe::{
    tokio::{fs, io::AsyncWriteExt},
    tracing,
};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        let full = self.root.join(path);
        fs::read(&full).await.ok()
    }

    pub async fn write_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        let full = self.root.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&full, data).await?;
        tracing::info!(path = %full.display(), bytes = data.len(), "wrote file");
        Ok(())
    }

    pub async fn append_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        let full = self.root.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&full)
            .await?;
        file.write_all(data).await?;
        tracing::info!(path = %full.display(), bytes = data.len(), "appended to file");
        Ok(())
    }

    pub async fn list_dir(&self, path: &str) -> Vec<String> {
        let full = self.root.join(path);
        let mut entries = Vec::new();
        if let Ok(mut read_dir) = fs::read_dir(&full).await {
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                if let Some(name) = entry.file_name().to_str() {
                    entries.push(name.to_string());
                }
            }
        }
        entries
    }

    /// Compute Cargo sparse index path from crate name length.
    pub fn cargo_index_path(name: &str) -> String {
        match name.len() {
            1 => format!("1/{name}"),
            2 => format!("2/{name}"),
            3 => format!("3/{}/{name}", &name[..1]),
            _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
        }
    }
}
