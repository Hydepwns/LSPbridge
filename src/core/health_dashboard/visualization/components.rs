use crate::core::health_dashboard::types::{
    DashboardMetrics, EffortLevel, ImpactLevel, PerformanceRecommendation,
};
use std::time::SystemTime;

pub struct DashboardComponents;

impl DashboardComponents {
    /// Generate performance recommendations based on current metrics
    pub fn generate_recommendations(metrics: &DashboardMetrics) -> Vec<PerformanceRecommendation> {
        let mut recommendations = Vec::new();

        // Cache hit rate recommendation
        if metrics.cache_hit_rate < 0.7 {
            recommendations.push(PerformanceRecommendation {
                id: "cache-hit-rate".to_string(),
                component: "cache".to_string(),
                recommendation: if metrics.cache_hit_rate < 0.5 {
                    "Critical: Cache hit rate is very low. Consider increasing cache size, adjusting TTL settings, or reviewing cache key generation logic.".to_string()
                } else {
                    "Cache hit rate is below optimal. Consider increasing cache size or adjusting TTL settings.".to_string()
                },
                impact: if metrics.cache_hit_rate < 0.5 {
                    ImpactLevel::Critical
                } else {
                    ImpactLevel::High
                },
                effort: EffortLevel::Low,
                timestamp: SystemTime::now(),
            });
        }

        // Error rate recommendation
        if metrics.error_rate > 2.0 {
            recommendations.push(PerformanceRecommendation {
                id: "error-rate".to_string(),
                component: "processor".to_string(),
                recommendation: if metrics.error_rate > 5.0 {
                    "Critical: Error rate is very high. Immediate investigation required. Review error logs and implement additional error handling.".to_string()
                } else {
                    "Error rate is elevated. Review error logs and consider implementing additional error handling.".to_string()
                },
                impact: if metrics.error_rate > 5.0 {
                    ImpactLevel::Critical
                } else {
                    ImpactLevel::High
                },
                effort: EffortLevel::Medium,
                timestamp: SystemTime::now(),
            });
        }

        // Memory usage recommendation
        if metrics.memory_usage_mb > 1024.0 {
            recommendations.push(PerformanceRecommendation {
                id: "memory-usage".to_string(),
                component: "system".to_string(),
                recommendation: "High memory usage detected. Consider optimizing data structures or implementing memory limits.".to_string(),
                impact: ImpactLevel::Medium,
                effort: EffortLevel::High,
                timestamp: SystemTime::now(),
            });
        }

        // Processing time recommendation (if we had the data)
        if metrics.avg_processing_time.as_millis() > 1000 {
            recommendations.push(PerformanceRecommendation {
                id: "processing-time".to_string(),
                component: "processor".to_string(),
                recommendation: "Average processing time is high. Consider optimizing algorithms or adding parallel processing.".to_string(),
                impact: ImpactLevel::High,
                effort: EffortLevel::High,
                timestamp: SystemTime::now(),
            });
        }

        // Git integration recommendation
        if metrics.git_files_tracked == 0 && metrics.uptime.as_secs() > 60 {
            recommendations.push(PerformanceRecommendation {
                id: "git-integration".to_string(),
                component: "git".to_string(),
                recommendation: "No Git files are being tracked. Ensure Git integration is properly configured.".to_string(),
                impact: ImpactLevel::Low,
                effort: EffortLevel::Minimal,
                timestamp: SystemTime::now(),
            });
        }

        recommendations
    }

    /// Generate a health score summary
    pub fn calculate_health_summary(metrics: &DashboardMetrics) -> (f64, String) {
        let mut score = 100.0;
        let mut issues = Vec::new();

        // Deduct points for poor cache performance
        if metrics.cache_hit_rate < 0.7 {
            let penalty = (0.7 - metrics.cache_hit_rate) * 30.0;
            score -= penalty;
            issues.push("Low cache hit rate");
        }

        // Deduct points for high error rate
        if metrics.error_rate > 1.0 {
            let penalty = metrics.error_rate.min(10.0) * 3.0;
            score -= penalty;
            issues.push("Elevated error rate");
        }

        // Deduct points for high memory usage
        if metrics.memory_usage_mb > 512.0 {
            let penalty = ((metrics.memory_usage_mb - 512.0) / 512.0 * 10.0).min(20.0);
            score -= penalty;
            issues.push("High memory usage");
        }

        score = score.max(0.0);

        let summary = if issues.is_empty() {
            "All systems operating normally".to_string()
        } else {
            format!("Issues detected: {}", issues.join(", "))
        };

        (score, summary)
    }
}