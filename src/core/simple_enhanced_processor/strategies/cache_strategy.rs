//! Cache management strategy

use crate::core::{
    Diagnostic, FileHash, IncrementalProcessor, MetricsCollector, PersistentCache,
    persistent_cache::CacheEntry,
};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tracing::{debug, info};

/// Strategy for managing caching operations
pub struct CacheStrategy {
    core_processor: Arc<IncrementalProcessor>,
    persistent_cache: Option<Arc<PersistentCache>>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl CacheStrategy {
    /// Create a new cache strategy
    pub fn new(
        core_processor: Arc<IncrementalProcessor>,
        persistent_cache: Option<Arc<PersistentCache>>,
        metrics: Option<Arc<MetricsCollector>>,
    ) -> Self {
        Self {
            core_processor,
            persistent_cache,
            metrics,
        }
    }

    /// Get cached diagnostics for a file
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

    /// Update cache with new diagnostics
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

            let entry = CacheEntry {
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

    /// Clear all caches
    pub async fn clear_all_caches(&self) -> Result<()> {
        info!("Clearing all caches");

        if let Some(persistent_cache) = &self.persistent_cache {
            persistent_cache.clear_all().await?;
        }

        self.core_processor.clear_cache().await?;

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize, Option<crate::core::persistent_cache::CacheStats>) {
        let core_stats = self.core_processor.get_cache_stats().await;
        
        let persistent_stats = if let Some(cache) = &self.persistent_cache {
            Some(cache.get_stats().await)
        } else {
            None
        };

        (core_stats.0, core_stats.1, persistent_stats)
    }
}