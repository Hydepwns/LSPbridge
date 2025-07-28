use crate::core::errors::{ConfigError, FileError};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::core::{CacheConfig, EvictionPolicy, MemoryConfig, RecoveryStrategy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicConfig {
    // Core processing settings
    pub processing: ProcessingConfig,

    // Cache settings
    pub cache: DynamicCacheConfig,

    // Memory management
    pub memory: DynamicMemoryConfig,

    // Error recovery
    pub error_recovery: DynamicErrorRecoveryConfig,

    // Git integration
    pub git: GitConfig,

    // Metrics and monitoring
    pub metrics: MetricsConfig,

    // Feature flags
    pub features: FeatureFlags,

    // Performance tuning
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub parallel_processing: bool,
    pub chunk_size: usize,
    pub max_concurrent_files: usize,
    pub file_size_limit_mb: usize,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicCacheConfig {
    pub enable_persistent_cache: bool,
    pub enable_memory_cache: bool,
    pub cache_dir: PathBuf,
    pub max_size_mb: usize,
    pub max_entries: usize,
    pub ttl_hours: u64,
    pub cleanup_interval_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMemoryConfig {
    pub max_memory_mb: usize,
    pub max_entries: usize,
    pub eviction_policy: String, // Serializable version of EvictionPolicy
    pub high_water_mark: f64,
    pub low_water_mark: f64,
    pub eviction_batch_size: usize,
    pub monitoring_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicErrorRecoveryConfig {
    pub enable_circuit_breaker: bool,
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub failure_threshold: usize,
    pub success_threshold: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub enable_git_integration: bool,
    pub scan_interval_seconds: u64,
    pub ignore_untracked: bool,
    pub track_staged_changes: bool,
    pub auto_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enable_metrics: bool,
    pub prometheus_port: u16,
    pub collection_interval_seconds: u64,
    pub retention_hours: u64,
    pub export_format: String, // "prometheus", "json", "csv"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub auto_optimization: bool,
    pub health_monitoring: bool,
    pub cache_warming: bool,
    pub advanced_diagnostics: bool,
    pub experimental_features: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub optimization_interval_minutes: u64,
    pub health_check_interval_minutes: u64,
    pub gc_threshold_mb: usize,
    pub max_cpu_usage_percent: f64,
    pub adaptive_scaling: bool,
}

impl Default for DynamicConfig {
    fn default() -> Self {
        Self {
            processing: ProcessingConfig {
                parallel_processing: true,
                chunk_size: 100,
                max_concurrent_files: 1000,
                file_size_limit_mb: 10,
                timeout_seconds: 30,
            },
            cache: DynamicCacheConfig {
                enable_persistent_cache: true,
                enable_memory_cache: true,
                cache_dir: std::env::temp_dir().join("lsp-bridge-cache"),
                max_size_mb: 100,
                max_entries: 10000,
                ttl_hours: 24,
                cleanup_interval_minutes: 60,
            },
            memory: DynamicMemoryConfig {
                max_memory_mb: 256,
                max_entries: 50000,
                eviction_policy: "Adaptive".to_string(),
                high_water_mark: 0.8,
                low_water_mark: 0.6,
                eviction_batch_size: 100,
                monitoring_interval_seconds: 30,
            },
            error_recovery: DynamicErrorRecoveryConfig {
                enable_circuit_breaker: true,
                max_retries: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2.0,
                failure_threshold: 5,
                success_threshold: 3,
                timeout_ms: 10000,
            },
            git: GitConfig {
                enable_git_integration: true,
                scan_interval_seconds: 30,
                ignore_untracked: false,
                track_staged_changes: true,
                auto_refresh: true,
            },
            metrics: MetricsConfig {
                enable_metrics: true,
                prometheus_port: 9090,
                collection_interval_seconds: 10,
                retention_hours: 72,
                export_format: "prometheus".to_string(),
            },
            features: FeatureFlags {
                auto_optimization: true,
                health_monitoring: true,
                cache_warming: true,
                advanced_diagnostics: false,
                experimental_features: false,
            },
            performance: PerformanceConfig {
                optimization_interval_minutes: 60,
                health_check_interval_minutes: 5,
                gc_threshold_mb: 512,
                max_cpu_usage_percent: 80.0,
                adaptive_scaling: true,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigChange {
    pub field_path: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: SystemTime,
}

pub type ConfigChangeNotifier = broadcast::Sender<ConfigChange>;
pub type ConfigChangeReceiver = broadcast::Receiver<ConfigChange>;

pub struct DynamicConfigManager {
    config: Arc<RwLock<DynamicConfig>>,
    config_file: PathBuf,
    last_modified: RwLock<Option<SystemTime>>,
    change_notifier: ConfigChangeNotifier,
    watchers: RwLock<Vec<String>>, // Field paths being watched
    auto_reload: bool,
    validation_rules: RwLock<HashMap<String, Box<dyn Fn(&str) -> bool + Send + Sync>>>,
}

impl DynamicConfigManager {
    pub async fn new(config_file: PathBuf) -> Result<Self, ConfigError> {
        let config = if config_file.exists() {
            Self::load_from_file(&config_file).await?
        } else {
            let default_config = DynamicConfig::default();
            Self::save_to_file(&default_config, &config_file).await?;
            default_config
        };

        let (change_notifier, _) = broadcast::channel(100);

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            config_file,
            last_modified: RwLock::new(None),
            change_notifier,
            watchers: RwLock::new(Vec::new()),
            auto_reload: true,
            validation_rules: RwLock::new(HashMap::new()),
        };

        // Initialize validation rules
        manager.setup_validation_rules().await;

        info!("Dynamic configuration manager initialized");
        Ok(manager)
    }

    async fn setup_validation_rules(&self) {
        let mut rules = self.validation_rules.write().await;

        // Memory limits
        rules.insert(
            "memory.max_memory_mb".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 64 && n <= 16384)),
        );

        // Cache settings
        rules.insert(
            "cache.max_size_mb".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 10 && n <= 10240)),
        );

        // Performance limits
        rules.insert(
            "performance.max_cpu_usage_percent".to_string(),
            Box::new(|v| v.parse::<f64>().map_or(false, |n| n >= 10.0 && n <= 100.0)),
        );

        // Port validation
        rules.insert(
            "metrics.prometheus_port".to_string(),
            Box::new(|v| v.parse::<u16>().map_or(false, |n| n >= 1024 && n <= 65535)),
        );
    }

    pub async fn get_config(&self) -> DynamicConfig {
        let config = self.config.read().await;
        config.clone()
    }

    pub async fn update_config<F>(&self, updater: F) -> Result<Vec<ConfigChange>, ConfigError>
    where
        F: FnOnce(&mut DynamicConfig) -> Result<(), ConfigError>,
    {
        let mut config = self.config.write().await;
        let old_config = config.clone();

        updater(&mut *config)?;

        // Validate the updated config
        self.validate_config(&*config).await?;

        // Calculate changes
        let changes = self.calculate_changes(&old_config, &*config);

        // Save to file
        Self::save_to_file(&*config, &self.config_file).await?;

        // Notify watchers
        for change in &changes {
            if let Err(e) = self.change_notifier.send(change.clone()) {
                warn!("Failed to notify config change: {}", e);
            }
        }

        info!("Configuration updated with {} changes", changes.len());
        Ok(changes)
    }

    pub async fn reload_from_file(&self) -> Result<Vec<ConfigChange>, ConfigError> {
        if !self.config_file.exists() {
            return Err(ConfigError::FileNotFound {
                path: self.config_file.clone(),
            });
        }

        let new_config = Self::load_from_file(&self.config_file).await?;

        let mut config = self.config.write().await;
        let old_config = config.clone();

        // Validate before applying
        self.validate_config(&new_config).await?;

        *config = new_config;

        let changes = self.calculate_changes(&old_config, &*config);

        // Notify watchers
        for change in &changes {
            if let Err(e) = self.change_notifier.send(change.clone()) {
                warn!("Failed to notify config change: {}", e);
            }
        }

        info!(
            "Configuration reloaded from file with {} changes",
            changes.len()
        );
        Ok(changes)
    }

    pub async fn save_current_config(&self) -> Result<(), ConfigError> {
        let config = self.config.read().await;
        Self::save_to_file(&*config, &self.config_file).await
    }

    pub async fn subscribe_to_changes(&self) -> ConfigChangeReceiver {
        self.change_notifier.subscribe()
    }

    pub async fn watch_field(&self, field_path: String) {
        let mut watchers = self.watchers.write().await;
        if !watchers.contains(&field_path) {
            watchers.push(field_path.clone());
            debug!("Watching field: {}", field_path);
        }
    }

    pub async fn get_field_value(&self, field_path: &str) -> Result<String, ConfigError> {
        let config = self.config.read().await;
        self.extract_field_value(&*config, field_path)
    }

    pub async fn set_field_value(
        &self,
        field_path: &str,
        value: &str,
    ) -> Result<ConfigChange, ConfigError> {
        // Validate the value first
        if let Some(validator) = self.validation_rules.read().await.get(field_path) {
            if !validator(value) {
                return Err(ConfigError::InvalidValue {
                    field: field_path.to_string(),
                    value: value.to_string(),
                    reason: "Failed validation".to_string(),
                });
            }
        }

        let _old_value = self.get_field_value(field_path).await?;

        let changes = self
            .update_config(|config| self.set_field_value_in_config(config, field_path, value))
            .await?;

        if let Some(change) = changes.into_iter().find(|c| c.field_path == field_path) {
            Ok(change)
        } else {
            Err(ConfigError::DynamicUpdateFailed {
                field: field_path.to_string(),
                reason: "Field not found in changes".to_string(),
            })
        }
    }

    pub async fn enable_auto_reload(self: Arc<Self>, enable: bool) {
        if enable && !self.auto_reload {
            // Start file watcher
            self.start_file_watcher().await;
        }
    }

    async fn start_file_watcher(self: Arc<Self>) {
        let config_file = self.config_file.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                if let Ok(metadata) = fs::metadata(&config_file).await {
                    if let Ok(modified) = metadata.modified() {
                        let should_reload = {
                            let last_modified = self.last_modified.read().await;
                            last_modified.map_or(true, |last| modified > last)
                        };

                        if should_reload {
                            if let Err(e) = self.reload_from_file().await {
                                error!("Failed to reload config: {}", e);
                            } else {
                                let mut last_modified = self.last_modified.write().await;
                                *last_modified = Some(modified);
                            }
                        }
                    }
                }
            }
        });
    }

    async fn load_from_file(path: &Path) -> Result<DynamicConfig, ConfigError> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|_e| ConfigError::FileNotFound {
                path: path.to_path_buf(),
            })?;
        let config: DynamicConfig =
            toml::from_str(&content).map_err(|e| ConfigError::ValidationFailed {
                reason: format!("Failed to parse config file: {}", e),
            })?;
        Ok(config)
    }

    async fn save_to_file(config: &DynamicConfig, path: &Path) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(config).map_err(|e| ConfigError::ValidationFailed {
                reason: format!("Failed to serialize config: {}", e),
            })?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| FileError::DirectoryError {
                    path: parent.to_path_buf(),
                    operation: "create_dir_all".to_string(),
                    source: e,
                })?;
        }

        fs::write(path, content)
            .await
            .map_err(|e| ConfigError::from(FileError::write_error(path.to_path_buf(), e)))?;
        Ok(())
    }

    async fn validate_config(&self, config: &DynamicConfig) -> Result<(), ConfigError> {
        // Basic validation
        if config.memory.max_memory_mb < 64 {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory limit too low: minimum 64MB".to_string(),
            });
        }

        if config.cache.max_size_mb > config.memory.max_memory_mb {
            return Err(ConfigError::ValidationFailed {
                reason: "Cache size cannot exceed memory limit".to_string(),
            });
        }

        if config.performance.max_cpu_usage_percent > 100.0 {
            return Err(ConfigError::ValidationFailed {
                reason: "CPU usage cannot exceed 100%".to_string(),
            });
        }

        // Field-specific validation
        let rules = self.validation_rules.read().await;

        if let Ok(value) = self.extract_field_value(config, "memory.max_memory_mb") {
            if let Some(validator) = rules.get("memory.max_memory_mb") {
                if !validator(&value) {
                    return Err(ConfigError::ValidationFailed {
                        reason: "Invalid memory limit".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    fn calculate_changes(&self, old: &DynamicConfig, new: &DynamicConfig) -> Vec<ConfigChange> {
        let mut changes = Vec::new();
        let timestamp = SystemTime::now();

        // Compare processing config
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

        // Compare memory config
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

        // Compare feature flags
        if old.features.auto_optimization != new.features.auto_optimization {
            changes.push(ConfigChange {
                field_path: "features.auto_optimization".to_string(),
                old_value: old.features.auto_optimization.to_string(),
                new_value: new.features.auto_optimization.to_string(),
                timestamp,
            });
        }

        // Compare metrics config
        if old.metrics.prometheus_port != new.metrics.prometheus_port {
            changes.push(ConfigChange {
                field_path: "metrics.prometheus_port".to_string(),
                old_value: old.metrics.prometheus_port.to_string(),
                new_value: new.metrics.prometheus_port.to_string(),
                timestamp,
            });
        }

        // Compare performance config
        if old.performance.max_cpu_usage_percent != new.performance.max_cpu_usage_percent {
            changes.push(ConfigChange {
                field_path: "performance.max_cpu_usage_percent".to_string(),
                old_value: old.performance.max_cpu_usage_percent.to_string(),
                new_value: new.performance.max_cpu_usage_percent.to_string(),
                timestamp,
            });
        }

        changes
    }

    fn extract_field_value(
        &self,
        config: &DynamicConfig,
        field_path: &str,
    ) -> Result<String, ConfigError> {
        match field_path {
            "processing.parallel_processing" => {
                Ok(config.processing.parallel_processing.to_string())
            }
            "processing.chunk_size" => Ok(config.processing.chunk_size.to_string()),
            "memory.max_memory_mb" => Ok(config.memory.max_memory_mb.to_string()),
            "memory.eviction_policy" => Ok(config.memory.eviction_policy.clone()),
            "features.auto_optimization" => Ok(config.features.auto_optimization.to_string()),
            "metrics.enable_metrics" => Ok(config.metrics.enable_metrics.to_string()),
            "metrics.prometheus_port" => Ok(config.metrics.prometheus_port.to_string()),
            "performance.max_cpu_usage_percent" => {
                Ok(config.performance.max_cpu_usage_percent.to_string())
            }
            "git.enable_git_integration" => Ok(config.git.enable_git_integration.to_string()),
            _ => Err(ConfigError::InvalidValue {
                field: field_path.to_string(),
                value: "unknown".to_string(),
                reason: "Unknown field path".to_string(),
            }),
        }
    }

    fn parse_value<T: std::str::FromStr>(
        field_path: &str,
        value: &str,
        type_name: &str,
    ) -> Result<T, ConfigError> {
        value.parse().map_err(|_| ConfigError::InvalidValue {
            field: field_path.to_string(),
            value: value.to_string(),
            reason: format!("Invalid {} value", type_name),
        })
    }

    fn set_field_value_in_config(
        &self,
        config: &mut DynamicConfig,
        field_path: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field_path {
            "processing.parallel_processing" => {
                config.processing.parallel_processing =
                    Self::parse_value(field_path, value, "boolean")?;
            }
            "processing.chunk_size" => {
                config.processing.chunk_size = Self::parse_value(field_path, value, "number")?;
            }
            "memory.max_memory_mb" => {
                config.memory.max_memory_mb = Self::parse_value(field_path, value, "number")?;
            }
            "memory.eviction_policy" => {
                config.memory.eviction_policy = value.to_string();
            }
            "features.auto_optimization" => {
                config.features.auto_optimization =
                    Self::parse_value(field_path, value, "boolean")?;
            }
            "metrics.enable_metrics" => {
                config.metrics.enable_metrics = Self::parse_value(field_path, value, "boolean")?;
            }
            "metrics.prometheus_port" => {
                config.metrics.prometheus_port = Self::parse_value(field_path, value, "number")?;
            }
            "performance.max_cpu_usage_percent" => {
                config.performance.max_cpu_usage_percent =
                    Self::parse_value(field_path, value, "number")?;
            }
            "git.enable_git_integration" => {
                config.git.enable_git_integration =
                    Self::parse_value(field_path, value, "boolean")?;
            }
            _ => {
                return Err(ConfigError::InvalidValue {
                    field: field_path.to_string(),
                    value: "unknown".to_string(),
                    reason: "Unknown field path".to_string(),
                })
            }
        }
        Ok(())
    }
}

// Conversion utilities for integrating with existing config types

impl DynamicCacheConfig {
    pub fn to_cache_config(&self) -> CacheConfig {
        CacheConfig {
            cache_dir: self.cache_dir.clone(),
            max_size_mb: self.max_size_mb,
            max_entries: self.max_entries,
            ttl: Duration::from_secs(self.ttl_hours * 3600),
            enable_compression: true, // Default
        }
    }
}

impl DynamicMemoryConfig {
    pub fn to_memory_config(&self) -> MemoryConfig {
        let eviction_policy = match self.eviction_policy.as_str() {
            "LRU" => EvictionPolicy::LRU,
            "LFU" => EvictionPolicy::LFU,
            "SizeWeighted" => EvictionPolicy::SizeWeighted,
            "AgeWeighted" => EvictionPolicy::AgeWeighted,
            "Adaptive" | _ => EvictionPolicy::Adaptive,
        };

        MemoryConfig {
            max_memory_mb: self.max_memory_mb,
            max_entries: self.max_entries,
            eviction_policy,
            high_water_mark: self.high_water_mark,
            low_water_mark: self.low_water_mark,
            eviction_batch_size: self.eviction_batch_size,
            monitoring_interval: Duration::from_secs(self.monitoring_interval_seconds),
        }
    }
}

impl DynamicErrorRecoveryConfig {
    pub fn to_recovery_strategy(&self) -> RecoveryStrategy {
        RecoveryStrategy {
            max_retries: self.max_retries,
            initial_delay: Duration::from_millis(self.initial_delay_ms),
            max_delay: Duration::from_millis(self.max_delay_ms),
            backoff_multiplier: self.backoff_multiplier,
            circuit_breaker_threshold: self.failure_threshold,
            circuit_breaker_timeout: Duration::from_millis(self.timeout_ms),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file.clone()).await?;
        assert!(config_file.exists());

        let config = manager.get_config().await;
        assert_eq!(config.processing.parallel_processing, true);

        Ok(())
    }

    #[tokio::test]
    async fn test_config_updates() -> Result<()> {
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
    async fn test_field_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file).await?;

        // Test get field value
        let value = manager.get_field_value("memory.max_memory_mb").await?;
        assert_eq!(value, "256");

        // Test set field value
        let change = manager
            .set_field_value("memory.max_memory_mb", "512")
            .await?;
        assert_eq!(change.field_path, "memory.max_memory_mb");
        assert_eq!(change.old_value, "256");
        assert_eq!(change.new_value, "512");

        Ok(())
    }

    #[tokio::test]
    async fn test_validation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let manager = DynamicConfigManager::new(config_file).await?;

        // Test invalid memory limit
        let result = manager.set_field_value("memory.max_memory_mb", "32").await;
        assert!(result.is_err());

        // Test valid memory limit
        let result = manager.set_field_value("memory.max_memory_mb", "512").await;
        assert!(result.is_ok());

        Ok(())
    }
}
