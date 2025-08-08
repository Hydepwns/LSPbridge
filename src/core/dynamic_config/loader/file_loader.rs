//! File-based configuration loader

use super::{ConfigLoader, DynamicConfig};
use crate::core::errors::{ConfigError, FileError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// File-based configuration loader
pub struct FileLoader {
    file_path: PathBuf,
}

impl FileLoader {
    /// Create a new file loader
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    /// Get the file path
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

#[async_trait]
impl ConfigLoader for FileLoader {
    async fn load(&self) -> Result<DynamicConfig, ConfigError> {
        debug!("Loading config from file: {}", self.file_path.display());
        
        let content = fs::read_to_string(&self.file_path)
            .await
            .map_err(|_e| ConfigError::FileNotFound {
                path: self.file_path.clone(),
            })?;

        let config: DynamicConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::ValidationFailed {
                reason: format!("Failed to parse config file: {e}"),
            })?;

        info!("Successfully loaded config from file: {}", self.file_path.display());
        Ok(config)
    }

    async fn save(&self, config: &DynamicConfig) -> Result<(), ConfigError> {
        debug!("Saving config to file: {}", self.file_path.display());
        
        let content = toml::to_string_pretty(config)
            .map_err(|e| ConfigError::ValidationFailed {
                reason: format!("Failed to serialize config: {e}"),
            })?;

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| FileError::DirectoryError {
                    path: parent.to_path_buf(),
                    operation: "create_dir_all".to_string(),
                    source: e,
                })?;
        }

        fs::write(&self.file_path, content)
            .await
            .map_err(|e| ConfigError::from(FileError::write_error(self.file_path.clone(), e)))?;

        info!("Successfully saved config to file: {}", self.file_path.display());
        Ok(())
    }

    async fn exists(&self) -> bool {
        self.file_path.exists()
    }

    fn loader_type(&self) -> &'static str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_loader_save_and_load() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");
        
        let loader = FileLoader::new(config_file.clone());
        
        // Should not exist initially
        assert!(!loader.exists().await);
        
        // Save default config
        let config = DynamicConfig::default();
        loader.save(&config).await?;
        
        // Should exist now
        assert!(loader.exists().await);
        
        // Load and verify
        let loaded_config = loader.load().await?;
        assert_eq!(loaded_config.processing.parallel_processing, config.processing.parallel_processing);
        assert_eq!(loaded_config.memory.max_memory_mb, config.memory.max_memory_mb);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_file_loader_nonexistent_file() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("nonexistent.toml");
        
        let loader = FileLoader::new(config_file);
        
        // Should not exist
        assert!(!loader.exists().await);
        
        // Loading should fail
        assert!(loader.load().await.is_err());
        
        Ok(())
    }
}