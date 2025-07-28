use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

/// Common file operation utilities with standardized error handling
pub struct FileUtils;

impl FileUtils {
    /// Read file with context information for better error messages
    pub async fn read_with_context(path: &Path, context: &str) -> Result<String> {
        fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read {}: {}", context, path.display()))
    }

    /// Write file with context information for better error messages  
    pub async fn write_with_context(path: &Path, content: &str, context: &str) -> Result<()> {
        fs::write(path, content)
            .await
            .with_context(|| format!("Failed to write {}: {}", context, path.display()))
    }

    /// Read file synchronously with context
    pub fn read_sync_with_context(path: &Path, context: &str) -> Result<String> {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}: {}", context, path.display()))
    }

    /// Write file synchronously with context
    pub fn write_sync_with_context(path: &Path, content: &str, context: &str) -> Result<()> {
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write {}: {}", context, path.display()))
    }

    /// Ensure parent directory exists before writing
    pub async fn ensure_parent_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }
        Ok(())
    }

    /// Create directory with context
    pub async fn create_dir_with_context(path: &Path, context: &str) -> Result<()> {
        fs::create_dir_all(path)
            .await
            .with_context(|| format!("Failed to create {}: {}", context, path.display()))
    }
}

/// Common serialization utilities
pub struct SerdeUtils;

impl SerdeUtils {
    /// Serialize to JSON with pretty formatting
    pub fn to_json_pretty<T: serde::Serialize>(value: &T) -> Result<String> {
        serde_json::to_string_pretty(value).context("Failed to serialize to JSON")
    }

    /// Deserialize from JSON with better error context
    pub fn from_json<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
        serde_json::from_str(json).context("Failed to deserialize from JSON")
    }

    /// Read and deserialize JSON file
    pub async fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
        let content = FileUtils::read_with_context(path, "JSON file").await?;
        Self::from_json(&content)
    }

    /// Serialize and write JSON file
    pub async fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
        let content = Self::to_json_pretty(value)?;
        FileUtils::ensure_parent_dir(path).await?;
        FileUtils::write_with_context(path, &content, "JSON file").await
    }
}
