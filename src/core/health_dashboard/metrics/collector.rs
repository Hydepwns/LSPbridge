use anyhow::Result;
use std::time::{Duration, SystemTime};

use crate::core::{
    DynamicConfigManager, ErrorRecoverySystem, GitIntegration, SimpleEnhancedProcessor,
};
use crate::core::health_dashboard::types::{ComponentHealth, ComponentMetrics, ComponentStatus};

pub struct MetricsCollector;

impl MetricsCollector {
    pub async fn collect_processor_health(
        processor: &SimpleEnhancedProcessor,
    ) -> Result<ComponentHealth> {
        let summary = processor.get_performance_summary().await?;

        let mut score = 100.0;
        let mut issues = Vec::new();

        // Check cache hit rate
        if let Some(persistent_stats) = &summary.persistent_cache_stats {
            let hit_rate = persistent_stats.hit_rate();
            if hit_rate < 0.5 {
                score -= 20.0;
                issues.push(format!("Low cache hit rate: {:.1}%", hit_rate * 100.0));
            }
        }

        // Check error rate
        if summary.recent_error_rate > 5.0 {
            score -= 30.0;
            issues.push(format!("High error rate: {:.1}", summary.recent_error_rate));
        }

        let status = if score >= 90.0 {
            ComponentStatus::Online
        } else if score >= 70.0 {
            ComponentStatus::Degraded
        } else {
            ComponentStatus::Offline
        };

        Ok(ComponentHealth {
            name: "Processor".to_string(),
            status,
            score,
            metrics: ComponentMetrics {
                cpu_usage: 0.0,    // Would need system metrics integration
                memory_usage: 0.0, // Would need process memory tracking
                error_rate: summary.recent_error_rate,
                response_time: Duration::from_millis(100), // Placeholder
                throughput: summary.core_cache_files as f64,
                custom_metrics: Default::default(),
            },
            last_check: SystemTime::now(),
            issues,
        })
    }

    pub async fn collect_metrics_health(
        _metrics: &crate::core::MetricsCollector,
    ) -> Result<ComponentHealth> {
        // Placeholder implementation
        Ok(ComponentHealth {
            name: "Metrics".to_string(),
            status: ComponentStatus::Online,
            score: 100.0,
            metrics: ComponentMetrics {
                cpu_usage: 5.0,
                memory_usage: 10.0,
                error_rate: 0.0,
                response_time: Duration::from_millis(50),
                throughput: 100.0,
                custom_metrics: Default::default(),
            },
            last_check: SystemTime::now(),
            issues: Vec::new(),
        })
    }

    pub async fn collect_config_health(
        _config: &DynamicConfigManager,
    ) -> Result<ComponentHealth> {
        // Placeholder implementation
        Ok(ComponentHealth {
            name: "Configuration".to_string(),
            status: ComponentStatus::Online,
            score: 100.0,
            metrics: ComponentMetrics {
                cpu_usage: 1.0,
                memory_usage: 5.0,
                error_rate: 0.0,
                response_time: Duration::from_millis(10),
                throughput: 10.0,
                custom_metrics: Default::default(),
            },
            last_check: SystemTime::now(),
            issues: Vec::new(),
        })
    }

    pub async fn collect_git_health(git: &GitIntegration) -> Result<ComponentHealth> {
        let mut score = 100.0;
        let mut issues = Vec::new();

        // Check if Git is available
        if !git.is_git_available().await {
            score = 0.0;
            issues.push("Git not available".to_string());
        } else {
            // Check repository status
            if let Err(e) = git.is_repository_clean().await {
                score -= 20.0;
                issues.push(format!("Git status check failed: {e}"));
            }
        }

        let status = if score >= 90.0 {
            ComponentStatus::Online
        } else if score > 0.0 {
            ComponentStatus::Degraded
        } else {
            ComponentStatus::Offline
        };

        Ok(ComponentHealth {
            name: "Git Integration".to_string(),
            status,
            score,
            metrics: ComponentMetrics {
                cpu_usage: 2.0,
                memory_usage: 8.0,
                error_rate: if issues.is_empty() { 0.0 } else { 1.0 },
                response_time: Duration::from_millis(200),
                throughput: 50.0,
                custom_metrics: Default::default(),
            },
            last_check: SystemTime::now(),
            issues,
        })
    }

    pub async fn collect_recovery_health(
        recovery: &ErrorRecoverySystem,
    ) -> Result<ComponentHealth> {
        let stats = recovery.get_error_statistics().await;

        let mut score = 100.0;
        let mut issues = Vec::new();

        // Check error rate
        if stats.recent_error_rate > 5.0 {
            score -= 25.0;
            issues.push(format!("High error rate: {:.1}", stats.recent_error_rate));
        }

        // Check circuit breaker status
        if stats.circuit_breaker_open {
            score -= 50.0;
            issues.push("Circuit breaker is open".to_string());
        }

        let status = if score >= 90.0 {
            ComponentStatus::Online
        } else if score >= 50.0 {
            ComponentStatus::Degraded
        } else {
            ComponentStatus::Offline
        };

        Ok(ComponentHealth {
            name: "Error Recovery".to_string(),
            status,
            score,
            metrics: ComponentMetrics {
                cpu_usage: 3.0,
                memory_usage: 12.0,
                error_rate: stats.recent_error_rate,
                response_time: Duration::from_millis(100),
                throughput: 75.0,
                custom_metrics: Default::default(),
            },
            last_check: SystemTime::now(),
            issues,
        })
    }
}