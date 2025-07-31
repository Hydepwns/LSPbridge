/// Unified configuration system for LSP Bridge
///
/// This module consolidates all configuration into a single, coherent structure
/// that replaces the scattered config structs across modules. It maintains
/// backward compatibility while providing a cleaner architecture.
use super::traits::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::core::SecurityConfig;

/// Unified configuration structure that consolidates all LSP Bridge settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedConfig {
    /// Cache configuration for all caching operations
    pub cache: CacheConfig,

    /// Timeout configuration for all operations
    pub timeouts: TimeoutConfig,

    /// Performance configuration for processing and optimization
    pub performance: PerformanceConfig,

    /// Memory management configuration
    pub memory: MemoryConfig,

    /// Git integration configuration
    pub git: GitConfig,

    /// Analysis configuration for semantic context extraction
    pub analysis: AnalysisConfig,

    /// Multi-repository configuration
    pub multi_repo: super::traits::MultiRepoConfig,

    /// Error recovery configuration
    pub error_recovery: ErrorRecoveryConfig,

    /// Metrics and monitoring configuration
    pub metrics: MetricsConfig,

    /// Feature flags for experimental features
    pub features: FeatureFlags,

    /// Security configuration with secure defaults
    pub security: super::super::security_config::SecurityConfig,
}

/// Error recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    pub enable_circuit_breaker: bool,
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub failure_threshold: usize,
    pub success_threshold: usize,
    pub timeout_ms: u64,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            enable_circuit_breaker: true,
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            failure_threshold: 5,
            success_threshold: 3,
            timeout_ms: 10000,
        }
    }
}

/// Metrics and monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enable_metrics: bool,
    pub prometheus_port: u16,
    pub collection_interval_seconds: u64,
    pub retention_hours: u64,
    pub export_format: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            prometheus_port: 9090,
            collection_interval_seconds: 10,
            retention_hours: 72,
            export_format: "prometheus".to_string(),
        }
    }
}

/// Feature flags for experimental and optional functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub auto_optimization: bool,
    pub health_monitoring: bool,
    pub cache_warming: bool,
    pub advanced_diagnostics: bool,
    pub experimental_features: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            auto_optimization: true,
            health_monitoring: true,
            cache_warming: true,
            advanced_diagnostics: false,
            experimental_features: false,
        }
    }
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        let security = super::super::security_config::SecurityConfig::new();
        let mut config = Self {
            cache: CacheConfig::default(),
            timeouts: TimeoutConfig::default(),
            performance: PerformanceConfig::default(),
            memory: MemoryConfig::default(),
            git: GitConfig::default(),
            analysis: AnalysisConfig::default(),
            multi_repo: super::traits::MultiRepoConfig::default(),
            error_recovery: ErrorRecoveryConfig::default(),
            metrics: MetricsConfig::default(),
            features: FeatureFlags::default(),
            security: security.clone(),
        };
        
        // Apply security config to ensure secure defaults
        security.apply_to_unified_config(&mut config);
        config
    }
}

impl UnifiedConfig {
    /// Create a new unified config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a production-ready configuration with strict security
    pub fn production() -> Self {
        let security = super::super::security_config::SecurityConfig::strict();
        let mut config = Self {
            cache: CacheConfig::default(),
            timeouts: TimeoutConfig::default(),
            performance: PerformanceConfig::default(),
            memory: MemoryConfig::default(),
            git: GitConfig::default(),
            analysis: AnalysisConfig::default(),
            multi_repo: super::traits::MultiRepoConfig::default(),
            error_recovery: ErrorRecoveryConfig::default(),
            metrics: MetricsConfig::default(),
            features: FeatureFlags {
                auto_optimization: true,
                health_monitoring: true,
                cache_warming: true,
                advanced_diagnostics: false,  // Conservative for production
                experimental_features: false, // Never enable in production
            },
            security: security.clone(),
        };
        
        // Apply strict security constraints
        security.apply_to_unified_config(&mut config);
        
        // Additional production hardening
        config.git.auto_refresh = false;  // Manual refresh in production
        config.metrics.enable_metrics = true;  // Always monitor in production
        config.error_recovery.enable_circuit_breaker = true;
        
        config
    }

    /// Create a development-friendly configuration with relaxed security
    pub fn development() -> Self {
        let security = super::super::security_config::SecurityConfig::development();
        let mut config = Self {
            cache: CacheConfig {
                max_size_mb: 200,  // Larger cache for dev
                cleanup_interval_minutes: 120,  // Less frequent cleanup
                ..CacheConfig::default()
            },
            timeouts: TimeoutConfig {
                processing_timeout_seconds: 300,  // Longer timeouts for debugging
                network_timeout_seconds: 60,
                ..TimeoutConfig::default()
            },
            performance: PerformanceConfig {
                max_concurrent_files: 2000,  // Higher limits for dev
                adaptive_scaling: true,
                ..PerformanceConfig::default()
            },
            memory: MemoryConfig {
                max_memory_mb: 1024,  // More memory for dev
                monitoring_interval_seconds: 60,  // Less frequent monitoring
                ..MemoryConfig::default()
            },
            features: FeatureFlags {
                auto_optimization: true,
                health_monitoring: true,
                cache_warming: false,  // Skip cache warming in dev
                advanced_diagnostics: true,  // Enable for debugging
                experimental_features: true,  // Allow experimental features
            },
            security: security.clone(),
            ..Self::default()
        };
        
        // Apply development security settings
        security.apply_to_unified_config(&mut config);
        
        config
    }

    /// Create a testing configuration optimized for CI/CD
    pub fn testing() -> Self {
        let security = super::super::security_config::SecurityConfig::development();
        let config = Self {
            cache: CacheConfig {
                enable_persistent_cache: false,  // Memory-only cache for tests
                max_size_mb: 50,  // Smaller cache for tests
                ..CacheConfig::default()
            },
            timeouts: TimeoutConfig {
                processing_timeout_seconds: 60,  // Shorter timeouts for tests
                network_timeout_seconds: 15,
                file_operation_timeout_seconds: 10,
                analysis_timeout_seconds: 30,
            },
            performance: PerformanceConfig {
                max_concurrent_files: 100,  // Lower concurrency for stable tests
                chunk_size: 50,
                parallel_processing: false,  // Sequential processing for predictable tests
                ..PerformanceConfig::default()
            },
            memory: MemoryConfig {
                max_memory_mb: 128,  // Minimal memory for tests
                max_entries: 1000,
                ..MemoryConfig::default()
            },
            git: GitConfig {
                enable_git_integration: false,  // Disable git in tests
                ..GitConfig::default()
            },
            features: FeatureFlags {
                auto_optimization: false,  // Disable auto-optimization in tests
                health_monitoring: false,  // Disable monitoring in tests
                cache_warming: false,
                advanced_diagnostics: false,
                experimental_features: false,
            },
            metrics: MetricsConfig {
                enable_metrics: false,  // Disable metrics collection in tests
                ..MetricsConfig::default()
            },
            security,
            ..Self::default()
        };
        
        config
    }

    /// Load configuration from file, falling back to defaults if file doesn't exist
    pub async fn load_or_default(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::load(path).await
        } else {
            Ok(Self::new())
        }
    }

    /// Load configuration from TOML file
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub async fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Security validation first (most critical)
        self.security.validate().map_err(|e| anyhow::anyhow!("Security validation failed: {}", e))?;

        // Memory validation with security constraints
        let min_memory = 64.max(self.security.resource_limits.max_memory_mb / 4); // At least 1/4 of security limit
        if self.memory.max_memory_mb < min_memory {
            anyhow::bail!("Memory limit too low: minimum {}MB (based on security config)", min_memory);
        }

        if self.cache.max_size_mb > self.memory.max_memory_mb {
            anyhow::bail!("Cache size cannot exceed memory limit");
        }

        // Cache size must respect security limits
        if self.cache.max_size_mb > self.security.resource_limits.max_cache_size_mb {
            anyhow::bail!("Cache size exceeds security limit of {}MB", self.security.resource_limits.max_cache_size_mb);
        }

        // Performance validation with security constraints
        let max_cpu = self.performance.max_cpu_usage_percent.min(self.security.resource_limits.max_cpu_percent);
        if self.performance.max_cpu_usage_percent > max_cpu {
            anyhow::bail!("CPU usage cannot exceed security limit of {}%", max_cpu);
        }

        if self.performance.max_concurrent_files == 0 {
            anyhow::bail!("Max concurrent files must be greater than 0");
        }

        if self.performance.max_concurrent_files > self.security.resource_limits.max_concurrent_operations {
            anyhow::bail!("Max concurrent files exceeds security limit of {}", self.security.resource_limits.max_concurrent_operations);
        }

        // File size validation
        if self.performance.file_size_limit_mb > self.security.input_validation.max_file_size_mb {
            anyhow::bail!("File size limit exceeds security limit of {}MB", self.security.input_validation.max_file_size_mb);
        }

        // Timeout validation with security constraints
        if self.timeouts.processing_timeout_seconds == 0 {
            anyhow::bail!("Processing timeout must be greater than 0");
        }

        if self.timeouts.processing_timeout_seconds > self.security.resource_limits.max_processing_time_seconds {
            anyhow::bail!("Processing timeout exceeds security limit of {}s", self.security.resource_limits.max_processing_time_seconds);
        }

        // Network timeout validation
        if self.timeouts.network_timeout_seconds > self.security.network.network_timeout_seconds {
            anyhow::bail!("Network timeout exceeds security limit of {}s", self.security.network.network_timeout_seconds);
        }

        // Metrics validation
        if self.metrics.prometheus_port < 1024 || self.metrics.prometheus_port > 65535 {
            anyhow::bail!("Prometheus port must be between 1024 and 65535");
        }

        // Git integration safety
        if self.git.scan_interval_seconds > 0 && self.git.scan_interval_seconds < 10 {
            anyhow::bail!("Git scan interval too frequent: minimum 10 seconds to prevent resource exhaustion");
        }

        Ok(())
    }
}

// Implement the configuration traits for UnifiedConfig
impl HasCacheConfig for UnifiedConfig {
    fn cache_config(&self) -> &CacheConfig {
        &self.cache
    }
}

impl HasTimeoutConfig for UnifiedConfig {
    fn timeout_config(&self) -> &TimeoutConfig {
        &self.timeouts
    }
}

impl HasPerformanceConfig for UnifiedConfig {
    fn performance_config(&self) -> &PerformanceConfig {
        &self.performance
    }
}

impl HasMemoryConfig for UnifiedConfig {
    fn memory_config(&self) -> &MemoryConfig {
        &self.memory
    }
}

impl HasGitConfig for UnifiedConfig {
    fn git_config(&self) -> &GitConfig {
        &self.git
    }
}

impl super::traits::HasMultiRepoConfig for UnifiedConfig {
    fn multi_repo_config(&self) -> &super::traits::MultiRepoConfig {
        &self.multi_repo
    }
}

/// Migration utilities for backward compatibility with existing config types
impl UnifiedConfig {
    /// Convert from the existing DynamicConfig for migration
    pub fn from_dynamic_config(dynamic: &crate::core::dynamic_config::DynamicConfig) -> Self {
        Self {
            cache: CacheConfig::from(&dynamic.cache),
            timeouts: TimeoutConfig::default(), // Not in dynamic config
            performance: PerformanceConfig::from(&dynamic.processing),
            memory: MemoryConfig::from(&dynamic.memory),
            git: GitConfig::from(&dynamic.git),
            analysis: AnalysisConfig::default(), // Not in dynamic config
            multi_repo: super::traits::MultiRepoConfig::default(), // Not in dynamic config
            error_recovery: ErrorRecoveryConfig {
                enable_circuit_breaker: dynamic.error_recovery.enable_circuit_breaker,
                max_retries: dynamic.error_recovery.max_retries,
                initial_delay_ms: dynamic.error_recovery.initial_delay_ms,
                max_delay_ms: dynamic.error_recovery.max_delay_ms,
                backoff_multiplier: dynamic.error_recovery.backoff_multiplier,
                failure_threshold: dynamic.error_recovery.failure_threshold,
                success_threshold: dynamic.error_recovery.success_threshold,
                timeout_ms: dynamic.error_recovery.timeout_ms,
            },
            metrics: MetricsConfig {
                enable_metrics: dynamic.metrics.enable_metrics,
                prometheus_port: dynamic.metrics.prometheus_port,
                collection_interval_seconds: dynamic.metrics.collection_interval_seconds,
                retention_hours: dynamic.metrics.retention_hours,
                export_format: dynamic.metrics.export_format.clone(),
            },
            features: FeatureFlags {
                auto_optimization: dynamic.features.auto_optimization,
                health_monitoring: dynamic.features.health_monitoring,
                cache_warming: dynamic.features.cache_warming,
                advanced_diagnostics: dynamic.features.advanced_diagnostics,
                experimental_features: dynamic.features.experimental_features,
            },
            security: SecurityConfig::default(), // Not in dynamic config
        }
    }

    /// Convert to DynamicConfig for backward compatibility
    pub fn to_dynamic_config(&self) -> crate::core::dynamic_config::DynamicConfig {
        use crate::core::dynamic_config::*;

        DynamicConfig {
            processing: ProcessingConfig {
                parallel_processing: self.performance.parallel_processing,
                chunk_size: self.performance.chunk_size,
                max_concurrent_files: self.performance.max_concurrent_files,
                file_size_limit_mb: self.performance.file_size_limit_mb,
                timeout_seconds: self.timeouts.processing_timeout_seconds,
            },
            cache: DynamicCacheConfig {
                enable_persistent_cache: self.cache.enable_persistent_cache,
                enable_memory_cache: self.cache.enable_cache,
                cache_dir: self.cache.cache_dir.clone(),
                max_size_mb: self.cache.max_size_mb,
                max_entries: self.cache.max_entries,
                ttl_hours: self.cache.ttl_hours,
                cleanup_interval_minutes: self.cache.cleanup_interval_minutes,
            },
            memory: DynamicMemoryConfig {
                max_memory_mb: self.memory.max_memory_mb,
                max_entries: self.memory.max_entries,
                eviction_policy: self.memory.eviction_policy.clone(),
                high_water_mark: self.memory.high_water_mark,
                low_water_mark: self.memory.low_water_mark,
                eviction_batch_size: self.memory.eviction_batch_size,
                monitoring_interval_seconds: self.memory.monitoring_interval_seconds,
            },
            error_recovery: DynamicErrorRecoveryConfig {
                enable_circuit_breaker: self.error_recovery.enable_circuit_breaker,
                max_retries: self.error_recovery.max_retries,
                initial_delay_ms: self.error_recovery.initial_delay_ms,
                max_delay_ms: self.error_recovery.max_delay_ms,
                backoff_multiplier: self.error_recovery.backoff_multiplier,
                failure_threshold: self.error_recovery.failure_threshold,
                success_threshold: self.error_recovery.success_threshold,
                timeout_ms: self.error_recovery.timeout_ms,
            },
            git: crate::core::dynamic_config::GitConfig {
                enable_git_integration: self.git.enable_git_integration,
                scan_interval_seconds: self.git.scan_interval_seconds,
                ignore_untracked: self.git.ignore_untracked,
                track_staged_changes: self.git.track_staged_changes,
                auto_refresh: self.git.auto_refresh,
            },
            metrics: crate::core::dynamic_config::MetricsConfig {
                enable_metrics: self.metrics.enable_metrics,
                prometheus_port: self.metrics.prometheus_port,
                collection_interval_seconds: self.metrics.collection_interval_seconds,
                retention_hours: self.metrics.retention_hours,
                export_format: self.metrics.export_format.clone(),
            },
            features: crate::core::dynamic_config::FeatureFlags {
                auto_optimization: self.features.auto_optimization,
                health_monitoring: self.features.health_monitoring,
                cache_warming: self.features.cache_warming,
                advanced_diagnostics: self.features.advanced_diagnostics,
                experimental_features: self.features.experimental_features,
            },
            performance: crate::core::dynamic_config::PerformanceConfig {
                optimization_interval_minutes: 60, // Default
                health_check_interval_minutes: 5,  // Default
                gc_threshold_mb: 512,              // Default
                max_cpu_usage_percent: self.performance.max_cpu_usage_percent,
                adaptive_scaling: self.performance.adaptive_scaling,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_unified_config_creation() {
        let config = UnifiedConfig::new();
        assert!(config.validate().is_ok());

        // Test default values (after security constraints are applied)
        assert_eq!(config.cache.max_size_mb, 100);
        assert_eq!(config.performance.max_concurrent_files, 50); // Limited by security config
        assert_eq!(config.memory.max_memory_mb, 256);
    }

    #[tokio::test]
    async fn test_config_save_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.toml");

        let mut config = UnifiedConfig::new();
        config.cache.max_size_mb = 100; // Within security limit of 128MB
        config.performance.max_concurrent_files = 40; // Within security limit of 50

        config.save(&config_file).await?;
        assert!(config_file.exists());

        let loaded_config = UnifiedConfig::load(&config_file).await?;
        assert_eq!(loaded_config.cache.max_size_mb, 100);
        assert_eq!(loaded_config.performance.max_concurrent_files, 40);

        Ok(())
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = UnifiedConfig::new();

        // Test invalid memory limit
        config.memory.max_memory_mb = 32;
        assert!(config.validate().is_err());

        // Test cache size exceeding memory limit
        config.memory.max_memory_mb = 256;
        config.cache.max_size_mb = 512;
        assert!(config.validate().is_err());

        // Test invalid CPU usage
        config.cache.max_size_mb = 100;
        config.performance.max_cpu_usage_percent = 150.0;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_migration_compatibility() {
        let dynamic_config = crate::core::dynamic_config::DynamicConfig::default();
        let unified_config = UnifiedConfig::from_dynamic_config(&dynamic_config);

        assert_eq!(
            unified_config.cache.enable_persistent_cache,
            dynamic_config.cache.enable_persistent_cache
        );
        assert_eq!(
            unified_config.memory.max_memory_mb,
            dynamic_config.memory.max_memory_mb
        );
        assert_eq!(
            unified_config.performance.parallel_processing,
            dynamic_config.processing.parallel_processing
        );

        // Test round-trip conversion
        let converted_back = unified_config.to_dynamic_config();
        assert_eq!(
            converted_back.cache.enable_persistent_cache,
            dynamic_config.cache.enable_persistent_cache
        );
    }
}
