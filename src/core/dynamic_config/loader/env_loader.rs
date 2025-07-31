//! Environment variable configuration loader

use super::{ConfigLoader, DynamicConfig};
use crate::core::errors::ConfigError;
use async_trait::async_trait;
use std::env;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Environment variable configuration loader
pub struct EnvLoader {
    prefix: String,
}

impl EnvLoader {
    /// Create a new environment loader with the given prefix
    pub fn new(prefix: String) -> Self {
        Self { prefix }
    }

    /// Create a default environment loader with "LSPBRIDGE_" prefix
    pub fn default() -> Self {
        Self::new("LSPBRIDGE_".to_string())
    }

    /// Apply environment variables to existing config
    pub fn apply_env_overrides(&self, mut config: DynamicConfig) -> DynamicConfig {
        // Processing overrides
        if let Ok(val) = env::var(format!("{}PARALLEL_PROCESSING", self.prefix)) {
            if let Ok(enabled) = val.parse::<bool>() {
                config.processing.parallel_processing = enabled;
                debug!("Applied env override: parallel_processing = {}", enabled);
            }
        }

        if let Ok(val) = env::var(format!("{}CHUNK_SIZE", self.prefix)) {
            if let Ok(size) = val.parse::<usize>() {
                config.processing.chunk_size = size;
                debug!("Applied env override: chunk_size = {}", size);
            }
        }

        if let Ok(val) = env::var(format!("{}MAX_CONCURRENT_FILES", self.prefix)) {
            if let Ok(max_files) = val.parse::<usize>() {
                config.processing.max_concurrent_files = max_files;
                debug!("Applied env override: max_concurrent_files = {}", max_files);
            }
        }

        // Memory overrides
        if let Ok(val) = env::var(format!("{}MAX_MEMORY_MB", self.prefix)) {
            if let Ok(memory) = val.parse::<usize>() {
                config.memory.max_memory_mb = memory;
                debug!("Applied env override: max_memory_mb = {}", memory);
            }
        }

        if let Ok(val) = env::var(format!("{}EVICTION_POLICY", self.prefix)) {
            config.memory.eviction_policy = val.clone();
            debug!("Applied env override: eviction_policy = {}", val);
        }

        // Cache overrides
        if let Ok(val) = env::var(format!("{}CACHE_DIR", self.prefix)) {
            config.cache.cache_dir = PathBuf::from(val.clone());
            debug!("Applied env override: cache_dir = {}", val);
        }

        if let Ok(val) = env::var(format!("{}CACHE_MAX_SIZE_MB", self.prefix)) {
            if let Ok(size) = val.parse::<usize>() {
                config.cache.max_size_mb = size;
                debug!("Applied env override: cache.max_size_mb = {}", size);
            }
        }

        // Metrics overrides
        if let Ok(val) = env::var(format!("{}METRICS_ENABLED", self.prefix)) {
            if let Ok(enabled) = val.parse::<bool>() {
                config.metrics.enable_metrics = enabled;
                debug!("Applied env override: metrics.enable_metrics = {}", enabled);
            }
        }

        if let Ok(val) = env::var(format!("{}PROMETHEUS_PORT", self.prefix)) {
            if let Ok(port) = val.parse::<u16>() {
                config.metrics.prometheus_port = port;
                debug!("Applied env override: metrics.prometheus_port = {}", port);
            }
        }

        // Performance overrides
        if let Ok(val) = env::var(format!("{}MAX_CPU_USAGE_PERCENT", self.prefix)) {
            if let Ok(cpu) = val.parse::<f64>() {
                config.performance.max_cpu_usage_percent = cpu;
                debug!("Applied env override: performance.max_cpu_usage_percent = {}", cpu);
            }
        }

        // Feature flags
        if let Ok(val) = env::var(format!("{}ENABLE_SMART_CACHING", self.prefix)) {
            if let Ok(enabled) = val.parse::<bool>() {
                config.features.enable_smart_caching = enabled;
                debug!("Applied env override: features.enable_smart_caching = {}", enabled);
            }
        }

        if let Ok(val) = env::var(format!("{}ENABLE_EXPERIMENTAL_FEATURES", self.prefix)) {
            if let Ok(enabled) = val.parse::<bool>() {
                config.features.enable_experimental_features = enabled;
                debug!("Applied env override: features.enable_experimental_features = {}", enabled);
            }
        }

        config
    }
}

#[async_trait]
impl ConfigLoader for EnvLoader {
    async fn load(&self) -> Result<DynamicConfig, ConfigError> {
        debug!("Loading config from environment variables with prefix: {}", self.prefix);
        
        let mut config = DynamicConfig::default();
        config = self.apply_env_overrides(config);
        
        info!("Successfully loaded config from environment variables");
        Ok(config)
    }

    async fn save(&self, _config: &DynamicConfig) -> Result<(), ConfigError> {
        warn!("Cannot save configuration to environment variables");
        Err(ConfigError::ValidationFailed {
            reason: "Environment variables are read-only".to_string(),
        })
    }

    async fn exists(&self) -> bool {
        // Environment variables always "exist" in the sense that we can read them
        // Check if any of our expected variables are set
        env::var(format!("{}MAX_MEMORY_MB", self.prefix)).is_ok()
            || env::var(format!("{}PARALLEL_PROCESSING", self.prefix)).is_ok()
            || env::var(format!("{}CACHE_DIR", self.prefix)).is_ok()
    }

    fn loader_type(&self) -> &'static str {
        "environment"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_env_loader_default_config() -> anyhow::Result<()> {
        let loader = EnvLoader::default();
        
        // Should always be able to load (returns default + env overrides)
        let config = loader.load().await?;
        assert_eq!(config.processing.parallel_processing, true); // default value
        
        Ok(())
    }

    #[tokio::test] 
    async fn test_env_loader_with_overrides() -> anyhow::Result<()> {
        let loader = EnvLoader::default();
        
        // Set some environment variables
        env::set_var("LSPBRIDGE_MAX_MEMORY_MB", "2048");
        env::set_var("LSPBRIDGE_PARALLEL_PROCESSING", "false");
        env::set_var("LSPBRIDGE_CHUNK_SIZE", "500");
        
        let config = loader.load().await?;
        
        // Check that overrides were applied
        assert_eq!(config.memory.max_memory_mb, 2048);
        assert_eq!(config.processing.parallel_processing, false);
        assert_eq!(config.processing.chunk_size, 500);
        
        // Clean up
        env::remove_var("LSPBRIDGE_MAX_MEMORY_MB");
        env::remove_var("LSPBRIDGE_PARALLEL_PROCESSING");
        env::remove_var("LSPBRIDGE_CHUNK_SIZE");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_env_loader_cannot_save() -> anyhow::Result<()> {
        let loader = EnvLoader::default();
        let config = DynamicConfig::default();
        
        // Should not be able to save
        assert!(loader.save(&config).await.is_err());
        
        Ok(())
    }
}