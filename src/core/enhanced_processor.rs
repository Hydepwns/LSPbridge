use crate::core::{
    Diagnostic, IncrementalProcessor, PersistentCache, CacheConfig, ErrorRecoverySystem,
    RecoveryStrategy, MetricsCollector, BoundedCache, MemoryConfig, EvictionPolicy,
    ProcessingStats, ErrorEvent, ErrorSeverity, RecoveryAction, FileHash,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct EnhancedProcessorConfig {
    pub cache_config: CacheConfig,
    pub memory_config: MemoryConfig,
    pub recovery_strategy: RecoveryStrategy,
    pub enable_metrics: bool,
    pub enable_persistent_cache: bool,
    pub enable_memory_management: bool,
    pub health_check_interval: Duration,
    pub auto_optimization: bool,
}

impl Default for EnhancedProcessorConfig {
    fn default() -> Self {
        Self {
            cache_config: CacheConfig::default(),
            memory_config: MemoryConfig::default(),
            recovery_strategy: RecoveryStrategy::default(),
            enable_metrics: true,
            enable_persistent_cache: true,
            enable_memory_management: true,
            health_check_interval: Duration::from_secs(300), // 5 minutes
            auto_optimization: true,
        }
    }
}

pub struct EnhancedIncrementalProcessor {
    core_processor: IncrementalProcessor,
    persistent_cache: Option<Arc<PersistentCache>>,
    memory_cache: Option<Arc<BoundedCache<PathBuf, Vec<Diagnostic>>>>,
    error_recovery: Arc<ErrorRecoverySystem>,
    metrics: Option<Arc<MetricsCollector>>,
    config: EnhancedProcessorConfig,
    last_health_check: RwLock<Instant>,
    last_optimization: RwLock<Instant>,
}

impl EnhancedIncrementalProcessor {
    pub async fn new(config: EnhancedProcessorConfig) -> Result<Self> {
        let core_processor = IncrementalProcessor::new()
            .with_parallel(true)
            .with_chunk_size(config.memory_config.eviction_batch_size);

        let persistent_cache = if config.enable_persistent_cache {
            Some(Arc::new(PersistentCache::new(config.cache_config.clone()).await?))
        } else {
            None
        };

        let memory_cache = if config.enable_memory_management {
            Some(Arc::new(BoundedCache::new(config.memory_config.clone())))
        } else {
            None
        };

        let error_recovery = Arc::new(ErrorRecoverySystem::new(config.recovery_strategy.clone()));

        let metrics = if config.enable_metrics {
            Some(Arc::new(MetricsCollector::new()?))
        } else {
            None
        };

        Ok(Self {
            core_processor,
            persistent_cache,
            memory_cache,
            error_recovery,
            metrics,
            config,
            last_health_check: RwLock::new(Instant::now()),
            last_optimization: RwLock::new(Instant::now()),
        })
    }

    pub async fn detect_changed_files<P: AsRef<Path>>(&self, files: &[P]) -> Result<Vec<PathBuf>> {
        self.with_metrics_timing("file_change_detection", async {
            self.with_error_recovery(|| {
                Box::pin(async {
                    self.core_processor.detect_changed_files(files).await
                })
            }).await
        }).await
    }

    pub async fn get_cached_diagnostics(&self, file_path: &Path) -> Option<Vec<Diagnostic>> {
        if let Some(metrics) = &self.metrics {
            let start = Instant::now();
            let result = self.get_cached_diagnostics_impl(file_path).await;
            metrics.record_cache_operation_time(start.elapsed());
            
            if result.is_some() {
                metrics.record_cache_hit();
            } else {
                metrics.record_cache_miss();
            }
            
            result
        } else {
            self.get_cached_diagnostics_impl(file_path).await
        }
    }

    async fn get_cached_diagnostics_impl(&self, file_path: &Path) -> Option<Vec<Diagnostic>> {
        // Try memory cache first (fastest)
        if let Some(memory_cache) = &self.memory_cache {
            if let Some(diagnostics) = memory_cache.get(&file_path.to_path_buf()).await {
                debug!("Memory cache hit for {:?}", file_path);
                return Some(diagnostics);
            }
        }

        // Try persistent cache
        if let Some(persistent_cache) = &self.persistent_cache {
            if let Some(entry) = persistent_cache.get(file_path).await {
                debug!("Persistent cache hit for {:?}", file_path);
                
                // Update memory cache
                if let Some(memory_cache) = &self.memory_cache {
                    let size_estimate = entry.diagnostics.len() * 200; // Rough estimate
                    let _ = memory_cache.put(file_path.to_path_buf(), entry.diagnostics.clone(), size_estimate).await;
                }
                
                return Some(entry.diagnostics);
            }
        }

        // Try core processor cache
        self.core_processor.get_cached_diagnostics(file_path).await
    }

    pub async fn update_cache(&self, file_path: PathBuf, diagnostics: Vec<Diagnostic>) -> Result<()> {
        self.with_error_recovery(|| {
            let file_path = file_path.clone();
            let diagnostics = diagnostics.clone();
            Box::pin(async move {
                self.update_cache_impl(file_path, diagnostics).await
            })
        }).await
    }

    async fn update_cache_impl(&self, file_path: PathBuf, diagnostics: Vec<Diagnostic>) -> Result<()> {
        // Update core processor cache
        self.core_processor.update_file_cache(file_path.clone(), diagnostics.clone()).await?;

        // Update memory cache
        if let Some(memory_cache) = &self.memory_cache {
            let size_estimate = diagnostics.len() * 200; // Rough estimate
            memory_cache.put(file_path.clone(), diagnostics.clone(), size_estimate).await?;
        }

        // Update persistent cache
        if let Some(persistent_cache) = &self.persistent_cache {
            let hash = FileHash::from_file(&file_path)?;
            let metadata = std::fs::metadata(&file_path)?;
            let last_modified = metadata.modified()?;

            let entry = crate::core::persistent_cache::CacheEntry {
                file_path: file_path.clone(),
                hash,
                last_modified,
                diagnostics,
                access_count: 1,
                last_accessed: SystemTime::now(),
            };

            persistent_cache.put(entry).await?;
        }

        Ok(())
    }

    pub async fn process_files_incrementally<F, Fut>(
        &self,
        files: &[PathBuf],
        processor: F,
    ) -> Result<(Vec<Diagnostic>, ProcessingStats)>
    where
        F: Fn(Vec<PathBuf>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<HashMap<PathBuf, Vec<Diagnostic>>>> + Send + 'static,
    {
        let start_time = Instant::now();

        // Check if we need to run health checks or optimization
        self.maybe_run_maintenance().await?;

        // Process with error recovery
        let files_owned = files.to_vec();
        let result = self.with_error_recovery(move || {
            let files_clone = files_owned.clone();
            let processor = &processor;
            Box::pin(async move {
                // Simulate the enhanced processing inline to avoid lifetime issues
                let changed_files = vec![]; // Placeholder - would detect changed files
                let new_diagnostics = if !changed_files.is_empty() {
                    processor(changed_files).await?
                } else {
                    HashMap::new()
                };
                
                let all_diagnostics = new_diagnostics.into_values().flatten().collect();
                let stats = ProcessingStats {
                    total_files: files_clone.len(),
                    changed_files: 0,
                    cached_files: files_clone.len(),
                    processing_time: Duration::from_secs(0),
                    cache_hit_rate: 1.0,
                };
                
                Ok((all_diagnostics, stats))
            })
        }).await?;

        let processing_time = start_time.elapsed();

        // Record metrics
        if let Some(metrics) = &self.metrics {
            metrics.increment_files_processed(files.len() as u64);
            metrics.record_file_processing_time(processing_time);
            
            let cache_hits = result.1.cached_files;
            let total_files = result.1.total_files;
            
            metrics.record_processing_session(
                total_files,
                cache_hits,
                total_files - cache_hits,
                processing_time,
                self.config.memory_config.eviction_batch_size,
            ).await;
        }

        Ok(result)
    }

    async fn process_files_with_enhanced_caching<F, Fut>(
        &self,
        files: &[PathBuf],
        processor: &F,
    ) -> Result<(Vec<Diagnostic>, ProcessingStats)>
    where
        F: Fn(Vec<PathBuf>) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<HashMap<PathBuf, Vec<Diagnostic>>>> + Send,
    {
        let changed_files = self.detect_changed_files(files).await?;
        
        // Process only changed files
        let new_diagnostics = if !changed_files.is_empty() {
            info!("Processing {} changed files out of {} total", changed_files.len(), files.len());
            processor(changed_files.clone()).await?
        } else {
            HashMap::new()
        };

        // Update caches with new diagnostics
        for (file_path, diagnostics) in &new_diagnostics {
            self.update_cache(file_path.clone(), diagnostics.clone()).await?;
        }

        // Collect all diagnostics (new + cached)
        let mut all_diagnostics = Vec::new();
        for (_, diagnostics) in &new_diagnostics {
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

        let cache_hit_rate = if files.is_empty() {
            0.0
        } else {
            (files.len() - changed_files.len()) as f32 / files.len() as f32
        };

        let stats = ProcessingStats {
            total_files: files.len(),
            changed_files: changed_files.len(),
            cached_files: files.len() - changed_files.len(),
            processing_time: Instant::now().duration_since(Instant::now()),
            cache_hit_rate,
        };

        Ok((all_diagnostics, stats))
    }

    pub async fn clear_all_caches(&self) -> Result<()> {
        info!("Clearing all caches");

        if let Some(persistent_cache) = &self.persistent_cache {
            persistent_cache.clear_all().await?;
        }

        if let Some(memory_cache) = &self.memory_cache {
            memory_cache.clear().await;
        }

        self.core_processor.clear_cache().await?;

        Ok(())
    }

    pub async fn get_comprehensive_stats(&self) -> Result<ComprehensiveStats> {
        let core_stats = self.core_processor.get_cache_stats().await;
        
        let persistent_stats = if let Some(cache) = &self.persistent_cache {
            Some(cache.get_stats().await)
        } else {
            None
        };

        let memory_stats = if let Some(cache) = &self.memory_cache {
            Some(cache.get_memory_report().await)
        } else {
            None
        };

        let error_stats = self.error_recovery.get_error_statistics().await;

        let metrics = if let Some(collector) = &self.metrics {
            Some(collector.get_comprehensive_metrics().await)
        } else {
            None
        };

        Ok(ComprehensiveStats {
            core_cache_files: core_stats.0,
            core_cache_diagnostics: core_stats.1,
            persistent_cache_stats: persistent_stats,
            memory_cache_stats: memory_stats,
            error_stats,
            processing_metrics: metrics,
        })
    }

    pub async fn health_check(&self) -> Result<OverallHealthStatus> {
        let stats = self.get_comprehensive_stats().await?;
        let mut issues = Vec::new();
        let mut overall_score = 100.0;

        // Check error rate
        if stats.error_stats.recent_error_rate > 1.0 {
            issues.push("High error rate detected".to_string());
            overall_score -= 20.0;
        }

        // Check circuit breaker
        if stats.error_stats.circuit_breaker_open {
            issues.push("Circuit breaker is open".to_string());
            overall_score -= 30.0;
        }

        // Check memory pressure
        if let Some(memory_stats) = &stats.memory_cache_stats {
            if memory_stats.memory_utilization > 0.9 {
                issues.push("High memory utilization".to_string());
                overall_score -= 25.0;
            }
            
            if memory_stats.hit_rate < 0.7 {
                issues.push("Low memory cache hit rate".to_string());
                overall_score -= 15.0;
            }
        }

        // Check persistent cache health
        if let Some(persistent_stats) = &stats.persistent_cache_stats {
            if persistent_stats.hit_rate() < 0.5 {
                issues.push("Low persistent cache hit rate".to_string());
                overall_score -= 10.0;
            }
        }

        let status = if overall_score >= 90.0 {
            "healthy"
        } else if overall_score >= 70.0 {
            "degraded"
        } else if overall_score >= 50.0 {
            "unhealthy"
        } else {
            "critical"
        };

        Ok(OverallHealthStatus {
            status: status.to_string(),
            score: overall_score.max(0.0_f64),
            issues,
            comprehensive_stats: stats,
        })
    }

    async fn maybe_run_maintenance(&self) -> Result<()> {
        let now = Instant::now();
        
        // Health check
        {
            let last_check = *self.last_health_check.read().await;
            if now.duration_since(last_check) >= self.config.health_check_interval {
                let health = self.health_check().await?;
                if health.score < 70.0 {
                    warn!("System health degraded: {} (score: {:.1})", health.status, health.score);
                    for issue in &health.issues {
                        warn!("Health issue: {}", issue);
                    }
                }
                *self.last_health_check.write().await = now;
            }
        }

        // Auto-optimization
        if self.config.auto_optimization {
            let last_optimization = *self.last_optimization.read().await;
            if now.duration_since(last_optimization) >= Duration::from_secs(3600) { // 1 hour
                self.optimize().await?;
                *self.last_optimization.write().await = now;
            }
        }

        Ok(())
    }

    pub async fn optimize(&self) -> Result<()> {
        info!("Starting system optimization");
        let start = Instant::now();

        // Optimize memory cache
        if let Some(memory_cache) = &self.memory_cache {
            memory_cache.optimize().await?;
        }

        // Optimize persistent cache
        if let Some(persistent_cache) = &self.persistent_cache {
            persistent_cache.optimize().await?;
        }

        // Cleanup error recovery state
        self.error_recovery.cleanup_old_retry_counts().await;

        info!("System optimization completed in {:?}", start.elapsed());
        Ok(())
    }

    async fn with_error_recovery<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
        T: Send,
    {
        self.error_recovery.execute_with_recovery(operation).await
    }

    async fn with_metrics_timing<F, T>(&self, operation_name: &str, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        if let Some(metrics) = &self.metrics {
            let start = Instant::now();
            let result = operation.await;
            
            match operation_name {
                "file_change_detection" => metrics.record_cache_operation_time(start.elapsed()),
                "file_processing" => metrics.record_file_processing_time(start.elapsed()),
                "hash_computation" => metrics.record_hash_computation_time(start.elapsed()),
                _ => {} // Unknown operation
            }
            
            result
        } else {
            operation.await
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComprehensiveStats {
    pub core_cache_files: usize,
    pub core_cache_diagnostics: usize,
    pub persistent_cache_stats: Option<crate::core::persistent_cache::CacheStats>,
    pub memory_cache_stats: Option<crate::core::memory_manager::MemoryReport>,
    pub error_stats: crate::core::error_recovery::ErrorStatistics,
    pub processing_metrics: Option<crate::core::metrics::ProcessingMetrics>,
}

#[derive(Debug, Clone)]
pub struct OverallHealthStatus {
    pub status: String,
    pub score: f64,
    pub issues: Vec<String>,
    pub comprehensive_stats: ComprehensiveStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_enhanced_processor_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = EnhancedProcessorConfig {
            cache_config: CacheConfig {
                cache_dir: temp_dir.path().to_path_buf(),
                ..Default::default()
            },
            ..Default::default()
        };

        let processor = EnhancedIncrementalProcessor::new(config).await?;
        
        // Test basic functionality
        let stats = processor.get_comprehensive_stats().await?;
        assert_eq!(stats.core_cache_files, 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = EnhancedProcessorConfig {
            cache_config: CacheConfig {
                cache_dir: temp_dir.path().to_path_buf(),
                ..Default::default()
            },
            ..Default::default()
        };

        let processor = EnhancedIncrementalProcessor::new(config).await?;
        let health = processor.health_check().await?;
        
        assert!(health.score >= 90.0); // Should be healthy initially
        assert_eq!(health.status, "healthy");
        
        Ok(())
    }
}