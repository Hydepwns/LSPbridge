//! Configuration validation functionality

pub mod rules;
pub mod schema;

pub use rules::ValidationRules;
pub use schema::ConfigValidator;

use super::types::DynamicConfig;
use crate::core::errors::ConfigError;

/// Main configuration validator
pub struct ConfigValidationEngine {
    rules: ValidationRules,
    validator: ConfigValidator,
}

impl ConfigValidationEngine {
    /// Create a new validation engine
    pub fn new() -> Self {
        Self {
            rules: ValidationRules::new(),
            validator: ConfigValidator::new(),
        }
    }

    /// Validate a configuration
    pub async fn validate(&self, config: &DynamicConfig) -> Result<(), ConfigError> {
        // Schema validation first
        self.validator.validate_schema(config)?;
        
        // Then business rules validation
        self.rules.validate_all(config).await?;
        
        Ok(())
    }

    /// Add a custom validation rule
    pub async fn add_rule<F>(&self, field_path: String, validator: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.rules.add_rule(field_path, Box::new(validator)).await;
    }

    /// Validate a specific field value
    pub async fn validate_field(&self, config: &DynamicConfig, field_path: &str) -> Result<(), ConfigError> {
        let value = self.extract_field_value(config, field_path)?;
        self.rules.validate_field(field_path, &value).await
    }

    /// Extract field value from config for validation
    fn extract_field_value(&self, config: &DynamicConfig, field_path: &str) -> Result<String, ConfigError> {
        match field_path {
            "processing.parallel_processing" => Ok(config.processing.parallel_processing.to_string()),
            "processing.chunk_size" => Ok(config.processing.chunk_size.to_string()),
            "processing.max_concurrent_files" => Ok(config.processing.max_concurrent_files.to_string()),
            "processing.file_size_limit_mb" => Ok(config.processing.file_size_limit_mb.to_string()),
            "processing.timeout_seconds" => Ok(config.processing.timeout_seconds.to_string()),
            
            "memory.max_memory_mb" => Ok(config.memory.max_memory_mb.to_string()),
            "memory.max_entries" => Ok(config.memory.max_entries.to_string()),
            "memory.eviction_policy" => Ok(config.memory.eviction_policy.clone()),
            "memory.high_water_mark" => Ok(config.memory.high_water_mark.to_string()),
            "memory.low_water_mark" => Ok(config.memory.low_water_mark.to_string()),
            
            "cache.enable_persistent_cache" => Ok(config.cache.enable_persistent_cache.to_string()),
            "cache.enable_memory_cache" => Ok(config.cache.enable_memory_cache.to_string()),
            "cache.max_size_mb" => Ok(config.cache.max_size_mb.to_string()),
            "cache.max_entries" => Ok(config.cache.max_entries.to_string()),
            "cache.ttl_hours" => Ok(config.cache.ttl_hours.to_string()),
            
            "metrics.enable_metrics" => Ok(config.metrics.enable_metrics.to_string()),
            "metrics.prometheus_port" => Ok(config.metrics.prometheus_port.to_string()),
            "metrics.collection_interval_seconds" => Ok(config.metrics.collection_interval_seconds.to_string()),
            "metrics.retention_hours" => Ok(config.metrics.retention_hours.to_string()),
            "metrics.export_format" => Ok(config.metrics.export_format.clone()),
            
            "performance.max_cpu_usage_percent" => Ok(config.performance.max_cpu_usage_percent.to_string()),
            "performance.io_priority" => Ok(config.performance.io_priority.clone()),
            "performance.enable_parallel_io" => Ok(config.performance.enable_parallel_io.to_string()),
            
            _ => Err(ConfigError::InvalidValue {
                field: field_path.to_string(),
                value: "unknown".to_string(),
                reason: "Unknown field path".to_string(),
            }),
        }
    }
}

impl Default for ConfigValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}