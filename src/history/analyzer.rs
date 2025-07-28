use crate::core::DiagnosticSeverity;
use crate::history::storage::{
    DiagnosticSnapshot, HistoricalErrorPattern, HistoryStorage, TimeSeriesPoint,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub error_velocity: f32,       // Errors per hour
    pub warning_velocity: f32,     // Warnings per hour
    pub hot_spots: Vec<FileStats>, // Problem files
    pub recurring_issues: Vec<Pattern>,
    pub fix_time_estimates: HashMap<DiagnosticCategory, Duration>,
    pub trend_direction: TrendDirection,
    pub health_score: f32, // 0.0 (worst) to 1.0 (best)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    pub file_path: PathBuf,
    pub error_density: f32,    // Errors per snapshot
    pub warning_density: f32,  // Warnings per snapshot
    pub volatility_score: f32, // How much diagnostics change
    pub recent_trend: TrendDirection,
    pub last_error_count: usize,
    pub last_warning_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_id: String,
    pub description: String,
    pub occurrence_rate: f32, // Occurrences per day
    pub affected_files: Vec<PathBuf>,
    pub severity: DiagnosticSeverity,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    TypeErrors,
    SyntaxErrors,
    Linting,
    Runtime,
    Build,
    Other,
}

pub struct TrendAnalyzer {
    storage: Arc<HistoryStorage>,
}

impl TrendAnalyzer {
    pub fn new(storage: Arc<HistoryStorage>) -> Self {
        Self { storage }
    }

    pub async fn analyze_trends(
        &self,
        time_window: Duration,
        min_samples: usize,
    ) -> Result<TrendAnalysis> {
        let end_time = SystemTime::now();
        let start_time = end_time - time_window;

        // Get time series data
        let interval = Duration::from_secs(3600); // 1 hour buckets
        let time_series = self
            .storage
            .get_time_series_data(start_time, end_time, interval)
            .await?;

        // Calculate velocities
        let (error_velocity, warning_velocity) =
            self.calculate_velocities(&time_series, time_window);

        // Find hot spots
        let hot_spots = self.identify_hot_spots(start_time, end_time).await?;

        // Analyze recurring patterns
        let recurring_issues = self.analyze_recurring_patterns(min_samples).await?;

        // Estimate fix times
        let fix_time_estimates = self.estimate_fix_times(&time_series).await?;

        // Determine overall trend
        let trend_direction = self.determine_trend_direction(&time_series);

        // Calculate health score
        let health_score = self.calculate_health_score(&time_series, &hot_spots);

        Ok(TrendAnalysis {
            error_velocity,
            warning_velocity,
            hot_spots,
            recurring_issues,
            fix_time_estimates,
            trend_direction,
            health_score,
        })
    }

    pub async fn analyze_file_trends(
        &self,
        file_path: &Path,
        time_window: Duration,
    ) -> Result<FileTrendReport> {
        let snapshots = self
            .storage
            .get_snapshots_for_file(file_path, Some(SystemTime::now() - time_window), None)
            .await?;

        if snapshots.is_empty() {
            return Ok(FileTrendReport {
                file_path: file_path.to_path_buf(),
                trend_direction: TrendDirection::Stable,
                error_trend: Vec::new(),
                warning_trend: Vec::new(),
                volatility: 0.0,
                predictions: FilePredictions::default(),
            });
        }

        // Extract trends
        let mut error_trend = Vec::new();
        let mut warning_trend = Vec::new();

        for snapshot in &snapshots {
            error_trend.push((snapshot.timestamp, snapshot.error_count));
            warning_trend.push((snapshot.timestamp, snapshot.warning_count));
        }

        // Calculate volatility
        let volatility = self.calculate_volatility(&error_trend);

        // Determine trend direction
        let trend_direction = self.determine_file_trend(&error_trend, &warning_trend);

        // Make predictions
        let predictions = self.predict_file_future(&snapshots).await?;

        Ok(FileTrendReport {
            file_path: file_path.to_path_buf(),
            trend_direction,
            error_trend,
            warning_trend,
            volatility,
            predictions,
        })
    }

    pub async fn get_hot_spots(&self, limit: usize) -> Result<Vec<HotSpot>> {
        let end_time = SystemTime::now();
        let start_time = end_time - Duration::from_secs(7 * 24 * 60 * 60); // Last 7 days

        let hot_spots = self.identify_hot_spots(start_time, end_time).await?;

        Ok(hot_spots
            .into_iter()
            .take(limit)
            .map(|stats| {
                let recommendation = self.generate_recommendation(&stats);
                HotSpot {
                    file_path: stats.file_path,
                    score: stats.error_density * 2.0 + stats.warning_density,
                    recent_errors: stats.last_error_count,
                    recent_warnings: stats.last_warning_count,
                    trend: stats.recent_trend,
                    recommendation,
                }
            })
            .collect())
    }

    pub async fn predict_fix_time(
        &self,
        diagnostic_category: DiagnosticCategory,
    ) -> Result<Duration> {
        // Analyze historical fix times
        let historical_data = self.get_historical_fix_data(diagnostic_category).await?;

        if historical_data.is_empty() {
            // Default estimates based on category
            return Ok(match diagnostic_category {
                DiagnosticCategory::SyntaxErrors => Duration::from_secs(5 * 60), // 5 minutes
                DiagnosticCategory::TypeErrors => Duration::from_secs(15 * 60),  // 15 minutes
                DiagnosticCategory::Linting => Duration::from_secs(10 * 60),     // 10 minutes
                DiagnosticCategory::Runtime => Duration::from_secs(30 * 60),     // 30 minutes
                DiagnosticCategory::Build => Duration::from_secs(20 * 60),       // 20 minutes
                DiagnosticCategory::Other => Duration::from_secs(15 * 60),       // 15 minutes
            });
        }

        // Calculate average fix time from historical data
        let total_time: Duration = historical_data.iter().map(|d| d.fix_duration).sum();
        let avg_time = total_time / historical_data.len() as u32;

        Ok(avg_time)
    }

    // Private helper methods

    fn calculate_velocities(
        &self,
        time_series: &[TimeSeriesPoint],
        window: Duration,
    ) -> (f32, f32) {
        if time_series.is_empty() {
            return (0.0, 0.0);
        }

        let total_errors: usize = time_series.iter().map(|p| p.total_errors).sum();
        let total_warnings: usize = time_series.iter().map(|p| p.total_warnings).sum();

        let hours = window.as_secs() as f32 / 3600.0;

        (total_errors as f32 / hours, total_warnings as f32 / hours)
    }

    async fn identify_hot_spots(
        &self,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Result<Vec<FileStats>> {
        // Get all unique files with activity in the time window
        let time_series = self
            .storage
            .get_time_series_data(
                start_time,
                end_time,
                Duration::from_secs(24 * 60 * 60), // Daily buckets
            )
            .await?;

        let mut file_stats_map = HashMap::new();

        // Aggregate stats per file
        for point in &time_series {
            // Note: This is simplified. In a real implementation, we'd query per-file data
            // For now, we'll use the unique_files count as a proxy
            debug!(
                "Processing time series point with {} unique files",
                point.unique_files
            );
        }

        // Get recurring patterns to identify problem files
        let patterns = self.storage.get_recurring_patterns(5).await?;

        for pattern in patterns {
            // Track files affected by recurring issues
            let stats = file_stats_map
                .entry(pattern.error_message.clone())
                .or_insert(FileStats {
                    file_path: PathBuf::from("aggregate"), // Placeholder
                    error_density: 0.0,
                    warning_density: 0.0,
                    volatility_score: 0.0,
                    recent_trend: TrendDirection::Stable,
                    last_error_count: 0,
                    last_warning_count: 0,
                });

            stats.error_density =
                pattern.occurrence_count as f32 / (pattern.files_affected as f32).max(1.0);
        }

        // Sort by problem score (error_density * 2 + warning_density)
        let mut hot_spots: Vec<FileStats> = file_stats_map.into_values().collect();
        hot_spots.sort_by(|a, b| {
            let score_a = a.error_density * 2.0 + a.warning_density;
            let score_b = b.error_density * 2.0 + b.warning_density;
            score_b.partial_cmp(&score_a).unwrap()
        });

        Ok(hot_spots)
    }

    async fn analyze_recurring_patterns(&self, min_occurrences: usize) -> Result<Vec<Pattern>> {
        let error_patterns = self.storage.get_recurring_patterns(min_occurrences).await?;

        let patterns: Vec<Pattern> = error_patterns
            .into_iter()
            .map(|ep| {
                let days_active = ep
                    .last_seen
                    .duration_since(ep.first_seen)
                    .unwrap_or(Duration::from_secs(1))
                    .as_secs() as f32
                    / (24.0 * 3600.0);

                let suggested_action = self.suggest_action_for_pattern(&ep);

                Pattern {
                    pattern_id: ep.pattern_hash,
                    description: ep.error_message,
                    occurrence_rate: ep.occurrence_count as f32 / days_active.max(1.0),
                    affected_files: vec![], // Would need additional query to get actual files
                    severity: DiagnosticSeverity::Error,
                    suggested_action,
                }
            })
            .collect();

        Ok(patterns)
    }

    async fn estimate_fix_times(
        &self,
        _time_series: &[TimeSeriesPoint],
    ) -> Result<HashMap<DiagnosticCategory, Duration>> {
        let mut estimates = HashMap::new();

        // These are placeholder estimates. In a real implementation,
        // we would analyze historical data to see how long errors typically persist
        estimates.insert(
            DiagnosticCategory::SyntaxErrors,
            Duration::from_secs(5 * 60),
        );
        estimates.insert(DiagnosticCategory::TypeErrors, Duration::from_secs(15 * 60));
        estimates.insert(DiagnosticCategory::Linting, Duration::from_secs(10 * 60));
        estimates.insert(DiagnosticCategory::Runtime, Duration::from_secs(30 * 60));
        estimates.insert(DiagnosticCategory::Build, Duration::from_secs(20 * 60));
        estimates.insert(DiagnosticCategory::Other, Duration::from_secs(15 * 60));

        Ok(estimates)
    }

    fn determine_trend_direction(&self, time_series: &[TimeSeriesPoint]) -> TrendDirection {
        if time_series.len() < 2 {
            return TrendDirection::Stable;
        }

        // Simple linear regression on error counts
        let n = time_series.len() as f32;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_xx = 0.0;

        for (i, point) in time_series.iter().enumerate() {
            let x = i as f32;
            let y = point.total_errors as f32;

            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);

        if slope < -0.5 {
            TrendDirection::Improving
        } else if slope > 0.5 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        }
    }

    fn calculate_health_score(
        &self,
        time_series: &[TimeSeriesPoint],
        hot_spots: &[FileStats],
    ) -> f32 {
        if time_series.is_empty() {
            return 1.0; // No data = healthy
        }

        let latest = time_series.last().unwrap();

        // Factor 1: Error density (0.0 to 1.0, inverted)
        let error_factor = 1.0 / (1.0 + latest.avg_errors as f32 / 10.0);

        // Factor 2: Warning density (0.0 to 1.0, inverted)
        let warning_factor = 1.0 / (1.0 + latest.avg_warnings as f32 / 20.0);

        // Factor 3: Hot spot count (0.0 to 1.0, inverted)
        let hot_spot_factor = 1.0 / (1.0 + hot_spots.len() as f32 / 10.0);

        // Weighted average
        (error_factor * 0.5 + warning_factor * 0.3 + hot_spot_factor * 0.2).clamp(0.0, 1.0)
    }

    fn calculate_volatility(&self, trend: &[(SystemTime, usize)]) -> f32 {
        if trend.len() < 2 {
            return 0.0;
        }

        let values: Vec<f32> = trend.iter().map(|(_, count)| *count as f32).collect();
        let mean = values.iter().sum::<f32>() / values.len() as f32;

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;

        variance.sqrt() / (mean + 1.0) // Coefficient of variation
    }

    fn determine_file_trend(
        &self,
        error_trend: &[(SystemTime, usize)],
        warning_trend: &[(SystemTime, usize)],
    ) -> TrendDirection {
        if error_trend.len() < 2 {
            return TrendDirection::Stable;
        }

        // Compare first half average to second half average
        let mid = error_trend.len() / 2;

        let first_half_errors: f32 = error_trend[..mid]
            .iter()
            .map(|(_, count)| *count as f32)
            .sum::<f32>()
            / mid as f32;

        let second_half_errors: f32 = error_trend[mid..]
            .iter()
            .map(|(_, count)| *count as f32)
            .sum::<f32>()
            / (error_trend.len() - mid) as f32;

        let first_half_warnings: f32 = warning_trend[..mid]
            .iter()
            .map(|(_, count)| *count as f32)
            .sum::<f32>()
            / mid as f32;

        let second_half_warnings: f32 = warning_trend[mid..]
            .iter()
            .map(|(_, count)| *count as f32)
            .sum::<f32>()
            / (warning_trend.len() - mid) as f32;

        let error_change = second_half_errors - first_half_errors;
        let warning_change = second_half_warnings - first_half_warnings;
        let total_change = error_change * 2.0 + warning_change; // Errors weighted more

        if total_change < -1.0 {
            TrendDirection::Improving
        } else if total_change > 1.0 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        }
    }

    async fn predict_file_future(
        &self,
        snapshots: &[DiagnosticSnapshot],
    ) -> Result<FilePredictions> {
        if snapshots.len() < 3 {
            return Ok(FilePredictions::default());
        }

        // Simple moving average prediction
        let recent_errors: Vec<usize> = snapshots.iter().take(5).map(|s| s.error_count).collect();

        let recent_warnings: Vec<usize> =
            snapshots.iter().take(5).map(|s| s.warning_count).collect();

        let predicted_errors = recent_errors.iter().sum::<usize>() / recent_errors.len();
        let predicted_warnings = recent_warnings.iter().sum::<usize>() / recent_warnings.len();

        // Confidence based on volatility
        let volatility = self.calculate_volatility(
            &snapshots
                .iter()
                .map(|s| (s.timestamp, s.error_count))
                .collect::<Vec<_>>(),
        );

        let confidence = 1.0 / (1.0 + volatility);

        Ok(FilePredictions {
            next_hour_errors: predicted_errors,
            next_hour_warnings: predicted_warnings,
            confidence,
            suggested_action: self.suggest_file_action(predicted_errors, predicted_warnings),
        })
    }

    fn suggest_action_for_pattern(&self, pattern: &HistoricalErrorPattern) -> String {
        if pattern.occurrence_count > 100 {
            "Critical: This error pattern is very frequent. Consider creating a custom lint rule or automated fix.".to_string()
        } else if pattern.files_affected > 10 {
            "Widespread issue affecting multiple files. Consider a codebase-wide refactoring."
                .to_string()
        } else if pattern.error_code.as_deref() == Some("TS2304") {
            "TypeScript cannot find name. Check imports and type definitions.".to_string()
        } else {
            "Monitor this pattern. Consider fixing if frequency increases.".to_string()
        }
    }

    fn generate_recommendation(&self, stats: &FileStats) -> String {
        if stats.error_density > 5.0 {
            "High error density. This file needs immediate attention.".to_string()
        } else if stats.volatility_score > 0.8 {
            "High volatility. Consider stabilizing this file with better test coverage.".to_string()
        } else if stats.recent_trend == TrendDirection::Degrading {
            "Trend is degrading. Review recent changes to this file.".to_string()
        } else {
            "Monitor this file for changes.".to_string()
        }
    }

    fn suggest_file_action(&self, predicted_errors: usize, predicted_warnings: usize) -> String {
        if predicted_errors > 10 {
            "High error count predicted. Schedule immediate review.".to_string()
        } else if predicted_errors > 5 {
            "Moderate error count predicted. Plan for fixes in next sprint.".to_string()
        } else if predicted_warnings > 20 {
            "High warning count. Consider addressing warnings to prevent future errors.".to_string()
        } else {
            "File is stable. Continue monitoring.".to_string()
        }
    }

    async fn get_historical_fix_data(&self, _category: DiagnosticCategory) -> Result<Vec<FixData>> {
        // This would query historical data about how long it took to fix issues
        // For now, return empty to use defaults
        Ok(vec![])
    }
}

// Supporting types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrendReport {
    pub file_path: PathBuf,
    pub trend_direction: TrendDirection,
    pub error_trend: Vec<(SystemTime, usize)>,
    pub warning_trend: Vec<(SystemTime, usize)>,
    pub volatility: f32,
    pub predictions: FilePredictions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilePredictions {
    pub next_hour_errors: usize,
    pub next_hour_warnings: usize,
    pub confidence: f32,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSpot {
    pub file_path: PathBuf,
    pub score: f32,
    pub recent_errors: usize,
    pub recent_warnings: usize,
    pub trend: TrendDirection,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
struct FixData {
    pub fix_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::storage::HistoryConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_trend_analysis() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = HistoryConfig {
            db_path: temp_dir.path().join("test_history.db"),
            ..Default::default()
        };

        let storage = Arc::new(HistoryStorage::new(config).await?);
        let analyzer = TrendAnalyzer::new(storage);

        // Test with minimal data
        let analysis = analyzer
            .analyze_trends(
                Duration::from_secs(24 * 60 * 60), // 24 hours
                5,                                 // min samples
            )
            .await?;

        assert_eq!(analysis.error_velocity, 0.0);
        assert_eq!(analysis.trend_direction, TrendDirection::Stable);
        assert!(analysis.health_score > 0.9);

        Ok(())
    }
}
