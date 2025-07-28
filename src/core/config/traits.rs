/// Unified configuration traits for LSP Bridge
///
/// This module provides configuration traits that enable shared access patterns
/// across different modules while maintaining type safety and clear boundaries.
use std::path::PathBuf;

/// Trait for components that need cache configuration
pub trait HasCacheConfig {
    fn cache_config(&self) -> &CacheConfig;
}

/// Trait for components that need timeout configuration
pub trait HasTimeoutConfig {
    fn timeout_config(&self) -> &TimeoutConfig;
}

/// Trait for components that need performance configuration
pub trait HasPerformanceConfig {
    fn performance_config(&self) -> &PerformanceConfig;
}

/// Trait for components that need memory management configuration
pub trait HasMemoryConfig {
    fn memory_config(&self) -> &MemoryConfig;
}

/// Trait for components that need git integration configuration
pub trait HasGitConfig {
    fn git_config(&self) -> &GitConfig;
}

/// Trait for components that need multi-repository configuration
pub trait HasMultiRepoConfig {
    fn multi_repo_config(&self) -> &MultiRepoConfig;
}

/// Core cache configuration - unified from multiple scattered definitions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    pub enable_cache: bool,
    pub enable_persistent_cache: bool,
    pub cache_dir: PathBuf,
    pub max_size_mb: usize,
    pub max_entries: usize,
    pub ttl_hours: u64,
    pub cleanup_interval_minutes: u64,
    pub enable_compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            enable_persistent_cache: true,
            cache_dir: std::env::temp_dir().join("lsp-bridge-cache"),
            max_size_mb: 100,
            max_entries: 10000,
            ttl_hours: 24,
            cleanup_interval_minutes: 60,
            enable_compression: true,
        }
    }
}

/// Core timeout configuration - unified from various timeout patterns
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeoutConfig {
    pub processing_timeout_seconds: u64,
    pub network_timeout_seconds: u64,
    pub file_operation_timeout_seconds: u64,
    pub analysis_timeout_seconds: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            processing_timeout_seconds: 30,
            network_timeout_seconds: 10,
            file_operation_timeout_seconds: 5,
            analysis_timeout_seconds: 30,
        }
    }
}

/// Core performance configuration - unified from scattered performance settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceConfig {
    pub max_concurrent_files: usize,
    pub chunk_size: usize,
    pub parallel_processing: bool,
    pub file_size_limit_mb: usize,
    pub max_cpu_usage_percent: f64,
    pub adaptive_scaling: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_files: 1000,
            chunk_size: 100,
            parallel_processing: true,
            file_size_limit_mb: 10,
            max_cpu_usage_percent: 80.0,
            adaptive_scaling: true,
        }
    }
}

/// Core memory configuration - unified from multiple memory management configs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryConfig {
    pub max_memory_mb: usize,
    pub max_entries: usize,
    pub eviction_policy: String,
    pub high_water_mark: f64,
    pub low_water_mark: f64,
    pub eviction_batch_size: usize,
    pub monitoring_interval_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 256,
            max_entries: 50000,
            eviction_policy: "Adaptive".to_string(),
            high_water_mark: 0.8,
            low_water_mark: 0.6,
            eviction_batch_size: 100,
            monitoring_interval_seconds: 30,
        }
    }
}

/// Core git configuration - unified from git integration settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitConfig {
    pub enable_git_integration: bool,
    pub scan_interval_seconds: u64,
    pub ignore_untracked: bool,
    pub track_staged_changes: bool,
    pub auto_refresh: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            enable_git_integration: true,
            scan_interval_seconds: 30,
            ignore_untracked: false,
            track_staged_changes: true,
            auto_refresh: true,
        }
    }
}

/// Core analysis configuration for semantic context extraction
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisConfig {
    pub max_context_tokens: usize,
    pub include_dependencies: bool,
    pub include_call_hierarchy: bool,
    pub context_extraction_depth: u32,
    pub enable_incremental_analysis: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 2000,
            include_dependencies: true,
            include_call_hierarchy: true,
            context_extraction_depth: 3,
            enable_incremental_analysis: true,
        }
    }
}

/// Core multi-repository configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiRepoConfig {
    /// Path to the repository registry database
    pub registry_path: PathBuf,
    /// Path to the team collaboration database
    pub team_db_path: Option<PathBuf>,
    /// Enable automatic monorepo detection
    pub auto_detect_monorepo: bool,
    /// Enable cross-repository type tracking
    pub enable_cross_repo_types: bool,
    /// Maximum number of repositories to analyze concurrently
    pub max_concurrent_repos: usize,
    /// Cache directory for cross-repo analysis
    pub cache_dir: PathBuf,
}

impl Default for MultiRepoConfig {
    fn default() -> Self {
        Self {
            registry_path: PathBuf::from(".lsp-bridge/repos.db"),
            team_db_path: None,
            auto_detect_monorepo: true,
            enable_cross_repo_types: true,
            max_concurrent_repos: 4,
            cache_dir: PathBuf::from(".lsp-bridge/cache/multi-repo"),
        }
    }
}

/// Conversion utilities for backward compatibility
impl From<&super::super::dynamic_config::DynamicCacheConfig> for CacheConfig {
    fn from(dynamic: &super::super::dynamic_config::DynamicCacheConfig) -> Self {
        Self {
            enable_cache: dynamic.enable_memory_cache,
            enable_persistent_cache: dynamic.enable_persistent_cache,
            cache_dir: dynamic.cache_dir.clone(),
            max_size_mb: dynamic.max_size_mb,
            max_entries: dynamic.max_entries,
            ttl_hours: dynamic.ttl_hours,
            cleanup_interval_minutes: dynamic.cleanup_interval_minutes,
            enable_compression: true, // Default
        }
    }
}

impl From<&super::super::dynamic_config::ProcessingConfig> for PerformanceConfig {
    fn from(processing: &super::super::dynamic_config::ProcessingConfig) -> Self {
        Self {
            max_concurrent_files: processing.max_concurrent_files,
            chunk_size: processing.chunk_size,
            parallel_processing: processing.parallel_processing,
            file_size_limit_mb: processing.file_size_limit_mb,
            max_cpu_usage_percent: 80.0, // Default
            adaptive_scaling: true,      // Default
        }
    }
}

impl From<&super::super::dynamic_config::DynamicMemoryConfig> for MemoryConfig {
    fn from(dynamic: &super::super::dynamic_config::DynamicMemoryConfig) -> Self {
        Self {
            max_memory_mb: dynamic.max_memory_mb,
            max_entries: dynamic.max_entries,
            eviction_policy: dynamic.eviction_policy.clone(),
            high_water_mark: dynamic.high_water_mark,
            low_water_mark: dynamic.low_water_mark,
            eviction_batch_size: dynamic.eviction_batch_size,
            monitoring_interval_seconds: dynamic.monitoring_interval_seconds,
        }
    }
}

impl From<&super::super::dynamic_config::GitConfig> for GitConfig {
    fn from(dynamic: &super::super::dynamic_config::GitConfig) -> Self {
        Self {
            enable_git_integration: dynamic.enable_git_integration,
            scan_interval_seconds: dynamic.scan_interval_seconds,
            ignore_untracked: dynamic.ignore_untracked,
            track_staged_changes: dynamic.track_staged_changes,
            auto_refresh: dynamic.auto_refresh,
        }
    }
}
