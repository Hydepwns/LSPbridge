//! Processing pipeline for the enhanced processor

use crate::core::{Diagnostic, MetricsCollector, ProcessingStats};
use crate::core::simple_enhanced_processor::strategies::{CacheStrategy, ChangeDetectionStrategy};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Processing pipeline that orchestrates the flow
pub struct ProcessingPipeline {
    cache_strategy: Arc<CacheStrategy>,
    change_detection: Arc<ChangeDetectionStrategy>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl ProcessingPipeline {
    /// Create a new processing pipeline
    pub fn new(
        cache_strategy: Arc<CacheStrategy>,
        change_detection: Arc<ChangeDetectionStrategy>,
        metrics: Option<Arc<MetricsCollector>>,
    ) -> Self {
        Self {
            cache_strategy,
            change_detection,
            metrics,
        }
    }

    /// Process files incrementally
    pub async fn process_files_incrementally(
        &self,
        files: &[PathBuf],
        processor_fn: impl Fn(&[PathBuf]) -> Result<HashMap<PathBuf, Vec<Diagnostic>>>,
    ) -> Result<(Vec<Diagnostic>, ProcessingStats)> {
        let start_time = Instant::now();

        // Check for changed files
        let changed_files = self.change_detection.detect_changed_files(files).await?;

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
            self.cache_strategy.update_cache(file_path, diagnostics).await?;
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
                if let Some(cached_diagnostics) = self.cache_strategy.get_cached_diagnostics(file_path).await {
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

        Ok((all_diagnostics, stats))
    }
}