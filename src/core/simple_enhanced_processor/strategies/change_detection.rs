//! Change detection strategy

use crate::core::{GitIntegration, IncrementalProcessor, MetricsCollector};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Strategy for detecting file changes
pub struct ChangeDetectionStrategy {
    core_processor: Arc<IncrementalProcessor>,
    git_integration: Option<Arc<GitIntegration>>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl ChangeDetectionStrategy {
    /// Create a new change detection strategy
    pub fn new(
        core_processor: Arc<IncrementalProcessor>,
        git_integration: Option<Arc<GitIntegration>>,
        metrics: Option<Arc<MetricsCollector>>,
    ) -> Self {
        Self {
            core_processor,
            git_integration,
            metrics,
        }
    }

    /// Detect changed files
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
}