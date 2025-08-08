//! Dynamic configuration management system
//!
//! This module provides a comprehensive dynamic configuration system that supports:
//! - File-based and environment variable configuration loading
//! - Real-time configuration validation with custom rules
//! - Automatic file watching and hot-reloading
//! - Configuration change notifications
//! - Type-safe conversion to static configuration types
//!
//! # Example Usage
//!
//! ```rust
//! use lsp_bridge::core::dynamic_config::{DynamicConfigManager, DynamicConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config_file = PathBuf::from("config.toml");
//!     let manager = DynamicConfigManager::new(config_file).await?;
//!
//!     // Get current configuration
//!     let config = manager.get_config().await;
//!     println!("Max memory: {}MB", config.memory.max_memory_mb);
//!
//!     // Update configuration
//!     let changes = manager.update_config(|config| {
//!         config.memory.max_memory_mb = 2048;
//!         config.processing.parallel_processing = false;
//!         Ok(())
//!     }).await?;
//!
//!     println!("Applied {} configuration changes", changes.len());
//!
//!     // Start automatic file watching
//!     manager.start_auto_reload().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod loader;
pub mod types;
pub mod validation;
pub mod watchers;

// Re-export main types for convenience
pub use types::{
    ConfigChange, DynamicCacheConfig, DynamicConfig, DynamicErrorRecoveryConfig,
    DynamicMemoryConfig, FeatureFlags, GitConfig, MetricsConfig, PerformanceConfig,
    ProcessingConfig,
};

use crate::core::errors::ConfigError;
use loader::{CombinedLoader, EnvLoader, FileLoader};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use validation::ConfigValidationEngine;
use watchers::{ConfigChangeNotifier, FileWatcher};

/// Main dynamic configuration manager
///
/// This is the primary interface for managing dynamic configuration.
/// It combines loading, validation, watching, and change notification
/// functionality into a single, easy-to-use API.
pub struct DynamicConfigManager {
    config: Arc<RwLock<DynamicConfig>>,
    loader: CombinedLoader,
    validator: ConfigValidationEngine,
    watcher: Option<FileWatcher>,
    change_notifier: ConfigChangeNotifier,
    watchers: RwLock<Vec<String>>, // Field paths being watched
    auto_reload: bool,
}

impl DynamicConfigManager {
    /// Create a new dynamic configuration manager
    ///
    /// This will attempt to load configuration from the specified file.
    /// If the file doesn't exist, a default configuration will be created and saved.
    pub async fn new(config_file: PathBuf) -> Result<Self, ConfigError> {
        let loader = CombinedLoader::new()
            .add_loader(Box::new(FileLoader::new(config_file.clone())))
            .add_loader(Box::new(EnvLoader::default()));

        let config = loader.load().await?;
        let validator = ConfigValidationEngine::new();
        
        // Validate the loaded configuration
        validator.validate(&config).await?;
        
        // If the config file doesn't exist, save the default config
        if !config_file.exists() {
            loader.save(&config).await?;
        }

        let (change_notifier, _) = ConfigChangeNotifier::new(100);
        let watcher = Some(FileWatcher::new(config_file));

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            loader,
            validator,
            watcher,
            change_notifier,
            watchers: RwLock::new(Vec::new()),
            auto_reload: true,
        };

        info!("Dynamic configuration manager initialized");
        Ok(manager)
    }

    /// Create a configuration manager with custom loaders
    pub async fn with_loaders(loaders: Vec<Box<dyn loader::ConfigLoader + Send + Sync>>) -> Result<Self, ConfigError> {
        let mut combined_loader = CombinedLoader::new();
        for loader in loaders {
            combined_loader = combined_loader.add_loader(loader);
        }

        let config = combined_loader.load().await?;
        let validator = ConfigValidationEngine::new();
        
        // Validate the loaded configuration
        validator.validate(&config).await?;

        let (change_notifier, _) = ConfigChangeNotifier::new(100);

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            loader: combined_loader,
            validator,
            watcher: None,
            change_notifier,
            watchers: RwLock::new(Vec::new()),
            auto_reload: false,
        };

        info!("Dynamic configuration manager initialized with custom loaders");
        Ok(manager)
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> DynamicConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update the configuration using a closure
    ///
    /// The closure receives a mutable reference to the configuration
    /// and can make any desired changes. The configuration will be
    /// validated after the update and saved to storage.
    pub async fn update_config<F>(&self, updater: F) -> Result<Vec<ConfigChange>, ConfigError>
    where
        F: FnOnce(&mut DynamicConfig) -> Result<(), ConfigError>,
    {
        let mut config = self.config.write().await;
        let old_config = config.clone();

        // Clone config for updating
        let mut new_config = config.clone();
        
        // Apply the update to the clone
        updater(&mut new_config)?;

        // Validate the updated config
        self.validator.validate(&new_config).await?;

        // Only apply changes if validation passed
        *config = new_config;

        // Calculate changes
        let changes = self.calculate_changes(&old_config, &config);

        // Save to storage
        self.loader.save(&config).await?;

        // Notify watchers
        for change in &changes {
            if let Err(e) = self.change_notifier.notify(change.clone()) {
                warn!("Failed to notify config change: {}", e);
            }
        }

        info!("Configuration updated with {} changes", changes.len());
        Ok(changes)
    }

    /// Subscribe to configuration change notifications
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChange> {
        self.change_notifier.subscribe()
    }

    /// Add a field to watch for changes
    pub async fn watch_field(&self, field_path: String) {
        let mut watchers = self.watchers.write().await;
        if !watchers.contains(&field_path) {
            watchers.push(field_path);
            debug!("Added field watcher: {}", watchers.last().unwrap());
        }
    }

    /// Remove a field from the watch list
    pub async fn unwatch_field(&self, field_path: &str) {
        let mut watchers = self.watchers.write().await;
        watchers.retain(|w| w != field_path);
        debug!("Removed field watcher: {}", field_path);
    }

    /// Start automatic configuration file reloading
    pub async fn start_auto_reload(&self) -> Result<(), ConfigError> {
        if !self.auto_reload {
            return Err(ConfigError::ValidationFailed {
                reason: "Auto-reload not supported for this manager".to_string(),
            });
        }

        if let Some(watcher) = &self.watcher {
            use watchers::ConfigWatcher;
            watcher.start_watching().await?;
            info!("Started automatic configuration reloading");
        }

        Ok(())
    }

    /// Stop automatic configuration file reloading
    pub async fn stop_auto_reload(&self) -> Result<(), ConfigError> {
        if let Some(watcher) = &self.watcher {
            use watchers::ConfigWatcher;
            watcher.stop_watching().await?;
            info!("Stopped automatic configuration reloading");
        }

        Ok(())
    }

    /// Manually reload configuration from storage
    pub async fn reload(&self) -> Result<Vec<ConfigChange>, ConfigError> {
        let new_config = self.loader.load().await?;
        
        // Validate the new configuration
        self.validator.validate(&new_config).await?;

        let old_config = {
            let config = self.config.read().await;
            config.clone()
        };

        // Calculate changes
        let changes = self.calculate_changes(&old_config, &new_config);

        // Update the stored configuration
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }

        // Notify watchers
        for change in &changes {
            if let Err(e) = self.change_notifier.notify(change.clone()) {
                warn!("Failed to notify config change: {}", e);
            }
        }

        info!("Configuration reloaded with {} changes", changes.len());
        Ok(changes)
    }

    /// Add a custom validation rule
    pub async fn add_validation_rule<F>(&self, field_path: String, validator: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.validator.add_rule(field_path, validator).await;
    }

    /// Validate the current configuration
    pub async fn validate_current(&self) -> Result<(), ConfigError> {
        let config = self.config.read().await;
        self.validator.validate(&config).await
    }

    /// Calculate differences between two configurations
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

        if old.processing.max_concurrent_files != new.processing.max_concurrent_files {
            changes.push(ConfigChange {
                field_path: "processing.max_concurrent_files".to_string(),
                old_value: old.processing.max_concurrent_files.to_string(),
                new_value: new.processing.max_concurrent_files.to_string(),
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

        if old.cache.ttl_hours != new.cache.ttl_hours {
            changes.push(ConfigChange {
                field_path: "cache.ttl_hours".to_string(),
                old_value: old.cache.ttl_hours.to_string(),
                new_value: new.cache.ttl_hours.to_string(),
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

        if old.metrics.enable_metrics != new.metrics.enable_metrics {
            changes.push(ConfigChange {
                field_path: "metrics.enable_metrics".to_string(),
                old_value: old.metrics.enable_metrics.to_string(),
                new_value: new.metrics.enable_metrics.to_string(),
                timestamp,
            });
        }

        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file.clone()).await?;
        assert!(config_file.exists());

        let config = manager.get_config().await;
        assert_eq!(config.processing.parallel_processing, true);

        Ok(())
    }

    #[tokio::test]
    async fn test_config_updates() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file).await?;

        let changes = manager
            .update_config(|config| {
                config.processing.chunk_size = 200;
                config.memory.max_memory_mb = 512;
                Ok(())
            })
            .await?;

        assert_eq!(changes.len(), 2);

        let config = manager.get_config().await;
        assert_eq!(config.processing.chunk_size, 200);
        assert_eq!(config.memory.max_memory_mb, 512);

        Ok(())
    }

    #[tokio::test]
    async fn test_change_notifications() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file).await?;
        let mut receiver = manager.subscribe_to_changes();

        // Update config in background
        let manager_clone = Arc::new(manager);
        let handle = {
            let manager = Arc::clone(&manager_clone);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let _ = manager
                    .update_config(|config| {
                        config.memory.max_memory_mb = 2048;
                        Ok(())
                    })
                    .await;
            })
        };

        // Wait for change notification
        let change = tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            receiver.recv(),
        )
        .await??;

        assert_eq!(change.field_path, "memory.max_memory_mb");
        assert_eq!(change.new_value, "2048");

        handle.await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_validation_errors() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file).await?;

        // Try to set invalid configuration
        let result = manager
            .update_config(|config| {
                config.memory.max_memory_mb = 32; // Too low
                Ok(())
            })
            .await;

        assert!(result.is_err());

        // Configuration should remain unchanged
        let config = manager.get_config().await;
        assert_eq!(config.memory.max_memory_mb, 1024); // Default value

        Ok(())
    }
}