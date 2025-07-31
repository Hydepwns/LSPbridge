//! Enhanced processor with caching, metrics, and integrations

pub mod integrations;
pub mod pipeline;
pub mod strategies;
pub mod types;

use crate::core::{
    CacheConfig, ConfigChange, Diagnostic, DynamicConfig, DynamicConfigManager, ErrorRecoverySystem,
    GitIntegration, IncrementalProcessor, MetricsCollector, PersistentCache, ProcessingStats,
    RecoveryStrategy,
};
use anyhow::Result;
use integrations::{ConfigIntegration, GitIntegrationWrapper};
use pipeline::ProcessingPipeline;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use strategies::{CacheStrategy, ChangeDetectionStrategy, OptimizationStrategy};
use tokio::sync::RwLock;
use tracing::warn;

/// Enhanced processor with multiple features
pub struct SimpleEnhancedProcessor {
    // Core components
    cache_strategy: Arc<CacheStrategy>,
    change_detection: Arc<ChangeDetectionStrategy>,
    optimization_strategy: Arc<OptimizationStrategy>,
    pipeline: Arc<ProcessingPipeline>,
    
    // Integrations
    git_integration: GitIntegrationWrapper,
    config_integration: ConfigIntegration,
    
    // Other components
    error_recovery: Arc<ErrorRecoverySystem>,
    metrics: Option<Arc<MetricsCollector>>,
    config: types::SimpleEnhancedConfig,
    last_optimization: RwLock<Instant>,
}

impl SimpleEnhancedProcessor {
    /// Create a new enhanced processor
    pub async fn new(config: types::SimpleEnhancedConfig) -> Result<Self> {
        let core_processor = Arc::new(
            IncrementalProcessor::new()
                .with_parallel(true)
                .with_chunk_size(100),
        );

        let persistent_cache = if config.enable_persistent_cache {
            let cache_config = CacheConfig {
                cache_dir: config.cache_dir.clone(),
                ..Default::default()
            };
            Some(Arc::new(PersistentCache::new(cache_config).await?))
        } else {
            None
        };

        let error_recovery = Arc::new(ErrorRecoverySystem::new(RecoveryStrategy::default()));

        let metrics = if config.enable_metrics {
            Some(Arc::new(MetricsCollector::new()?))
        } else {
            None
        };

        let git = if config.enable_git_integration {
            match GitIntegration::new().await {
                Ok(git) => Some(Arc::new(git)),
                Err(e) => {
                    warn!("Git integration failed to initialize: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let dynamic_config = if config.enable_dynamic_config {
            if let Some(config_file) = &config.config_file {
                match DynamicConfigManager::new(config_file.clone()).await {
                    Ok(manager) => {
                        let _receiver = manager.subscribe_to_changes();
                        Some(Arc::new(manager))
                    }
                    Err(e) => {
                        warn!("Dynamic configuration failed to initialize: {}", e);
                        None
                    }
                }
            } else {
                warn!("Dynamic configuration enabled but no config file specified");
                None
            }
        } else {
            None
        };

        // Create strategies
        let cache_strategy = Arc::new(CacheStrategy::new(
            core_processor.clone(),
            persistent_cache.clone(),
            metrics.clone(),
        ));

        let change_detection = Arc::new(ChangeDetectionStrategy::new(
            core_processor,
            git.clone(),
            metrics.clone(),
        ));

        let optimization_strategy = Arc::new(OptimizationStrategy::new(
            persistent_cache,
            error_recovery.clone(),
        ));

        let pipeline = Arc::new(ProcessingPipeline::new(
            cache_strategy.clone(),
            change_detection.clone(),
            metrics.clone(),
        ));

        Ok(Self {
            cache_strategy,
            change_detection,
            optimization_strategy,
            pipeline,
            git_integration: GitIntegrationWrapper::new(git),
            config_integration: ConfigIntegration::new(dynamic_config),
            error_recovery,
            metrics,
            config,
            last_optimization: RwLock::new(Instant::now()),
        })
    }

    // Cache operations

    /// Detect changed files
    pub async fn detect_changed_files(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        self.change_detection.detect_changed_files(files).await
    }

    /// Get cached diagnostics for a file
    pub async fn get_cached_diagnostics(&self, file_path: &Path) -> Option<Vec<Diagnostic>> {
        self.cache_strategy.get_cached_diagnostics(file_path).await
    }

    /// Update cache with new diagnostics
    pub async fn update_cache(
        &self,
        file_path: &PathBuf,
        diagnostics: &[Diagnostic],
    ) -> Result<()> {
        self.cache_strategy.update_cache(file_path, diagnostics).await
    }

    /// Clear all caches
    pub async fn clear_all_caches(&self) -> Result<()> {
        self.cache_strategy.clear_all_caches().await
    }

    // Processing operations

    /// Process files incrementally
    pub async fn process_files_incrementally(
        &self,
        files: &[PathBuf],
        processor_fn: impl Fn(&[PathBuf]) -> Result<HashMap<PathBuf, Vec<Diagnostic>>>,
    ) -> Result<(Vec<Diagnostic>, ProcessingStats)> {
        let result = self
            .pipeline
            .process_files_incrementally(files, processor_fn)
            .await?;

        // Maybe run optimization
        if self.config.auto_optimization {
            self.maybe_optimize().await?;
        }

        Ok(result)
    }

    // Performance operations

    /// Get performance summary
    pub async fn get_performance_summary(&self) -> Result<types::PerformanceSummary> {
        let (core_files, core_diagnostics, persistent_stats) = 
            self.cache_strategy.get_cache_stats().await;

        let processing_metrics = if let Some(collector) = &self.metrics {
            Some(collector.get_comprehensive_metrics().await)
        } else {
            None
        };

        let error_stats = self.error_recovery.get_error_statistics().await;

        Ok(types::PerformanceSummary {
            core_cache_files: core_files,
            core_cache_diagnostics: core_diagnostics,
            persistent_cache_stats: persistent_stats,
            processing_metrics,
            error_count: error_stats.total_errors,
            recent_error_rate: error_stats.recent_error_rate,
        })
    }

    /// Maybe run optimization if enough time has passed
    async fn maybe_optimize(&self) -> Result<()> {
        self.optimization_strategy.maybe_optimize().await
    }

    /// Run optimization immediately
    pub async fn optimize(&self) -> Result<()> {
        self.optimization_strategy.optimize().await
    }

    // Git integration delegations

    /// Get Git repository information
    pub async fn get_git_repository_info(&self) -> Option<crate::core::GitRepositoryInfo> {
        self.git_integration.get_repository_info().await
    }

    /// Get Git modified files
    pub async fn get_git_modified_files(&self) -> Result<Vec<PathBuf>> {
        self.git_integration.get_modified_files().await
    }

    /// Get conflicted files
    pub async fn get_conflicted_files(&self) -> Result<Vec<PathBuf>> {
        self.git_integration.get_conflicted_files().await
    }

    /// Get untracked files
    pub async fn get_untracked_files(&self) -> Result<Vec<PathBuf>> {
        self.git_integration.get_untracked_files().await
    }

    /// Check if file is ignored by Git
    pub async fn is_file_ignored(&self, file_path: &Path) -> Result<bool> {
        self.git_integration.is_file_ignored(file_path).await
    }

    /// Check if Git repository is clean
    pub async fn is_git_repository_clean(&self) -> Result<bool> {
        self.git_integration.is_repository_clean().await
    }

    /// Refresh Git status
    pub async fn refresh_git_status(&self) -> Result<()> {
        self.git_integration.refresh_git_status().await
    }

    // Dynamic configuration delegations

    /// Get current dynamic configuration
    pub async fn get_dynamic_config(&self) -> Option<DynamicConfig> {
        self.config_integration.get_config().await
    }

    /// Update dynamic configuration
    pub async fn update_dynamic_config<F>(&self, updater: F) -> Result<Vec<ConfigChange>>
    where
        F: FnOnce(&mut DynamicConfig) -> Result<()>,
    {
        self.config_integration.update_config(updater).await
    }

    /// Reload configuration from file
    pub async fn reload_config_from_file(&self) -> Result<Vec<ConfigChange>> {
        self.config_integration.reload_from_file().await
    }

    /// Save current configuration
    pub async fn save_current_config(&self) -> Result<()> {
        self.config_integration.save_current_config().await
    }

    /// Set configuration field value
    pub async fn set_config_field(&self, field_path: &str, value: &str) -> Result<ConfigChange> {
        self.config_integration.set_field_value(field_path, value).await
    }

    /// Get configuration field value
    pub async fn get_config_field(&self, field_path: &str) -> Result<String> {
        self.config_integration.get_field_value(field_path).await
    }

    /// Watch configuration field for changes
    pub async fn watch_config_field(&self, field_path: String) -> Result<()> {
        self.config_integration.watch_field(field_path).await
    }

    /// Apply dynamic configuration to processor
    pub async fn apply_dynamic_config_to_processor(&self) -> Result<()> {
        self.config_integration.apply_to_processor().await
    }

    /// Handle configuration changes (placeholder for future implementation)
    pub async fn handle_config_changes(&self) -> Result<()> {
        // TODO: Handle config changes when receiver is implemented
        Ok(())
    }
}

// Re-export types
pub use types::{PerformanceSummary, SimpleEnhancedConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_simple_enhanced_processor() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = types::SimpleEnhancedConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let processor = SimpleEnhancedProcessor::new(config).await?;

        // Test basic functionality
        let summary = processor.get_performance_summary().await?;
        assert_eq!(summary.core_cache_files, 0);

        Ok(())
    }
}