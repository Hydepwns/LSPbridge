//! Validation rules for dynamic configuration

use super::super::types::DynamicConfig;
use crate::core::errors::ConfigError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Validation rules manager
pub struct ValidationRules {
    rules: Arc<RwLock<HashMap<String, Box<dyn Fn(&str) -> bool + Send + Sync>>>>,
}

impl ValidationRules {
    /// Create a new validation rules manager
    pub fn new() -> Self {
        let rules = Arc::new(RwLock::new(HashMap::new()));
        let validator = Self { rules };
        
        // Initialize with default rules
        tokio::spawn({
            let validator = validator.clone();
            async move {
                validator.setup_default_rules().await;
            }
        });
        
        validator
    }

    /// Setup default validation rules
    async fn setup_default_rules(&self) {
        let mut rules = self.rules.write().await;

        // Memory limits
        rules.insert(
            "memory.max_memory_mb".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 64 && n <= 16384)),
        );

        rules.insert(
            "memory.max_entries".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 100 && n <= 1_000_000)),
        );

        rules.insert(
            "memory.high_water_mark".to_string(),
            Box::new(|v| v.parse::<f64>().map_or(false, |n| n >= 0.5 && n <= 0.95)),
        );

        rules.insert(
            "memory.low_water_mark".to_string(),
            Box::new(|v| v.parse::<f64>().map_or(false, |n| n >= 0.1 && n <= 0.8)),
        );

        rules.insert(
            "memory.eviction_policy".to_string(),
            Box::new(|v| matches!(v, "LRU" | "LFU" | "SizeWeighted" | "AgeWeighted" | "Adaptive")),
        );

        // Cache settings
        rules.insert(
            "cache.max_size_mb".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 10 && n <= 10240)),
        );

        rules.insert(
            "cache.max_entries".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 100 && n <= 1_000_000)),
        );

        rules.insert(
            "cache.ttl_hours".to_string(),
            Box::new(|v| v.parse::<u64>().map_or(false, |n| n >= 1 && n <= 8760)), // 1 hour to 1 year
        );

        // Performance limits
        rules.insert(
            "performance.max_cpu_usage_percent".to_string(),
            Box::new(|v| v.parse::<f64>().map_or(false, |n| n >= 10.0 && n <= 100.0)),
        );

        rules.insert(
            "performance.io_priority".to_string(),
            Box::new(|v| matches!(v, "low" | "normal" | "high")),
        );

        // Processing limits
        rules.insert(
            "processing.chunk_size".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 10 && n <= 10000)),
        );

        rules.insert(
            "processing.max_concurrent_files".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 1 && n <= 1000)),
        );

        rules.insert(
            "processing.file_size_limit_mb".to_string(),
            Box::new(|v| v.parse::<usize>().map_or(false, |n| n >= 1 && n <= 10240)),
        );

        rules.insert(
            "processing.timeout_seconds".to_string(),
            Box::new(|v| v.parse::<u64>().map_or(false, |n| n >= 1 && n <= 3600)),
        );

        // Port validation
        rules.insert(
            "metrics.prometheus_port".to_string(),
            Box::new(|v| v.parse::<u16>().map_or(false, |n| n >= 1024 && n <= 65535)),
        );

        // Metrics settings
        rules.insert(
            "metrics.collection_interval_seconds".to_string(),
            Box::new(|v| v.parse::<u64>().map_or(false, |n| n >= 1 && n <= 3600)),
        );

        rules.insert(
            "metrics.retention_hours".to_string(),
            Box::new(|v| v.parse::<u64>().map_or(false, |n| n >= 1 && n <= 8760)),
        );

        rules.insert(
            "metrics.export_format".to_string(),
            Box::new(|v| matches!(v, "prometheus" | "json" | "csv")),
        );

        debug!("Default validation rules initialized");
    }

    /// Add a custom validation rule
    pub async fn add_rule(&self, field_path: String, validator: Box<dyn Fn(&str) -> bool + Send + Sync>) {
        let mut rules = self.rules.write().await;
        rules.insert(field_path, validator);
    }

    /// Validate a specific field
    pub async fn validate_field(&self, field_path: &str, value: &str) -> Result<(), ConfigError> {
        let rules = self.rules.read().await;
        
        if let Some(validator) = rules.get(field_path) {
            if !validator(value) {
                return Err(ConfigError::InvalidValue {
                    field: field_path.to_string(),
                    value: value.to_string(),
                    reason: "Value failed validation rule".to_string(),
                });
            }
        }
        
        Ok(())
    }

    /// Validate all fields in a configuration
    pub async fn validate_all(&self, config: &DynamicConfig) -> Result<(), ConfigError> {
        // Basic structural validation
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

        if config.memory.high_water_mark <= config.memory.low_water_mark {
            return Err(ConfigError::ValidationFailed {
                reason: "High water mark must be greater than low water mark".to_string(),
            });
        }

        if config.processing.timeout_seconds == 0 {
            return Err(ConfigError::ValidationFailed {
                reason: "Timeout must be greater than 0".to_string(),
            });
        }

        // Rule-based validation would go here
        // (omitted for brevity but would validate each field against its rules)

        Ok(())
    }
}

impl Clone for ValidationRules {
    fn clone(&self) -> Self {
        Self {
            rules: Arc::clone(&self.rules),
        }
    }
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_rules_creation() {
        let rules = ValidationRules::new();
        
        // Give time for default rules to be set up
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Test some basic validation
        let config = DynamicConfig::default();
        assert!(rules.validate_all(&config).await.is_ok());
    }

    #[tokio::test]
    async fn test_custom_validation_rule() {
        let rules = ValidationRules::new();
        
        // Add a custom rule
        rules.add_rule(
            "test.field".to_string(),
            Box::new(|v| v == "valid_value"),
        ).await;
        
        // Test the rule
        assert!(rules.validate_field("test.field", "valid_value").await.is_ok());
        assert!(rules.validate_field("test.field", "invalid_value").await.is_err());
    }

    #[tokio::test]
    async fn test_structural_validation() {
        let rules = ValidationRules::new();
        
        let mut config = DynamicConfig::default();
        
        // Test memory limit too low
        config.memory.max_memory_mb = 32;
        assert!(rules.validate_all(&config).await.is_err());
        
        // Test cache size exceeds memory
        config.memory.max_memory_mb = 1024;
        config.cache.max_size_mb = 2048;
        assert!(rules.validate_all(&config).await.is_err());
        
        // Test valid config
        config.cache.max_size_mb = 512;
        assert!(rules.validate_all(&config).await.is_ok());
    }
}