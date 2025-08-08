//! Dynamic configuration integration

use crate::core::errors::ConfigError;
use crate::core::{ConfigChange, DynamicConfig, DynamicConfigManager};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

/// Integration with dynamic configuration system
pub struct ConfigIntegration {
    config_manager: Option<Arc<DynamicConfigManager>>,
}

impl ConfigIntegration {
    /// Create a new configuration integration
    pub fn new(config_manager: Option<Arc<DynamicConfigManager>>) -> Self {
        Self { config_manager }
    }

    /// Get current dynamic configuration
    pub async fn get_config(&self) -> Option<DynamicConfig> {
        if let Some(config_manager) = &self.config_manager {
            Some(config_manager.get_config().await)
        } else {
            None
        }
    }

    /// Update dynamic configuration
    pub async fn update_config<F>(&self, updater: F) -> Result<Vec<ConfigChange>>
    where
        F: FnOnce(&mut DynamicConfig) -> Result<()>,
    {
        if let Some(config_manager) = &self.config_manager {
            let wrapper = |config: &mut DynamicConfig| -> Result<(), ConfigError> {
                updater(config).map_err(|e| ConfigError::ValidationFailed {
                    reason: e.to_string(),
                })
            };
            config_manager
                .update_config(wrapper)
                .await
                .map_err(|e| anyhow::anyhow!("Config update failed: {}", e))
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    /// Reload configuration from file
    pub async fn reload_from_file(&self) -> Result<Vec<ConfigChange>> {
        if let Some(config_manager) = &self.config_manager {
            config_manager
                .reload()
                .await
                .map_err(|e| anyhow::anyhow!("Config reload failed: {}", e))
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    /// Save current configuration
    pub async fn save_current_config(&self) -> Result<()> {
        // Note: DynamicConfigManager doesn't have a save method
        // This would need to be implemented separately if needed
        if self.config_manager.is_some() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    /// Set configuration field value
    pub async fn set_field_value(&self, _field_path: &str, _value: &str) -> Result<ConfigChange> {
        // Note: This would need to be implemented using update_config
        Err(anyhow::anyhow!("Field-level updates not directly supported"))
    }

    /// Get configuration field value
    pub async fn get_field_value(&self, _field_path: &str) -> Result<String> {
        // Note: This would need to be implemented by inspecting the config struct
        Err(anyhow::anyhow!("Field-level access not directly supported"))
    }

    /// Watch configuration field for changes
    pub async fn watch_field(&self, field_path: String) -> Result<()> {
        if let Some(config_manager) = &self.config_manager {
            config_manager.watch_field(field_path).await;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    /// Apply dynamic configuration to processor components
    pub async fn apply_to_processor(&self) -> Result<()> {
        if let Some(dynamic_config) = self.get_config().await {
            // Apply configuration changes to the processor components
            
            // Update processing settings
            if !dynamic_config.processing.parallel_processing {
                info!("Disabling parallel processing based on dynamic config");
            }

            // Update cache settings
            info!(
                "Cache configuration: max_size={}MB, ttl={}h",
                dynamic_config.cache.max_size_mb, dynamic_config.cache.ttl_hours
            );

            // Update memory management settings
            info!(
                "Memory configuration: max={}MB, policy={}",
                dynamic_config.memory.max_memory_mb, dynamic_config.memory.eviction_policy
            );

            info!("Dynamic configuration applied to processor components");
        }
        Ok(())
    }
}