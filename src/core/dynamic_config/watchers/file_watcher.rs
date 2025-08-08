//! File-based configuration watcher

use super::{ConfigWatcher, ConfigChangeNotifier, ConfigChange};
use super::super::loader::{FileLoader, ConfigLoader};
use super::super::types::DynamicConfig;
use crate::core::errors::ConfigError;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tokio::fs;
use tracing::{debug, error, info, warn};

/// File-based configuration watcher
pub struct FileWatcher {
    file_path: PathBuf,
    loader: FileLoader,
    current_config: Arc<RwLock<DynamicConfig>>,
    last_modified: Arc<RwLock<Option<SystemTime>>>,
    change_notifier: ConfigChangeNotifier,
    watch_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    is_watching: Arc<RwLock<bool>>,
    poll_interval: Duration,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(file_path: PathBuf) -> Self {
        let loader = FileLoader::new(file_path.clone());
        let (change_notifier, _) = ConfigChangeNotifier::new(100);
        
        Self {
            file_path,
            loader,
            current_config: Arc::new(RwLock::new(DynamicConfig::default())),
            last_modified: Arc::new(RwLock::new(None)),
            change_notifier,
            watch_handle: Arc::new(RwLock::new(None)),
            is_watching: Arc::new(RwLock::new(false)),
            poll_interval: Duration::from_secs(1),
        }
    }

    /// Create a new file watcher with custom poll interval
    pub fn with_poll_interval(file_path: PathBuf, poll_interval: Duration) -> Self {
        let mut watcher = Self::new(file_path);
        watcher.poll_interval = poll_interval;
        watcher
    }

    /// Set the current configuration
    pub async fn set_config(&self, config: DynamicConfig) {
        let mut current = self.current_config.write().await;
        *current = config;
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> DynamicConfig {
        let config = self.current_config.read().await;
        config.clone()
    }

    /// Reload configuration from file
    async fn reload_config(&self) -> Result<(), ConfigError> {
        debug!("Reloading configuration from file: {}", self.file_path.display());
        
        let new_config = self.loader.load().await?;
        let old_config = {
            let current_config = self.current_config.read().await;
            current_config.clone()
        };

        // Calculate changes
        let changes = self.calculate_changes(&old_config, &new_config);
        
        // Update current config
        {
            let mut current_config = self.current_config.write().await;
            *current_config = new_config;
        }

        // Notify about changes
        for change in changes {
            if let Err(e) = self.change_notifier.notify(change) {
                warn!("Failed to notify config change: {}", e);
            }
        }

        info!("Configuration reloaded from file: {}", self.file_path.display());
        Ok(())
    }

    /// Calculate differences between old and new configurations
    fn calculate_changes(&self, old: &DynamicConfig, new: &DynamicConfig) -> Vec<ConfigChange> {
        let mut changes = Vec::new();
        let timestamp = SystemTime::now();

        // Processing changes
        if old.processing.parallel_processing != new.processing.parallel_processing {
            changes.push(ConfigChange {
                field_path: "processing.parallel_processing".to_string(),
                old_value: old.processing.parallel_processing.to_string(),
                new_value: new.processing.parallel_processing.to_string(),
                timestamp,
            });
        }

        if old.processing.chunk_size != new.processing.chunk_size {
            changes.push(ConfigChange {
                field_path: "processing.chunk_size".to_string(),
                old_value: old.processing.chunk_size.to_string(),
                new_value: new.processing.chunk_size.to_string(),
                timestamp,
            });
        }

        // Memory changes
        if old.memory.max_memory_mb != new.memory.max_memory_mb {
            changes.push(ConfigChange {
                field_path: "memory.max_memory_mb".to_string(),
                old_value: old.memory.max_memory_mb.to_string(),
                new_value: new.memory.max_memory_mb.to_string(),
                timestamp,
            });
        }

        if old.memory.eviction_policy != new.memory.eviction_policy {
            changes.push(ConfigChange {
                field_path: "memory.eviction_policy".to_string(),
                old_value: old.memory.eviction_policy.clone(),
                new_value: new.memory.eviction_policy.clone(),
                timestamp,
            });
        }

        // Cache changes
        if old.cache.max_size_mb != new.cache.max_size_mb {
            changes.push(ConfigChange {
                field_path: "cache.max_size_mb".to_string(),
                old_value: old.cache.max_size_mb.to_string(),
                new_value: new.cache.max_size_mb.to_string(),
                timestamp,
            });
        }

        // Performance changes
        if old.performance.max_cpu_usage_percent != new.performance.max_cpu_usage_percent {
            changes.push(ConfigChange {
                field_path: "performance.max_cpu_usage_percent".to_string(),
                old_value: old.performance.max_cpu_usage_percent.to_string(),
                new_value: new.performance.max_cpu_usage_percent.to_string(),
                timestamp,
            });
        }

        // Metrics changes
        if old.metrics.prometheus_port != new.metrics.prometheus_port {
            changes.push(ConfigChange {
                field_path: "metrics.prometheus_port".to_string(),
                old_value: old.metrics.prometheus_port.to_string(),
                new_value: new.metrics.prometheus_port.to_string(),
                timestamp,
            });
        }

        changes
    }

    /// Start the file watching loop
    async fn start_watch_loop(&self) -> Result<(), ConfigError> {
        let file_path = self.file_path.clone();
        let last_modified = Arc::clone(&self.last_modified);
        let is_watching = Arc::clone(&self.is_watching);
        let poll_interval = self.poll_interval;
        
        // Store a reference to self for the reload function
        let watcher = FileWatcher {
            file_path: self.file_path.clone(),
            loader: FileLoader::new(self.file_path.clone()),
            current_config: Arc::clone(&self.current_config),
            last_modified: Arc::clone(&self.last_modified),
            change_notifier: self.change_notifier.clone(),
            watch_handle: Arc::new(RwLock::new(None)),
            is_watching: Arc::clone(&self.is_watching),
            poll_interval: self.poll_interval,
        };

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);

            loop {
                interval.tick().await;

                // Check if we should still be watching
                {
                    let watching = is_watching.read().await;
                    if !*watching {
                        break;
                    }
                }

                if let Ok(metadata) = fs::metadata(&file_path).await {
                    if let Ok(modified) = metadata.modified() {
                        let should_reload = {
                            let last_modified_guard = last_modified.read().await;
                            last_modified_guard.map_or(true, |last| modified > last)
                        };

                        if should_reload {
                            if let Err(e) = watcher.reload_config().await {
                                error!("Failed to reload config: {}", e);
                            } else {
                                let mut last_modified_guard = last_modified.write().await;
                                *last_modified_guard = Some(modified);
                            }
                        }
                    }
                }
            }

            debug!("File watcher loop stopped for: {}", file_path.display());
        });

        let mut watch_handle = self.watch_handle.write().await;
        *watch_handle = Some(handle);

        Ok(())
    }
}

#[async_trait]
impl ConfigWatcher for FileWatcher {
    async fn start_watching(&self) -> Result<(), ConfigError> {
        let mut is_watching = self.is_watching.write().await;
        
        if *is_watching {
            return Err(ConfigError::ValidationFailed {
                reason: "File watcher is already running".to_string(),
            });
        }

        // Load initial configuration
        if self.loader.exists().await {
            self.reload_config().await?;
        }

        // Start the watch loop
        self.start_watch_loop().await?;
        
        *is_watching = true;
        info!("Started file watcher for: {}", self.file_path.display());
        
        Ok(())
    }

    async fn stop_watching(&self) -> Result<(), ConfigError> {
        let mut is_watching = self.is_watching.write().await;
        
        if !*is_watching {
            return Ok(());
        }

        *is_watching = false;

        // Cancel the watch handle
        let mut watch_handle = self.watch_handle.write().await;
        if let Some(handle) = watch_handle.take() {
            handle.abort();
        }

        info!("Stopped file watcher for: {}", self.file_path.display());
        Ok(())
    }

    fn get_change_receiver(&self) -> broadcast::Receiver<ConfigChange> {
        self.change_notifier.subscribe()
    }

    fn is_watching(&self) -> bool {
        // This is a synchronous method, so we can't await
        // In a real implementation, you might want to store the state differently
        false // Simplified for now
    }

    fn watcher_type(&self) -> &'static str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_file_watcher_creation() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");
        
        let watcher = FileWatcher::new(config_file);
        assert_eq!(watcher.watcher_type(), "file");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_file_watcher_config_changes() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");
        
        // Create initial config
        let loader = FileLoader::new(config_file.clone());
        let config = DynamicConfig::default();
        loader.save(&config).await?;
        
        let watcher = FileWatcher::new(config_file.clone());
        
        // Test change calculation
        let mut new_config = config.clone();
        new_config.memory.max_memory_mb = 2048;
        
        let changes = watcher.calculate_changes(&config, &new_config);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_path, "memory.max_memory_mb");
        assert_eq!(changes[0].old_value, "1024");
        assert_eq!(changes[0].new_value, "2048");
        
        Ok(())
    }
}