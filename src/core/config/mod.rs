/// Unified configuration system for LSP Bridge
///
/// This module provides a consolidated configuration system that replaces
/// the scattered config structs throughout the codebase. It includes:
///
/// - Unified configuration structure with all settings
/// - Configuration traits for type-safe access patterns
/// - Migration utilities for backward compatibility
/// - Validation and serialization support
pub mod traits;
pub mod unified;

// Re-export the main types for easy access
pub use traits::{
    AnalysisConfig, CacheConfig, GitConfig, HasCacheConfig, HasGitConfig, HasMemoryConfig,
    HasMultiRepoConfig, HasPerformanceConfig, HasTimeoutConfig, MemoryConfig, MultiRepoConfig,
    PerformanceConfig, TimeoutConfig,
};

pub use unified::{ErrorRecoveryConfig, FeatureFlags, MetricsConfig, UnifiedConfig};


// Re-export the original config system for backward compatibility
pub use legacy::{Config, ConfigDefaults};

// Re-export the macro from the crate root
pub use crate::impl_config_defaults;

mod legacy;
