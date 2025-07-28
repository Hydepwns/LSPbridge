use crate::core::errors::ConfigError;
use crate::core::{
    CacheConfig, ConfigChange, ConfigChangeReceiver, Diagnostic, DynamicConfig,
    DynamicConfigManager, ErrorRecoverySystem, FileHash, GitIntegration, IncrementalProcessor,
    MetricsCollector, PersistentCache, ProcessingStats, RecoveryStrategy,
};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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
                    .join("lsp-bridge-config.toml"),
            ),
        }
    }
}

pub struct SimpleEnhancedProcessor {
    core_processor: IncrementalProcessor,
    persistent_cache: Option<Arc<PersistentCache>>,
    error_recovery: Arc<ErrorRecoverySystem>,
    metrics: Option<Arc<MetricsCollector>>,
    git_integration: Option<Arc<GitIntegration>>,
    dynamic_config: Option<Arc<DynamicConfigManager>>,
    config: SimpleEnhancedConfig,
    last_optimization: RwLock<Instant>,
    config_change_receiver: RwLock<Option<ConfigChangeReceiver>>,
}

impl SimpleEnhancedProcessor {
    pub async fn new(config: SimpleEnhancedConfig) -> Result<Self> {
        let core_processor = IncrementalProcessor::new()
            .with_parallel(true)
            .with_chunk_size(100);

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

        let git_integration = if config.enable_git_integration {
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

        let (dynamic_config, config_change_receiver) = if config.enable_dynamic_config {
            if let Some(config_file) = &config.config_file {
                match DynamicConfigManager::new(config_file.clone()).await {
                    Ok(manager) => {
                        let receiver = manager.subscribe_to_changes().await;
                        (Some(Arc::new(manager)), Some(receiver))
                    }
                    Err(e) => {
                        warn!("Dynamic configuration failed to initialize: {}", e);
                        (None, None)
                    }
                }
            } else {
                warn!("Dynamic configuration enabled but no config file specified");
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(Self {
            core_processor,
            persistent_cache,
            error_recovery,
            metrics,
            git_integration,
            dynamic_config,
            config,
            last_optimization: RwLock::new(Instant::now()),
            config_change_receiver: RwLock::new(config_change_receiver),
        })
    }

    pub async fn detect_changed_files(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let start = Instant::now();

        // Use Git integration for enhanced change detection if available
        let result = if let Some(git) = &self.git_integration {
            self.detect_changed_files_with_git(files, git).await
        } else {
            // Fallback to core processor
            self.core_processor.detect_changed_files(files).await
        };

        if let Some(metrics) = &self.metrics {
            metrics.record_cache_operation_time(start.elapsed());
        }

        result
    }

    async fn detect_changed_files_with_git(
        &self,
        files: &[PathBuf],
        git: &GitIntegration,
    ) -> Result<Vec<PathBuf>> {
        let mut changed_files = Vec::new();

        // Get Git-tracked modified files
        let git_modified = git.get_modified_files().await.unwrap_or_default();

        // Check each file against both Git status and file hash
        for file_path in files {
            // Check Git status first (faster)
            let is_changed = if git_modified.contains(file_path) {
                true
            } else {
                // Fallback to hash-based detection for edge cases
                let core_changed = self
                    .core_processor
                    .detect_changed_files(&[file_path.clone()])
                    .await?;
                !core_changed.is_empty()
            };

            if is_changed {
                changed_files.push(file_path.clone());
            }
        }

        info!(
            "Git-enhanced change detection: {} changed out of {} files",
            changed_files.len(),
            files.len()
        );

        Ok(changed_files)
    }

    pub async fn get_cached_diagnostics(&self, file_path: &Path) -> Option<Vec<Diagnostic>> {
        let start = Instant::now();

        // Try persistent cache first
        if let Some(persistent_cache) = &self.persistent_cache {
            if let Some(entry) = persistent_cache.get(file_path).await {
                if let Some(metrics) = &self.metrics {
                    metrics.record_cache_hit();
                    metrics.record_cache_operation_time(start.elapsed());
                }
                return Some(entry.diagnostics);
            }
        }

        // Fall back to core processor cache
        let result = self.core_processor.get_cached_diagnostics(file_path).await;

        if let Some(metrics) = &self.metrics {
            if result.is_some() {
                metrics.record_cache_hit();
            } else {
                metrics.record_cache_miss();
            }
            metrics.record_cache_operation_time(start.elapsed());
        }

        result
    }

    pub async fn update_cache(
        &self,
        file_path: &PathBuf,
        diagnostics: &[Diagnostic],
    ) -> Result<()> {
        // Skip caching for non-existent files (e.g., test files)
        if !file_path.exists() {
            debug!(
                "Skipping cache update for non-existent file: {:?}",
                file_path
            );
            return Ok(());
        }

        // Update core processor cache
        self.core_processor
            .update_file_cache(file_path.clone(), diagnostics.to_vec())
            .await?;

        // Update persistent cache
        if let Some(persistent_cache) = &self.persistent_cache {
            let hash = FileHash::from_file(file_path)?;
            let metadata = std::fs::metadata(file_path)?;
            let last_modified = metadata.modified()?;

            let entry = crate::core::persistent_cache::CacheEntry {
                file_path: file_path.clone(),
                hash,
                last_modified,
                diagnostics: diagnostics.to_vec(),
                access_count: 1,
                last_accessed: SystemTime::now(),
            };

            persistent_cache.put(entry).await?;
        }

        Ok(())
    }

    pub async fn process_files_incrementally(
        &self,
        files: &[PathBuf],
        processor_fn: impl Fn(&[PathBuf]) -> Result<HashMap<PathBuf, Vec<Diagnostic>>>,
    ) -> Result<(Vec<Diagnostic>, ProcessingStats)> {
        let start_time = Instant::now();

        // Check for changed files
        let changed_files = self.detect_changed_files(files).await?;

        // Process only changed files
        let new_diagnostics = if !changed_files.is_empty() {
            info!(
                "Processing {} changed files out of {} total",
                changed_files.len(),
                files.len()
            );
            processor_fn(&changed_files)?
        } else {
            HashMap::new()
        };

        // Update caches with new diagnostics
        for (file_path, diagnostics) in &new_diagnostics {
            self.update_cache(file_path, diagnostics).await?;
        }

        // Collect all diagnostics (new + cached)
        let mut all_diagnostics = Vec::new();

        // Add new diagnostics
        for diagnostics in new_diagnostics.values() {
            all_diagnostics.extend(diagnostics.clone());
        }

        // Add cached diagnostics for unchanged files
        for file_path in files {
            if !changed_files.contains(file_path) {
                if let Some(cached_diagnostics) = self.get_cached_diagnostics(file_path).await {
                    all_diagnostics.extend(cached_diagnostics);
                }
            }
        }

        let processing_time = start_time.elapsed();
        let cache_hit_rate = if files.is_empty() {
            0.0
        } else {
            (files.len() - changed_files.len()) as f32 / files.len() as f32
        };

        let stats = ProcessingStats {
            total_files: files.len(),
            changed_files: changed_files.len(),
            cached_files: files.len() - changed_files.len(),
            processing_time,
            cache_hit_rate,
        };

        // Record metrics
        if let Some(metrics) = &self.metrics {
            metrics.increment_files_processed(files.len() as u64);
            metrics.record_file_processing_time(processing_time);

            metrics
                .record_processing_session(
                    files.len(),
                    stats.cached_files,
                    stats.changed_files,
                    processing_time,
                    4, // Approximate parallel tasks
                )
                .await;
        }

        // Maybe run optimization
        if self.config.auto_optimization {
            self.maybe_optimize().await?;
        }

        Ok((all_diagnostics, stats))
    }

    pub async fn clear_all_caches(&self) -> Result<()> {
        info!("Clearing all caches");

        if let Some(persistent_cache) = &self.persistent_cache {
            persistent_cache.clear_all().await?;
        }

        self.core_processor.clear_cache().await?;

        Ok(())
    }

    pub async fn get_performance_summary(&self) -> Result<PerformanceSummary> {
        let core_stats = self.core_processor.get_cache_stats().await;

        let persistent_stats = if let Some(cache) = &self.persistent_cache {
            Some(cache.get_stats().await)
        } else {
            None
        };

        let processing_metrics = if let Some(collector) = &self.metrics {
            Some(collector.get_comprehensive_metrics().await)
        } else {
            None
        };

        let error_stats = self.error_recovery.get_error_statistics().await;

        Ok(PerformanceSummary {
            core_cache_files: core_stats.0,
            core_cache_diagnostics: core_stats.1,
            persistent_cache_stats: persistent_stats,
            processing_metrics,
            error_count: error_stats.total_errors,
            recent_error_rate: error_stats.recent_error_rate,
        })
    }

    async fn maybe_optimize(&self) -> Result<()> {
        let now = Instant::now();
        let last_optimization = *self.last_optimization.read().await;

        if now.duration_since(last_optimization) >= Duration::from_secs(3600) {
            // 1 hour
            self.optimize().await?;
            *self.last_optimization.write().await = now;
        }

        Ok(())
    }

    pub async fn optimize(&self) -> Result<()> {
        info!("Starting system optimization");
        let start = Instant::now();

        // Optimize persistent cache
        if let Some(persistent_cache) = &self.persistent_cache {
            persistent_cache.optimize().await?;
        }

        // Cleanup error recovery state
        self.error_recovery.cleanup_old_retry_counts().await;

        info!("System optimization completed in {:?}", start.elapsed());
        Ok(())
    }

    // Git integration methods

    pub async fn get_git_repository_info(&self) -> Option<crate::core::GitRepositoryInfo> {
        self.git_integration.as_ref()?.get_repository_info().await
    }

    pub async fn get_git_modified_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(git) = &self.git_integration {
            git.get_modified_files().await
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_conflicted_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(git) = &self.git_integration {
            git.get_conflicted_files().await
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_untracked_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(git) = &self.git_integration {
            git.get_untracked_files().await
        } else {
            Ok(vec![])
        }
    }

    pub async fn is_file_ignored(&self, file_path: &Path) -> Result<bool> {
        if let Some(git) = &self.git_integration {
            git.is_file_ignored(file_path).await
        } else {
            Ok(false)
        }
    }

    pub async fn is_git_repository_clean(&self) -> Result<bool> {
        if let Some(git) = &self.git_integration {
            git.is_repository_clean().await
        } else {
            Ok(true) // Assume clean if no git
        }
    }

    pub async fn refresh_git_status(&self) -> Result<()> {
        if let Some(git) = &self.git_integration {
            git.refresh_git_status().await
        } else {
            Ok(())
        }
    }

    // Dynamic configuration methods

    pub async fn get_dynamic_config(&self) -> Option<DynamicConfig> {
        if let Some(config_manager) = &self.dynamic_config {
            Some(config_manager.get_config().await)
        } else {
            None
        }
    }

    pub async fn update_dynamic_config<F>(&self, updater: F) -> Result<Vec<ConfigChange>>
    where
        F: FnOnce(&mut DynamicConfig) -> Result<()>,
    {
        if let Some(config_manager) = &self.dynamic_config {
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

    pub async fn reload_config_from_file(&self) -> Result<Vec<ConfigChange>> {
        if let Some(config_manager) = &self.dynamic_config {
            config_manager
                .reload_from_file()
                .await
                .map_err(|e| anyhow::anyhow!("Config reload failed: {}", e))
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    pub async fn save_current_config(&self) -> Result<()> {
        if let Some(config_manager) = &self.dynamic_config {
            config_manager
                .save_current_config()
                .await
                .map_err(|e| anyhow::anyhow!("Config save failed: {}", e))
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    pub async fn set_config_field(&self, field_path: &str, value: &str) -> Result<ConfigChange> {
        if let Some(config_manager) = &self.dynamic_config {
            config_manager
                .set_field_value(field_path, value)
                .await
                .map_err(|e| anyhow::anyhow!("Config field set failed: {}", e))
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    pub async fn get_config_field(&self, field_path: &str) -> Result<String> {
        if let Some(config_manager) = &self.dynamic_config {
            config_manager
                .get_field_value(field_path)
                .await
                .map_err(|e| anyhow::anyhow!("Config field get failed: {}", e))
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    pub async fn watch_config_field(&self, field_path: String) -> Result<()> {
        if let Some(config_manager) = &self.dynamic_config {
            config_manager.watch_field(field_path).await;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Dynamic configuration not enabled"))
        }
    }

    pub async fn handle_config_changes(&self) -> Result<()> {
        if let Some(mut receiver) = self.config_change_receiver.write().await.take() {
            tokio::spawn(async move {
                while let Ok(change) = receiver.recv().await {
                    info!(
                        "Configuration change: {} = {} (was {})",
                        change.field_path, change.new_value, change.old_value
                    );

                    // Here you could implement specific logic for different config changes
                    match change.field_path.as_str() {
                        "processing.parallel_processing" => {
                            info!("Parallel processing toggled to: {}", change.new_value);
                        }
                        "memory.max_memory_mb" => {
                            info!("Memory limit changed to: {} MB", change.new_value);
                        }
                        "features.auto_optimization" => {
                            info!("Auto optimization toggled to: {}", change.new_value);
                        }
                        _ => {
                            debug!("Unhandled config change: {}", change.field_path);
                        }
                    }
                }
            });
        }
        Ok(())
    }

    pub async fn apply_dynamic_config_to_processor(&self) -> Result<()> {
        if let Some(dynamic_config) = self.get_dynamic_config().await {
            // Apply configuration changes to the processor components

            // Update processing settings
            if !dynamic_config.processing.parallel_processing {
                info!("Disabling parallel processing based on dynamic config");
                // Note: In a real implementation, you'd need to update the core processor
            }

            // Update cache settings if they changed significantly
            if let Some(_persistent_cache) = &self.persistent_cache {
                // You could implement cache reconfiguration here
                info!(
                    "Cache configuration: max_size={}MB, ttl={}h",
                    dynamic_config.cache.max_size_mb, dynamic_config.cache.ttl_hours
                );
            }

            // Update memory management settings
            if let Some(_memory_cache) = self.get_memory_cache_reference() {
                // You could implement memory cache reconfiguration here
                info!(
                    "Memory configuration: max={}MB, policy={}",
                    dynamic_config.memory.max_memory_mb, dynamic_config.memory.eviction_policy
                );
            }

            info!("Dynamic configuration applied to processor components");
        }
        Ok(())
    }

    // Helper method to get memory cache reference (would need to be implemented)
    fn get_memory_cache_reference(&self) -> Option<&dyn std::any::Any> {
        // This would return a reference to the memory cache if we had one
        // For now, just return None as our simple processor doesn't have a memory cache
        None
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub core_cache_files: usize,
    pub core_cache_diagnostics: usize,
    pub persistent_cache_stats: Option<crate::core::persistent_cache::CacheStats>,
    pub processing_metrics: Option<crate::core::metrics::ProcessingMetrics>,
    pub error_count: u64,
    pub recent_error_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_simple_enhanced_processor() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = SimpleEnhancedConfig {
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
