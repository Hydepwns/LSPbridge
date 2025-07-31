//! Types for the enhanced processor

use crate::core::{persistent_cache::CacheStats, metrics::ProcessingMetrics};
use std::path::PathBuf;

/// Configuration for the enhanced processor
#[derive(Debug, Clone)]
pub struct SimpleEnhancedConfig {
    pub cache_dir: PathBuf,
    pub enable_metrics: bool,
    pub enable_persistent_cache: bool,
    pub auto_optimization: bool,
    pub enable_git_integration: bool,
    pub enable_dynamic_config: bool,
    pub config_file: Option<PathBuf>,
}

impl Default for SimpleEnhancedConfig {
    fn default() -> Self {
        Self {
            cache_dir: std::env::temp_dir().join("lsp-bridge-cache"),
            enable_metrics: true,
            enable_persistent_cache: true,
            auto_optimization: true,
            enable_git_integration: true,
            enable_dynamic_config: true,
            config_file: Some(
                std::env::current_dir()
                    .unwrap_or_else(|_| std::env::temp_dir())
                    .join("lspbridge.toml"),
            ),
        }
    }
}

/// Performance summary for the processor
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub core_cache_files: usize,
    pub core_cache_diagnostics: usize,
    pub persistent_cache_stats: Option<CacheStats>,
    pub processing_metrics: Option<ProcessingMetrics>,
    pub error_count: u64,
    pub recent_error_rate: f64,
}