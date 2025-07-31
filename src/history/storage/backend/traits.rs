use crate::core::errors::DatabaseError;
use crate::history::storage::types::*;
use async_trait::async_trait;
use std::path::Path;
use std::time::SystemTime;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend with the given configuration
    async fn initialize(&mut self, config: &HistoryConfig) -> Result<(), DatabaseError>;

    /// Record a new diagnostic snapshot
    async fn record_snapshot(
        &self,
        snapshot: DiagnosticSnapshot,
    ) -> Result<i64, DatabaseError>;

    /// Get snapshots for a specific file
    async fn get_snapshots_for_file(
        &self,
        file_path: &Path,
        since: Option<SystemTime>,
        limit: Option<usize>,
    ) -> Result<Vec<DiagnosticSnapshot>, DatabaseError>;

    /// Get historical statistics for a file
    async fn get_file_history_stats(
        &self,
        file_path: &Path,
    ) -> Result<Option<FileHistoryStats>, DatabaseError>;

    /// Get recurring error patterns
    async fn get_recurring_patterns(
        &self,
        min_occurrences: usize,
    ) -> Result<Vec<HistoricalErrorPattern>, DatabaseError>;

    /// Get time series data for visualization
    async fn get_time_series_data(
        &self,
        start: SystemTime,
        end: SystemTime,
        interval: Duration,
    ) -> Result<Vec<TimeSeriesPoint>, DatabaseError>;

    /// Clean up old data based on retention policy
    async fn cleanup_old_data(&self, retention_days: u64) -> Result<usize, DatabaseError>;

    /// Export data in ML-ready format
    async fn export_ml_ready_data(&self, output_path: &Path) -> Result<(), DatabaseError>;

    /// Check if cleanup is needed
    async fn should_cleanup(&self) -> bool;

    /// Update the last cleanup time
    async fn update_last_cleanup(&self) -> Result<(), DatabaseError>;
}

use std::time::Duration;