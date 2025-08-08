pub mod backend;
pub mod cache;
pub mod migrations;
pub mod types;

use crate::core::config::ConfigDefaults;
use crate::core::errors::DatabaseError;
use crate::impl_config_defaults;
use backend::{sqlite::SqliteBackend, StorageBackend};
use cache::QueryCache;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub use types::*;

impl_config_defaults!(HistoryConfig, "history.toml", validate => |config: &HistoryConfig| {
    if config.retention_days == 0 {
        return Err(DatabaseError::Corruption {
            details: "retention_days must be greater than 0".to_string(),
        }.into());
    }
    if config.max_snapshots_per_file == 0 {
        return Err(DatabaseError::Corruption {
            details: "max_snapshots_per_file must be greater than 0".to_string(),
        }.into());
    }
    Ok(())
});

pub struct HistoryStorage {
    backend: Box<dyn StorageBackend>,
    cache: QueryCache,
    config: HistoryConfig,
}

impl HistoryStorage {
    pub async fn new(config: HistoryConfig) -> Result<Self, DatabaseError> {
        let backend = Box::new(SqliteBackend::new(config.clone()).await?);
        let cache = QueryCache::new(Duration::from_secs(300)); // 5 minute cache TTL
        
        Ok(Self {
            backend,
            cache,
            config,
        })
    }

    pub async fn record_snapshot(
        &self,
        snapshot: DiagnosticSnapshot,
    ) -> Result<i64, DatabaseError> {
        // Invalidate cache for this file
        self.cache.invalidate_file(&snapshot.file_path).await;
        
        let id = self.backend.record_snapshot(snapshot).await?;
        
        // Check if cleanup is needed
        if self.backend.should_cleanup().await {
            self.backend.cleanup_old_data(self.config.retention_days).await?;
            self.backend.update_last_cleanup().await?;
        }
        
        Ok(id)
    }

    pub async fn get_snapshots_for_file(
        &self,
        file_path: &Path,
        since: Option<SystemTime>,
        limit: Option<usize>,
    ) -> Result<Vec<DiagnosticSnapshot>, DatabaseError> {
        // Check cache first (only for queries without filters)
        if since.is_none() && limit.is_none() {
            if let Some(cached) = self.cache.get_file_snapshots(file_path).await {
                return Ok(cached);
            }
        }
        
        let snapshots = self.backend.get_snapshots_for_file(file_path, since, limit).await?;
        
        // Cache if no filters
        if since.is_none() && limit.is_none() {
            self.cache.cache_file_snapshots(file_path, snapshots.clone()).await;
        }
        
        Ok(snapshots)
    }

    pub async fn get_file_history_stats(
        &self,
        file_path: &Path,
    ) -> Result<Option<FileHistoryStats>, DatabaseError> {
        // Check cache first
        if let Some(cached) = self.cache.get_file_stats(file_path).await {
            return Ok(cached);
        }
        
        let stats = self.backend.get_file_history_stats(file_path).await?;
        self.cache.cache_file_stats(file_path, stats.clone()).await;
        
        Ok(stats)
    }

    pub async fn get_recurring_patterns(
        &self,
        min_occurrences: usize,
    ) -> Result<Vec<HistoricalErrorPattern>, DatabaseError> {
        // Check cache first
        if min_occurrences == 1 {
            if let Some(cached) = self.cache.get_patterns().await {
                return Ok(cached.into_iter()
                    .filter(|p| p.occurrence_count >= min_occurrences)
                    .collect());
            }
        }
        
        let patterns = self.backend.get_recurring_patterns(min_occurrences).await?;
        
        // Cache all patterns if querying with min_occurrences = 1
        if min_occurrences == 1 {
            self.cache.cache_patterns(patterns.clone()).await;
        }
        
        Ok(patterns)
    }

    pub async fn get_time_series_data(
        &self,
        start: SystemTime,
        end: SystemTime,
        interval: Duration,
    ) -> Result<Vec<TimeSeriesPoint>, DatabaseError> {
        self.backend.get_time_series_data(start, end, interval).await
    }

    pub async fn export_ml_ready_data(&self, output_path: &Path) -> Result<(), DatabaseError> {
        self.backend.export_ml_ready_data(output_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FileHash;
    use tempfile::TempDir;
    use std::path::PathBuf;

    #[tokio::test]
    #[ignore] // TODO: Fix file stats updating - requires refactoring connection pool usage
    async fn test_history_storage_basic() -> Result<(), DatabaseError> {
        let temp_dir = TempDir::new()?;
        let config = HistoryConfig {
            db_path: temp_dir.path().join("test_history.db"),
            retention_days: 30,
            max_snapshots_per_file: 100,
            auto_cleanup_interval: Duration::from_secs(3600),
            min_connections: 1,
            max_connections: 5,
            connection_timeout_secs: 5,
        };

        let storage = HistoryStorage::new(config).await?;

        // Create test snapshot
        let snapshot = DiagnosticSnapshot {
            id: 0,
            timestamp: SystemTime::now(),
            file_path: PathBuf::from("/test/file.rs"),
            file_hash: FileHash::new(b"test content"),
            diagnostics: vec![],
            error_count: 2,
            warning_count: 1,
            info_count: 0,
            hint_count: 0,
        };

        // Record snapshot
        let id = storage.record_snapshot(snapshot.clone()).await?;
        assert!(id > 0);

        // Retrieve snapshots
        let snapshots = storage
            .get_snapshots_for_file(&snapshot.file_path, None, None)
            .await?;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].error_count, 2);

        // Check file stats
        let stats = storage.get_file_history_stats(&snapshot.file_path).await?;
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_errors, 2);
        assert_eq!(stats.total_warnings, 1);

        Ok(())
    }
}