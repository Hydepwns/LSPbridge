use crate::core::{Diagnostic, FileHash};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    pub id: i64,
    pub timestamp: SystemTime,
    pub file_path: PathBuf,
    pub file_hash: FileHash,
    pub diagnostics: Vec<Diagnostic>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub hint_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub db_path: PathBuf,
    pub retention_days: u64,
    pub max_snapshots_per_file: usize,
    pub auto_cleanup_interval: Duration,
    /// Database connection pool settings
    pub min_connections: usize,
    pub max_connections: usize,
    pub connection_timeout_secs: u64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            db_path: crate::config::data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("lsp-bridge"))
                .join("history.db"),
            retention_days: 30,
            max_snapshots_per_file: 1000,
            auto_cleanup_interval: Duration::from_secs(24 * 60 * 60), // Daily
            min_connections: 2,
            max_connections: 10,
            connection_timeout_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistoryStats {
    pub file_path: PathBuf,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub total_snapshots: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub avg_error_count: f64,
    pub avg_warning_count: f64,
    pub max_error_count: usize,
    pub max_warning_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalErrorPattern {
    pub pattern_hash: String,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub occurrence_count: usize,
    pub files_affected: usize,
    pub error_message: String,
    pub error_code: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: SystemTime,
    pub snapshot_count: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub avg_errors: f64,
    pub avg_warnings: f64,
    pub unique_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLDataPoint {
    pub timestamp: i64,
    pub file_path: String,
    pub diagnostics: String,
    pub historical_avg_errors: f64,
    pub historical_avg_warnings: f64,
    pub file_complexity_score: f64,
}