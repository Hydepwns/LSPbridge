//! Dynamic configuration data structures and types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::core::{CacheConfig, EvictionPolicy, MemoryConfig, RecoveryStrategy};

/// Main dynamic configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicConfig {
    /// Core processing settings
    pub processing: ProcessingConfig,

    /// Cache settings
    pub cache: DynamicCacheConfig,

    /// Memory management
    pub memory: DynamicMemoryConfig,

    /// Error recovery
    pub error_recovery: DynamicErrorRecoveryConfig,

    /// Git integration
    pub git: GitConfig,

    /// Metrics and monitoring
    pub metrics: MetricsConfig,

    /// Feature flags
    pub features: FeatureFlags,

    /// Performance tuning
    pub performance: PerformanceConfig,
}

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub parallel_processing: bool,
    pub chunk_size: usize,
    pub max_concurrent_files: usize,
    pub file_size_limit_mb: usize,
    pub timeout_seconds: u64,
}

/// Dynamic cache configuration
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

/// Dynamic memory configuration
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

/// Dynamic error recovery configuration
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

/// Git integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub enable_git_integration: bool,
    pub scan_interval_seconds: u64,
    pub ignore_untracked: bool,
    pub track_staged_changes: bool,
    pub auto_refresh: bool,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enable_metrics: bool,
    pub prometheus_port: u16,
    pub collection_interval_seconds: u64,
    pub retention_hours: u64,
    pub export_format: String, // "prometheus", "json", "csv"
}

/// Feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_smart_caching: bool,
    pub enable_advanced_filtering: bool,
    pub enable_batch_processing: bool,
    pub enable_experimental_features: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_cpu_usage_percent: f64,
    pub io_priority: String, // "low", "normal", "high"
    pub enable_parallel_io: bool,
}

/// Configuration change notification
#[derive(Debug, Clone)]
pub struct ConfigChange {
    pub field_path: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: SystemTime,
}

impl Default for DynamicConfig {
    fn default() -> Self {
        Self {
            processing: ProcessingConfig {
                parallel_processing: true,
                chunk_size: 100,
                max_concurrent_files: 10,
                file_size_limit_mb: 100,
                timeout_seconds: 30,
            },
            cache: DynamicCacheConfig {
                enable_persistent_cache: true,
                enable_memory_cache: true,
                cache_dir: PathBuf::from(".lsp-bridge/cache"),
                max_size_mb: 512,
                max_entries: 10000,
                ttl_hours: 24,
                cleanup_interval_minutes: 30,
            },
            memory: DynamicMemoryConfig {
                max_memory_mb: 1024,
                max_entries: 50000,
                eviction_policy: "Adaptive".to_string(),
                high_water_mark: 0.8,
                low_water_mark: 0.6,
                eviction_batch_size: 100,
                monitoring_interval_seconds: 10,
            },
            error_recovery: DynamicErrorRecoveryConfig {
                enable_circuit_breaker: true,
                max_retries: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2.0,
                failure_threshold: 5,
                success_threshold: 2,
                timeout_ms: 30000,
            },
            git: GitConfig {
                enable_git_integration: true,
                scan_interval_seconds: 30,
                ignore_untracked: true,
                track_staged_changes: true,
                auto_refresh: true,
            },
            metrics: MetricsConfig {
                enable_metrics: true,
                prometheus_port: 9090,
                collection_interval_seconds: 60,
                retention_hours: 168, // 1 week
                export_format: "prometheus".to_string(),
            },
            features: FeatureFlags {
                enable_smart_caching: true,
                enable_advanced_filtering: true,
                enable_batch_processing: true,
                enable_experimental_features: false,
            },
            performance: PerformanceConfig {
                max_cpu_usage_percent: 80.0,
                io_priority: "normal".to_string(),
                enable_parallel_io: true,
            },
        }
    }
}

// Conversion utilities for integrating with existing config types

impl DynamicCacheConfig {
    /// Convert to the static cache config type
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
    /// Convert to the static memory config type
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
    /// Convert to the static recovery strategy type
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