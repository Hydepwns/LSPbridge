pub mod analyzer;
pub mod storage;
pub mod visualization;

pub use storage::{
    DiagnosticSnapshot, FileHistoryStats, HistoricalErrorPattern, HistoryConfig, HistoryStorage,
    MLDataPoint, TimeSeriesPoint,
};

pub use analyzer::{
    DiagnosticCategory, FilePredictions, FileStats, FileTrendReport, HotSpot, Pattern,
    TrendAnalysis, TrendAnalyzer, TrendDirection,
};

pub use visualization::{
    generate_html_dashboard, VisualizationData, VisualizationExporter, VisualizationLibrary,
};

use clap::Subcommand;
use std::path::PathBuf;

/// Actions for managing diagnostic history
#[derive(Debug, Clone, Subcommand)]
pub enum HistoryAction {
    /// View diagnostic trends over time
    Trends {
        /// Number of hours to analyze
        #[arg(short = 'h', long, default_value = "24")]
        hours: u64,
        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: crate::cli::OutputFormat,
    },
    /// Find diagnostic hot spots
    HotSpots {
        /// Maximum number of hot spots to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: crate::cli::OutputFormat,
    },
    /// Get history for a specific file
    File {
        /// File path to analyze
        path: PathBuf,
        /// Number of hours to analyze
        #[arg(short = 'h', long, default_value = "24")]
        hours: u64,
        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: crate::cli::OutputFormat,
    },
    /// Clean old history data
    Clean {
        /// Delete data older than this many days
        #[arg(long, default_value = "30")]
        older_than_days: u32,
    },
}

use crate::core::{Diagnostic, FileHash};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// High-level interface for historical analysis
pub struct HistoryManager {
    storage: Arc<HistoryStorage>,
    analyzer: TrendAnalyzer,
}

impl HistoryManager {
    pub async fn new(config: HistoryConfig) -> Result<Self> {
        let storage = Arc::new(HistoryStorage::new(config).await?);
        let analyzer = TrendAnalyzer::new(storage.clone());

        Ok(Self { storage, analyzer })
    }

    /// Record a new diagnostic snapshot
    pub async fn record_diagnostics(
        &self,
        file_path: &Path,
        file_hash: FileHash,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<()> {
        let snapshot = DiagnosticSnapshot {
            id: 0, // Will be assigned by database
            timestamp: SystemTime::now(),
            file_path: file_path.to_path_buf(),
            file_hash,
            diagnostics: diagnostics.clone(),
            error_count: diagnostics
                .iter()
                .filter(|d| d.severity == crate::core::DiagnosticSeverity::Error)
                .count(),
            warning_count: diagnostics
                .iter()
                .filter(|d| d.severity == crate::core::DiagnosticSeverity::Warning)
                .count(),
            info_count: diagnostics
                .iter()
                .filter(|d| d.severity == crate::core::DiagnosticSeverity::Information)
                .count(),
            hint_count: diagnostics
                .iter()
                .filter(|d| d.severity == crate::core::DiagnosticSeverity::Hint)
                .count(),
        };

        self.storage.record_snapshot(snapshot).await?;
        Ok(())
    }

    /// Get trend analysis for the specified time window
    pub async fn get_trends(&self, time_window: Duration) -> Result<TrendAnalysis> {
        self.analyzer.analyze_trends(time_window, 5).await
    }

    /// Get file-specific trend analysis
    pub async fn get_file_trends(
        &self,
        file_path: &Path,
        time_window: Duration,
    ) -> Result<FileTrendReport> {
        self.analyzer
            .analyze_file_trends(file_path, time_window)
            .await
    }

    /// Get the current hot spots (problem files)
    pub async fn get_hot_spots(&self, limit: usize) -> Result<Vec<HotSpot>> {
        self.analyzer.get_hot_spots(limit).await
    }

    /// Predict fix time for a category of diagnostics
    pub async fn predict_fix_time(&self, category: DiagnosticCategory) -> Result<Duration> {
        self.analyzer.predict_fix_time(category).await
    }

    /// Export data for ML training
    pub async fn export_ml_data(&self, output_path: &Path) -> Result<()> {
        self.storage
            .export_ml_ready_data(output_path)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get historical stats for a file
    pub async fn get_file_stats(&self, file_path: &Path) -> Result<Option<FileHistoryStats>> {
        self.storage
            .get_file_history_stats(file_path)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get recurring error patterns
    pub async fn get_recurring_patterns(
        &self,
        min_occurrences: usize,
    ) -> Result<Vec<HistoricalErrorPattern>> {
        self.storage
            .get_recurring_patterns(min_occurrences)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get time series data for custom analysis
    pub async fn get_time_series(
        &self,
        start: SystemTime,
        end: SystemTime,
        interval: Duration,
    ) -> Result<Vec<TimeSeriesPoint>> {
        self.storage
            .get_time_series_data(start, end, interval)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Clean old data from the history storage
    pub async fn clean_old_data(&self, _cutoff_date: chrono::DateTime<chrono::Utc>) -> Result<usize> {
        // For now, we don't have a specific method in HistoryStorage for this
        // This would typically be implemented in the storage layer
        Ok(0)
    }

    /// Export visualization data
    pub async fn export_visualization(
        &self,
        output_path: &Path,
        format: VisualizationFormat,
        time_window: Duration,
    ) -> Result<()> {
        let end = SystemTime::now();
        let start = end - time_window;

        // Get various data for visualization
        let time_series = self
            .get_time_series(start, end, Duration::from_secs(3600))
            .await?;
        let trends = self.get_trends(time_window).await?;
        let hot_spots = self.get_hot_spots(10).await?;

        // Create visualization data
        let mut all_charts = Vec::new();

        // Add time series visualization
        let ts_viz = VisualizationExporter::export_time_series(&time_series, "Diagnostic Trends")?;
        all_charts.extend(ts_viz.charts);

        // Add hot spots visualization
        let hs_viz = VisualizationExporter::export_hot_spots(&hot_spots)?;
        all_charts.extend(hs_viz.charts);

        // Add trend analysis visualization
        let trend_viz = VisualizationExporter::export_trend_analysis(&trends)?;
        all_charts.extend(trend_viz.charts);

        let combined_viz = VisualizationData {
            charts: all_charts,
            metadata: visualization::VisualizationMetadata {
                generated_at: SystemTime::now(),
                title: "LSP Bridge Diagnostic Analytics".to_string(),
                description: format!(
                    "Comprehensive diagnostic analysis for the last {} hours",
                    time_window.as_secs() / 3600
                ),
                time_range: visualization::TimeRange { start, end },
            },
        };

        // Export based on format
        let output_content = match format {
            VisualizationFormat::Html => generate_html_dashboard(&combined_viz)?,
            VisualizationFormat::Plotly => VisualizationExporter::export_for_library(
                &combined_viz,
                VisualizationLibrary::Plotly,
            )?,
            VisualizationFormat::ChartJs => VisualizationExporter::export_for_library(
                &combined_viz,
                VisualizationLibrary::ChartJs,
            )?,
            VisualizationFormat::Vega => VisualizationExporter::export_for_library(
                &combined_viz,
                VisualizationLibrary::Vega,
            )?,
            VisualizationFormat::Json => serde_json::to_string_pretty(&combined_viz)?,
        };

        std::fs::write(output_path, output_content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VisualizationFormat {
    Html,
    Plotly,
    ChartJs,
    Vega,
    Json,
}

// Implement Clone for HistoryStorage to allow sharing
impl Clone for HistoryStorage {
    fn clone(&self) -> Self {
        // Note: This is a simplified clone that shares the same database connection
        // In a production system, you might want to handle this differently
        panic!("HistoryStorage does not support cloning. Use Arc<HistoryStorage> instead.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_history_manager() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = HistoryConfig {
            db_path: temp_dir.path().join("test_history.db"),
            ..Default::default()
        };

        let manager = HistoryManager::new(config).await?;

        // Test recording diagnostics
        let file_path = Path::new("/test/file.rs");
        let file_hash = FileHash::new(b"test content");
        let diagnostics = vec![];

        manager
            .record_diagnostics(file_path, file_hash, diagnostics)
            .await?;

        // Test getting trends
        let trends = manager
            .get_trends(Duration::from_secs(24 * 60 * 60))
            .await?;
        assert_eq!(trends.error_velocity, 0.0);

        Ok(())
    }
}
