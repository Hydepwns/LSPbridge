use crate::core::health_dashboard::types::{
    ComponentHealthMap, DashboardMetrics, SystemHealthStatus,
};
use crate::core::{GitIntegration, SimpleEnhancedProcessor};
use anyhow::Result;
use std::time::Duration;

pub struct MetricsAggregator;

impl MetricsAggregator {
    /// Calculate overall system health status based on component scores
    pub fn calculate_overall_status(components: &ComponentHealthMap) -> SystemHealthStatus {
        let component_scores: Vec<f64> = components.values().map(|c| c.score).collect();

        if component_scores.is_empty() {
            return SystemHealthStatus::Unknown;
        }

        let avg_score = component_scores.iter().sum::<f64>() / component_scores.len() as f64;
        let min_score = component_scores.iter().cloned().fold(100.0, f64::min);

        // Consider both average and minimum scores
        let effective_score = (avg_score + min_score) / 2.0;

        match effective_score {
            s if s >= 90.0 => SystemHealthStatus::Healthy,
            s if s >= 70.0 => SystemHealthStatus::Degraded,
            s if s >= 50.0 => SystemHealthStatus::Unhealthy,
            _ => SystemHealthStatus::Critical,
        }
    }

    /// Aggregate metrics from various sources
    pub async fn aggregate_dashboard_metrics(
        processor: &SimpleEnhancedProcessor,
        git: Option<&GitIntegration>,
        uptime: Duration,
    ) -> Result<DashboardMetrics> {
        // Get processor summary
        let summary = processor.get_performance_summary().await?;

        let mut metrics = DashboardMetrics {
            files_processed_total: summary.core_cache_files as u64,
            cache_hit_rate: 0.0,
            error_rate: summary.recent_error_rate,
            avg_processing_time: Duration::from_millis(0), // Would need actual tracking
            memory_usage_mb: 128.0,                         // Placeholder - would need actual memory tracking
            git_files_tracked: 0,
            config_changes_today: 0, // Would need actual tracking
            uptime,
        };

        // Update cache hit rate
        if let Some(persistent_stats) = &summary.persistent_cache_stats {
            metrics.cache_hit_rate = persistent_stats.hit_rate();
        }

        // Update Git metrics
        if let Some(git_integration) = git {
            if let Ok(modified_files) = git_integration.get_modified_files().await {
                metrics.git_files_tracked = modified_files.len();
            }
        }

        Ok(metrics)
    }
}