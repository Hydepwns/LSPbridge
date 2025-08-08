//! Schema-based configuration validation

use super::super::types::DynamicConfig;
use crate::core::errors::ConfigError;
use tracing::debug;

/// Schema-based configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Create a new schema validator
    pub fn new() -> Self {
        Self
    }

    /// Validate configuration against schema constraints
    pub fn validate_schema(&self, config: &DynamicConfig) -> Result<(), ConfigError> {
        debug!("Running schema validation");

        // Validate processing config
        self.validate_processing_config(&config.processing)?;
        
        // Validate memory config
        self.validate_memory_config(&config.memory)?;
        
        // Validate cache config
        self.validate_cache_config(&config.cache)?;
        
        // Validate error recovery config
        self.validate_error_recovery_config(&config.error_recovery)?;
        
        // Validate git config
        self.validate_git_config(&config.git)?;
        
        // Validate metrics config
        self.validate_metrics_config(&config.metrics)?;
        
        // Validate feature flags
        self.validate_feature_flags(&config.features)?;
        
        // Validate performance config
        self.validate_performance_config(&config.performance)?;

        debug!("Schema validation passed");
        Ok(())
    }

    fn validate_processing_config(&self, config: &crate::core::dynamic_config::types::ProcessingConfig) -> Result<(), ConfigError> {
        if config.chunk_size == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Processing chunk_size must be greater than 0".to_string(),
            });
        }

        if config.max_concurrent_files == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Processing max_concurrent_files must be greater than 0".to_string(),
            });
        }

        if config.file_size_limit_mb == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Processing file_size_limit_mb must be greater than 0".to_string(),
            });
        }

        if config.timeout_seconds == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Processing timeout_seconds must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    fn validate_memory_config(&self, config: &crate::core::dynamic_config::types::DynamicMemoryConfig) -> Result<(), ConfigError> {
        if config.max_memory_mb == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory max_memory_mb must be greater than 0".to_string(),
            });
        }

        if config.max_entries == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory max_entries must be greater than 0".to_string(),
            });
        }

        if config.high_water_mark <= 0.0 || config.high_water_mark > 1.0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory high_water_mark must be between 0.0 and 1.0".to_string(),
            });
        }

        if config.low_water_mark <= 0.0 || config.low_water_mark > 1.0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory low_water_mark must be between 0.0 and 1.0".to_string(),
            });
        }

        if config.low_water_mark >= config.high_water_mark {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory low_water_mark must be less than high_water_mark".to_string(),
            });
        }

        if config.eviction_batch_size == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Memory eviction_batch_size must be greater than 0".to_string(),
            });
        }

        // Validate eviction policy
        if !matches!(config.eviction_policy.as_str(), "LRU" | "LFU" | "SizeWeighted" | "AgeWeighted" | "Adaptive") {
            return Err(ConfigError::ValidationFailed {
                reason: format!("Invalid eviction policy: {}", config.eviction_policy),
            });
        }

        Ok(())
    }

    fn validate_cache_config(&self, config: &crate::core::dynamic_config::types::DynamicCacheConfig) -> Result<(), ConfigError> {
        if config.max_size_mb == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Cache max_size_mb must be greater than 0".to_string(),
            });
        }

        if config.max_entries == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Cache max_entries must be greater than 0".to_string(),
            });
        }

        if config.ttl_hours == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Cache ttl_hours must be greater than 0".to_string(),
            });
        }

        // Validate cache directory path
        if config.cache_dir.as_os_str().is_empty() {
            return Err(ConfigError::ValidationFailed {
                reason: "Cache directory path cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    fn validate_error_recovery_config(&self, config: &crate::core::dynamic_config::types::DynamicErrorRecoveryConfig) -> Result<(), ConfigError> {
        if config.max_retries == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Error recovery max_retries must be greater than 0".to_string(),
            });
        }

        if config.initial_delay_ms == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Error recovery initial_delay_ms must be greater than 0".to_string(),
            });
        }

        if config.max_delay_ms < config.initial_delay_ms {
            return Err(ConfigError::ValidationFailed {
                reason: "Error recovery max_delay_ms must be >= initial_delay_ms".to_string(),
            });
        }

        if config.backoff_multiplier <= 1.0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Error recovery backoff_multiplier must be > 1.0".to_string(),
            });
        }

        if config.failure_threshold == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Error recovery failure_threshold must be greater than 0".to_string(),
            });
        }

        if config.success_threshold == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Error recovery success_threshold must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    fn validate_git_config(&self, _config: &crate::core::dynamic_config::types::GitConfig) -> Result<(), ConfigError> {
        // Git config is mostly boolean flags, no complex validation needed
        Ok(())
    }

    fn validate_metrics_config(&self, config: &crate::core::dynamic_config::types::MetricsConfig) -> Result<(), ConfigError> {
        if config.prometheus_port < 1024 {
            return Err(ConfigError::ValidationFailed {
                reason: "Metrics prometheus_port must be >= 1024".to_string(),
            });
        }

        if config.collection_interval_seconds == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Metrics collection_interval_seconds must be greater than 0".to_string(),
            });
        }

        if config.retention_hours == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Metrics retention_hours must be greater than 0".to_string(),
            });
        }

        // Validate export format
        if !matches!(config.export_format.as_str(), "prometheus" | "json" | "csv") {
            return Err(ConfigError::ValidationFailed {
                reason: format!("Invalid metrics export format: {}", config.export_format),
            });
        }

        Ok(())
    }

    fn validate_feature_flags(&self, _config: &crate::core::dynamic_config::types::FeatureFlags) -> Result<(), ConfigError> {
        // Feature flags are all boolean, no validation needed
        Ok(())
    }

    fn validate_performance_config(&self, config: &crate::core::dynamic_config::types::PerformanceConfig) -> Result<(), ConfigError> {
        if config.max_cpu_usage_percent <= 0.0 || config.max_cpu_usage_percent > 100.0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Performance max_cpu_usage_percent must be between 0.0 and 100.0".to_string(),
            });
        }

        // Validate IO priority
        if !matches!(config.io_priority.as_str(), "low" | "normal" | "high") {
            return Err(ConfigError::ValidationFailed {
                reason: format!("Invalid IO priority: {}", config.io_priority),
            });
        }

        Ok(())
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validation_success() {
        let validator = ConfigValidator::new();
        let config = DynamicConfig::default();
        
        assert!(validator.validate_schema(&config).is_ok());
    }

    #[test]
    fn test_schema_validation_processing_errors() {
        let validator = ConfigValidator::new();
        let mut config = DynamicConfig::default();
        
        // Test chunk_size = 0
        config.processing.chunk_size = 0;
        assert!(validator.validate_schema(&config).is_err());
        
        // Reset and test timeout = 0  
        config = DynamicConfig::default();
        config.processing.timeout_seconds = 0;
        assert!(validator.validate_schema(&config).is_err());
    }

    #[test]
    fn test_schema_validation_memory_errors() {
        let validator = ConfigValidator::new();
        let mut config = DynamicConfig::default();
        
        // Test invalid water marks
        config.memory.high_water_mark = 0.5;
        config.memory.low_water_mark = 0.8; // higher than high
        assert!(validator.validate_schema(&config).is_err());
        
        // Test invalid eviction policy
        config = DynamicConfig::default();
        config.memory.eviction_policy = "InvalidPolicy".to_string();
        assert!(validator.validate_schema(&config).is_err());
    }

    #[test]
    fn test_schema_validation_performance_errors() {
        let validator = ConfigValidator::new();
        let mut config = DynamicConfig::default();
        
        // Test invalid CPU usage
        config.performance.max_cpu_usage_percent = 150.0;
        assert!(validator.validate_schema(&config).is_err());
        
        // Test invalid IO priority
        config = DynamicConfig::default();
        config.performance.io_priority = "invalid".to_string();
        assert!(validator.validate_schema(&config).is_err());
    }
}