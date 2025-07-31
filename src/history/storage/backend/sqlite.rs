use super::traits::StorageBackend;
use crate::core::errors::DatabaseError;
use crate::core::{DatabasePool, DatabasePoolBuilder, FileHash};
use crate::history::storage::types::*;
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

pub struct SqliteBackend {
    pool: Arc<DatabasePool>,
    config: HistoryConfig,
    last_cleanup: tokio::sync::RwLock<SystemTime>,
}

impl SqliteBackend {
    pub async fn new(config: HistoryConfig) -> Result<Self, DatabaseError> {
        let pool = DatabasePoolBuilder::new(&config.db_path)
            .min_connections(config.min_connections)
            .max_connections(config.max_connections)
            .connection_timeout(Duration::from_secs(config.connection_timeout_secs))
            .enable_wal(true)
            .build()
            .await
            .map_err(|e| DatabaseError::Sqlite {
                operation: "create_pool".to_string(),
                message: format!("Failed to create connection pool: {}", e),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(e.to_string()),
                ),
            })?;

        let mut backend = Self {
            pool,
            config: config.clone(),
            last_cleanup: tokio::sync::RwLock::new(SystemTime::now()),
        };

        backend.initialize(&config).await?;
        Ok(backend)
    }

    pub(crate) fn init_schema(conn: &mut Connection) -> anyhow::Result<()> {
        conn.execute_batch(include_str!("../migrations/v1_initial.sql"))?;
        
        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
            params!["schema_version", "1.0"],
        )?;
        
        Ok(())
    }

    fn convert_timestamp_to_secs(time: SystemTime) -> Result<i64, DatabaseError> {
        time.duration_since(UNIX_EPOCH)
            .map_err(|e| DatabaseError::Serialization {
                data_type: "SystemTime".to_string(),
                reason: format!("Invalid timestamp: {}", e),
                source: bincode::ErrorKind::Custom(format!("timestamp error: {}", e)).into(),
            })
            .map(|d| d.as_secs() as i64)
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn initialize(&mut self, _config: &HistoryConfig) -> Result<(), DatabaseError> {
        self.pool.with_connection(|conn| Self::init_schema(conn))
            .await
            .map_err(|e| DatabaseError::Sqlite {
                operation: "init_schema".to_string(),
                message: format!("Failed to initialize schema: {}", e),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(e.to_string()),
                ),
            })?;

        info!("SQLite storage backend initialized at {:?}", self.config.db_path);
        Ok(())
    }

    async fn record_snapshot(
        &self,
        snapshot: DiagnosticSnapshot,
    ) -> Result<i64, DatabaseError> {
        let timestamp = Self::convert_timestamp_to_secs(snapshot.timestamp)?;
        let created_at = Self::convert_timestamp_to_secs(SystemTime::now())?;
        let diagnostics_json = serde_json::to_string(&snapshot.diagnostics)?;
        let file_path_for_log = snapshot.file_path.clone();
        
        let id = self.pool.with_connection(move |conn| {
            let id: i64 = conn.query_row(
                r#"
                INSERT INTO diagnostic_snapshots 
                (timestamp, file_path, file_hash, error_count, warning_count, 
                 info_count, hint_count, diagnostics_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING id
                "#,
                params![
                    timestamp,
                    snapshot.file_path.to_string_lossy(),
                    format!("{:?}", snapshot.file_hash),
                    snapshot.error_count,
                    snapshot.warning_count,
                    snapshot.info_count,
                    snapshot.hint_count,
                    diagnostics_json,
                    created_at
                ],
                |row| row.get(0),
            )?;
            Ok(id)
        }).await
        .map_err(|e| DatabaseError::Sqlite {
            operation: "record_snapshot".to_string(),
            message: format!("Failed to record snapshot: {}", e),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        debug!("Recorded snapshot {} for {:?}", id, file_path_for_log);
        Ok(id)
    }

    async fn get_snapshots_for_file(
        &self,
        file_path: &Path,
        since: Option<SystemTime>,
        limit: Option<usize>,
    ) -> Result<Vec<DiagnosticSnapshot>, DatabaseError> {
        let file_path_str = file_path.to_string_lossy().to_string();
        
        let since_ts = if let Some(since_time) = since {
            Some(Self::convert_timestamp_to_secs(since_time)?)
        } else {
            None
        };
        
        let snapshots = self.pool.with_read_connection(move |conn| {
            let mut query = String::from(
                "SELECT id, timestamp, file_path, file_hash, error_count, warning_count, 
                 info_count, hint_count, diagnostics_json 
                 FROM diagnostic_snapshots 
                 WHERE file_path = ?",
            );

            if let Some(since_timestamp) = since_ts {
                query.push_str(&format!(" AND timestamp >= {}", since_timestamp));
            }

            query.push_str(" ORDER BY timestamp DESC");

            if let Some(limit_value) = limit {
                query.push_str(&format!(" LIMIT {}", limit_value));
            }

            let mut stmt = conn.prepare(&query)?;
            let snapshots = stmt
                .query_map([&file_path_str], |row| {
                    let timestamp_secs: i64 = row.get(1)?;
                    let diagnostics_json: String = row.get(8)?;

                    Ok(DiagnosticSnapshot {
                        id: row.get(0)?,
                        timestamp: UNIX_EPOCH + Duration::from_secs(timestamp_secs as u64),
                        file_path: PathBuf::from(row.get::<_, String>(2)?),
                        file_hash: FileHash::new(row.get::<_, String>(3)?.as_bytes()),
                        diagnostics: serde_json::from_str(&diagnostics_json).unwrap_or_default(),
                        error_count: row.get(4)?,
                        warning_count: row.get(5)?,
                        info_count: row.get(6)?,
                        hint_count: row.get(7)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            Ok(snapshots)
        }).await
        .map_err(|e| DatabaseError::Sqlite {
            operation: "get_snapshots_for_file".to_string(),
            message: format!("Failed to get snapshots: {}", e),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        Ok(snapshots)
    }

    async fn get_file_history_stats(
        &self,
        file_path: &Path,
    ) -> Result<Option<FileHistoryStats>, DatabaseError> {
        let file_path_str = file_path.to_string_lossy().to_string();
        
        let stats = self.pool.with_read_connection(move |conn| {
            Ok(conn.query_row(
                "SELECT first_seen, last_seen, total_snapshots, total_errors, total_warnings,
             avg_error_count, avg_warning_count, max_error_count, max_warning_count
             FROM file_stats WHERE file_path = ?",
                [file_path_str.clone()],
                |row| {
                    let first_seen_secs: i64 = row.get(0)?;
                    let last_seen_secs: i64 = row.get(1)?;

                    Ok(FileHistoryStats {
                        file_path: PathBuf::from(&file_path_str),
                        first_seen: UNIX_EPOCH + Duration::from_secs(first_seen_secs as u64),
                        last_seen: UNIX_EPOCH + Duration::from_secs(last_seen_secs as u64),
                        total_snapshots: row.get(2)?,
                        total_errors: row.get(3)?,
                        total_warnings: row.get(4)?,
                        avg_error_count: row.get(5)?,
                        avg_warning_count: row.get(6)?,
                        max_error_count: row.get(7)?,
                        max_warning_count: row.get(8)?,
                    })
                },
            )
            .optional()?)
        }).await.map_err(|e| DatabaseError::Sqlite {
            operation: "get_file_history_stats".to_string(),
            message: e.to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        Ok(stats)
    }

    async fn get_recurring_patterns(
        &self,
        min_occurrences: usize,
    ) -> Result<Vec<HistoricalErrorPattern>, DatabaseError> {
        let patterns = self.pool.with_read_connection(move |conn| {
            let mut stmt = conn.prepare(
            "SELECT pattern_hash, first_seen, last_seen, occurrence_count, 
             files_affected, error_message, error_code, source
             FROM error_patterns
             WHERE occurrence_count >= ?
             ORDER BY occurrence_count DESC",
        )?;

        let patterns = stmt
            .query_map([min_occurrences], |row| {
                let first_seen_secs: i64 = row.get(1)?;
                let last_seen_secs: i64 = row.get(2)?;

                Ok(HistoricalErrorPattern {
                    pattern_hash: row.get(0)?,
                    first_seen: UNIX_EPOCH + Duration::from_secs(first_seen_secs as u64),
                    last_seen: UNIX_EPOCH + Duration::from_secs(last_seen_secs as u64),
                    occurrence_count: row.get(3)?,
                    files_affected: row.get(4)?,
                    error_message: row.get(5)?,
                    error_code: row.get(6)?,
                    source: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
            
            Ok(patterns)
        }).await.map_err(|e| DatabaseError::Sqlite {
            operation: "get_recurring_patterns".to_string(),
            message: e.to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        Ok(patterns)
    }

    async fn get_time_series_data(
        &self,
        start: SystemTime,
        end: SystemTime,
        interval: Duration,
    ) -> Result<Vec<TimeSeriesPoint>, DatabaseError> {
        let start_ts = Self::convert_timestamp_to_secs(start)?;
        let end_ts = Self::convert_timestamp_to_secs(end)?;
        let interval_secs = interval.as_secs() as i64;

        let query = format!(
            r#"
            SELECT 
                (timestamp / {}) * {} as time_bucket,
                COUNT(*) as snapshot_count,
                SUM(error_count) as total_errors,
                SUM(warning_count) as total_warnings,
                AVG(error_count) as avg_errors,
                AVG(warning_count) as avg_warnings,
                COUNT(DISTINCT file_path) as unique_files
            FROM diagnostic_snapshots
            WHERE timestamp >= {} AND timestamp <= {}
            GROUP BY time_bucket
            ORDER BY time_bucket
            "#,
            interval_secs, interval_secs, start_ts, end_ts
        );

        let points = self.pool.with_read_connection(move |conn| {
            let mut stmt = conn.prepare(&query)?;
            let points = stmt
            .query_map([], |row| {
                let bucket_secs: i64 = row.get(0)?;

                Ok(TimeSeriesPoint {
                    timestamp: UNIX_EPOCH + Duration::from_secs(bucket_secs as u64),
                    snapshot_count: row.get(1)?,
                    total_errors: row.get(2)?,
                    total_warnings: row.get(3)?,
                    avg_errors: row.get(4)?,
                    avg_warnings: row.get(5)?,
                    unique_files: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
            
            Ok(points)
        }).await.map_err(|e| DatabaseError::Sqlite {
            operation: "get_time_series_data".to_string(),
            message: e.to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        Ok(points)
    }

    async fn cleanup_old_data(&self, retention_days: u64) -> Result<usize, DatabaseError> {
        let retention_secs = retention_days * 24 * 60 * 60;
        let cutoff_time = Self::convert_timestamp_to_secs(SystemTime::now())? - retention_secs as i64;

        let deleted = self.pool.with_connection(move |conn| {
            let deleted = conn.execute(
                "DELETE FROM diagnostic_snapshots WHERE created_at < ?",
                [cutoff_time],
            )?;

            if deleted > 0 {
                conn.execute(
                    "DELETE FROM file_stats WHERE file_path NOT IN (SELECT DISTINCT file_path FROM diagnostic_snapshots)",
                    [],
                )?;
            }

            Ok(deleted)
        }).await.map_err(|e| DatabaseError::Sqlite {
            operation: "cleanup_old_data".to_string(),
            message: e.to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        if deleted > 0 {
            info!("Cleaned up {} old diagnostic snapshots", deleted);
        }

        *self.last_cleanup.write().await = SystemTime::now();
        Ok(deleted)
    }

    async fn export_ml_ready_data(&self, output_path: &Path) -> Result<(), DatabaseError> {
        let query = r#"
            SELECT 
                s.timestamp,
                s.file_path,
                s.diagnostics_json,
                f.avg_error_count,
                f.avg_warning_count,
                f.total_snapshots
            FROM diagnostic_snapshots s
            JOIN file_stats f ON s.file_path = f.file_path
            ORDER BY s.timestamp
        "#;

        let output_path_buf = output_path.to_path_buf();
        
        let ml_data = self.pool.with_read_connection(move |conn| {
            let mut stmt = conn.prepare(query)?;
            let mut ml_data = Vec::new();

            let rows = stmt.query_map([], |row| {
                Ok(MLDataPoint {
                    timestamp: row.get::<_, i64>(0)?,
                    file_path: row.get::<_, String>(1)?,
                    diagnostics: row.get::<_, String>(2)?,
                    historical_avg_errors: row.get::<_, f64>(3)?,
                    historical_avg_warnings: row.get::<_, f64>(4)?,
                    file_complexity_score: row.get::<_, i64>(5)? as f64 / 100.0,
                })
            })?;

            for row in rows {
                ml_data.push(row?);
            }
            
            Ok(ml_data)
        }).await.map_err(|e| DatabaseError::Sqlite {
            operation: "export_ml_ready_data".to_string(),
            message: e.to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;

        // Write to file in JSON Lines format
        use std::io::Write;
        let file = std::fs::File::create(&output_path_buf).map_err(|e| DatabaseError::Sqlite {
            operation: "create_export_file".to_string(),
            message: e.to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(e.to_string()),
            ),
        })?;
        let mut writer = std::io::BufWriter::new(file);

        for point in ml_data {
            serde_json::to_writer(&mut writer, &point).map_err(|e| DatabaseError::Serialization {
                data_type: "MLDataPoint".to_string(),
                reason: e.to_string(),
                source: bincode::ErrorKind::Custom(e.to_string()).into(),
            })?;
            writeln!(&mut writer).map_err(|e| DatabaseError::Sqlite {
                operation: "write_export_line".to_string(),
                message: e.to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(e.to_string()),
                ),
            })?;
        }

        info!("Exported ML-ready data to {:?}", output_path_buf);
        Ok(())
    }

    async fn should_cleanup(&self) -> bool {
        let last_cleanup = self.last_cleanup.read().await;
        
        match SystemTime::now().duration_since(*last_cleanup) {
            Ok(duration) => duration >= self.config.auto_cleanup_interval,
            Err(_) => true, // Clock went backwards, do cleanup
        }
    }

    async fn update_last_cleanup(&self) -> Result<(), DatabaseError> {
        *self.last_cleanup.write().await = SystemTime::now();
        Ok(())
    }
}