use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::core::{
    DynamicConfigManager, ErrorRecoverySystem, GitIntegration, MetricsCollector,
    SimpleEnhancedProcessor,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDashboard {
    pub timestamp: SystemTime,
    pub overall_status: SystemHealthStatus,
    pub components: ComponentHealthMap,
    pub metrics: DashboardMetrics,
    pub alerts: Vec<HealthAlert>,
    pub recommendations: Vec<PerformanceRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemHealthStatus {
    Healthy,   // All systems operational
    Degraded,  // Some issues but functional
    Unhealthy, // Significant problems
    Critical,  // System failure imminent
    Unknown,   // Status cannot be determined
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: ComponentStatus,
    pub score: f64, // 0-100 health score
    pub metrics: ComponentMetrics,
    pub last_check: SystemTime,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentStatus {
    Online,
    Degraded,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub error_rate: f64,
    pub response_time: Duration,
    pub throughput: f64,
    pub custom_metrics: HashMap<String, f64>,
}

pub type ComponentHealthMap = HashMap<String, ComponentHealth>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub files_processed_total: u64,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
    pub avg_processing_time: Duration,
    pub memory_usage_mb: f64,
    pub git_files_tracked: usize,
    pub config_changes_today: u64,
    pub uptime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub id: String,
    pub severity: AlertSeverity,
    pub component: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub resolved: bool,
    pub resolution_time: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecommendation {
    pub id: String,
    pub component: String,
    pub recommendation: String,
    pub impact: ImpactLevel,
    pub effort: EffortLevel,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Minimal,
    Low,
    Medium,
    High,
}

pub struct HealthMonitor {
    processor: Arc<SimpleEnhancedProcessor>,
    metrics_collector: Option<Arc<MetricsCollector>>,
    config_manager: Option<Arc<DynamicConfigManager>>,
    git_integration: Option<Arc<GitIntegration>>,
    error_recovery: Option<Arc<ErrorRecoverySystem>>,

    // Monitoring state
    dashboard_data: Arc<RwLock<HealthDashboard>>,
    start_time: Instant,
    alert_history: Arc<RwLock<Vec<HealthAlert>>>,
    component_history: Arc<RwLock<HashMap<String, Vec<ComponentHealth>>>>,

    // Configuration
    monitoring_config: MonitoringConfig,
}

#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub update_interval: Duration,
    pub retention_period: Duration,
    pub alert_thresholds: AlertThresholds,
    pub enable_recommendations: bool,
    pub max_alerts: usize,
    pub max_history_entries: usize,
}

#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub cpu_warning: f64,
    pub cpu_critical: f64,
    pub memory_warning: f64,
    pub memory_critical: f64,
    pub error_rate_warning: f64,
    pub error_rate_critical: f64,
    pub response_time_warning: Duration,
    pub response_time_critical: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(24 * 60 * 60), // 24 hours
            alert_thresholds: AlertThresholds {
                cpu_warning: 70.0,
                cpu_critical: 90.0,
                memory_warning: 80.0,
                memory_critical: 95.0,
                error_rate_warning: 5.0,
                error_rate_critical: 10.0,
                response_time_warning: Duration::from_millis(1000),
                response_time_critical: Duration::from_millis(5000),
            },
            enable_recommendations: true,
            max_alerts: 1000,
            max_history_entries: 10000,
        }
    }
}

impl HealthMonitor {
    pub async fn new(
        processor: Arc<SimpleEnhancedProcessor>,
        config: Option<MonitoringConfig>,
    ) -> Result<Self> {
        let monitoring_config = config.unwrap_or_default();

        let initial_dashboard = HealthDashboard {
            timestamp: SystemTime::now(),
            overall_status: SystemHealthStatus::Healthy,
            components: HashMap::new(),
            metrics: DashboardMetrics {
                files_processed_total: 0,
                cache_hit_rate: 0.0,
                error_rate: 0.0,
                avg_processing_time: Duration::from_millis(0),
                memory_usage_mb: 0.0,
                git_files_tracked: 0,
                config_changes_today: 0,
                uptime: Duration::from_secs(0),
            },
            alerts: Vec::new(),
            recommendations: Vec::new(),
        };

        let monitor = Self {
            processor,
            metrics_collector: None, // Will be set via setters
            config_manager: None,
            git_integration: None,
            error_recovery: None,
            dashboard_data: Arc::new(RwLock::new(initial_dashboard)),
            start_time: Instant::now(),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            component_history: Arc::new(RwLock::new(HashMap::new())),
            monitoring_config,
        };

        info!("Health monitor initialized");
        Ok(monitor)
    }

    // Setters for optional components
    pub fn with_metrics_collector(mut self, collector: Arc<MetricsCollector>) -> Self {
        self.metrics_collector = Some(collector);
        self
    }

    pub fn with_config_manager(mut self, manager: Arc<DynamicConfigManager>) -> Self {
        self.config_manager = Some(manager);
        self
    }

    pub fn with_git_integration(mut self, git: Arc<GitIntegration>) -> Self {
        self.git_integration = Some(git);
        self
    }

    pub fn with_error_recovery(mut self, recovery: Arc<ErrorRecoverySystem>) -> Self {
        self.error_recovery = Some(recovery);
        self
    }

    pub async fn start_monitoring(self: Arc<Self>) -> Result<()> {
        info!("Starting health monitoring");

        let update_interval = self.monitoring_config.update_interval;

        // Start the monitoring loop
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);

            loop {
                interval.tick().await;

                if let Err(e) = self.update_dashboard().await {
                    error!("Failed to update health dashboard: {}", e);
                }

                if let Err(e) = self.check_alerts().await {
                    error!("Failed to check alerts: {}", e);
                }

                if let Err(e) = self.generate_recommendations().await {
                    error!("Failed to generate recommendations: {}", e);
                }

                if let Err(e) = self.cleanup_old_data().await {
                    error!("Failed to cleanup old data: {}", e);
                }
            }
        });

        Ok(())
    }

    pub async fn get_dashboard(&self) -> HealthDashboard {
        let dashboard = self.dashboard_data.read().await;
        dashboard.clone()
    }

    pub async fn get_component_health(&self, component: &str) -> Option<ComponentHealth> {
        let dashboard = self.dashboard_data.read().await;
        dashboard.components.get(component).cloned()
    }

    pub async fn get_active_alerts(&self) -> Vec<HealthAlert> {
        let dashboard = self.dashboard_data.read().await;
        dashboard.alerts.clone()
    }

    pub async fn get_recommendations(&self) -> Vec<PerformanceRecommendation> {
        let dashboard = self.dashboard_data.read().await;
        dashboard.recommendations.clone()
    }

    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut dashboard = self.dashboard_data.write().await;

        if let Some(alert) = dashboard.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved = true;
            alert.resolution_time = Some(SystemTime::now());
            info!("Alert {} acknowledged", alert_id);
        } else {
            return Err(anyhow!("Alert {} not found", alert_id));
        }

        Ok(())
    }

    pub async fn update_dashboard(&self) -> Result<()> {
        let mut dashboard = self.dashboard_data.write().await;

        // Update timestamp and uptime
        dashboard.timestamp = SystemTime::now();
        dashboard.metrics.uptime = self.start_time.elapsed();

        // Update component health
        self.update_component_health(&mut dashboard).await?;

        // Update overall metrics
        self.update_dashboard_metrics(&mut dashboard).await?;

        // Calculate overall status
        dashboard.overall_status = self.calculate_overall_status(&dashboard);

        debug!("Dashboard updated");
        Ok(())
    }

    async fn update_component_health(&self, dashboard: &mut HealthDashboard) -> Result<()> {
        let _timestamp = SystemTime::now();

        // Core processor health
        let processor_health = self.check_processor_health().await?;
        dashboard
            .components
            .insert("processor".to_string(), processor_health);

        // Metrics collector health
        if let Some(metrics) = &self.metrics_collector {
            let metrics_health = self.check_metrics_health(metrics).await?;
            dashboard
                .components
                .insert("metrics".to_string(), metrics_health);
        }

        // Configuration manager health
        if let Some(config) = &self.config_manager {
            let config_health = self.check_config_health(config).await?;
            dashboard
                .components
                .insert("config".to_string(), config_health);
        }

        // Git integration health
        if let Some(git) = &self.git_integration {
            let git_health = self.check_git_health(git).await?;
            dashboard.components.insert("git".to_string(), git_health);
        }

        // Error recovery health
        if let Some(recovery) = &self.error_recovery {
            let recovery_health = self.check_recovery_health(recovery).await?;
            dashboard
                .components
                .insert("error_recovery".to_string(), recovery_health);
        }

        Ok(())
    }

    async fn check_processor_health(&self) -> Result<ComponentHealth> {
        let summary = self.processor.get_performance_summary().await?;

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
                custom_metrics: HashMap::new(),
            },
            last_check: SystemTime::now(),
            issues,
        })
    }

    async fn check_metrics_health(&self, _metrics: &MetricsCollector) -> Result<ComponentHealth> {
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
                custom_metrics: HashMap::new(),
            },
            last_check: SystemTime::now(),
            issues: Vec::new(),
        })
    }

    async fn check_config_health(&self, _config: &DynamicConfigManager) -> Result<ComponentHealth> {
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
                custom_metrics: HashMap::new(),
            },
            last_check: SystemTime::now(),
            issues: Vec::new(),
        })
    }

    async fn check_git_health(&self, git: &GitIntegration) -> Result<ComponentHealth> {
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
                issues.push(format!("Git status check failed: {}", e));
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
                custom_metrics: HashMap::new(),
            },
            last_check: SystemTime::now(),
            issues,
        })
    }

    async fn check_recovery_health(
        &self,
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
                custom_metrics: HashMap::new(),
            },
            last_check: SystemTime::now(),
            issues,
        })
    }

    async fn update_dashboard_metrics(&self, dashboard: &mut HealthDashboard) -> Result<()> {
        // Update metrics from processor summary
        let summary = self.processor.get_performance_summary().await?;

        dashboard.metrics.files_processed_total = summary.core_cache_files as u64;

        if let Some(persistent_stats) = &summary.persistent_cache_stats {
            dashboard.metrics.cache_hit_rate = persistent_stats.hit_rate();
        }

        dashboard.metrics.error_rate = summary.recent_error_rate;

        // Update Git metrics
        if let Some(git) = &self.git_integration {
            if let Ok(modified_files) = git.get_modified_files().await {
                dashboard.metrics.git_files_tracked = modified_files.len();
            }
        }

        // Update memory usage (placeholder)
        dashboard.metrics.memory_usage_mb = 128.0; // Would need actual memory tracking

        Ok(())
    }

    fn calculate_overall_status(&self, dashboard: &HealthDashboard) -> SystemHealthStatus {
        let component_scores: Vec<f64> = dashboard.components.values().map(|c| c.score).collect();

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

    pub async fn check_alerts(&self) -> Result<()> {
        let dashboard = self.dashboard_data.read().await;
        let mut new_alerts = Vec::new();

        for (component_name, component) in &dashboard.components {
            // Check CPU usage
            if component.metrics.cpu_usage > self.monitoring_config.alert_thresholds.cpu_critical {
                new_alerts.push(HealthAlert {
                    id: format!("cpu-critical-{}", component_name),
                    severity: AlertSeverity::Critical,
                    component: component_name.clone(),
                    message: format!("Critical CPU usage: {:.1}%", component.metrics.cpu_usage),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            } else if component.metrics.cpu_usage
                > self.monitoring_config.alert_thresholds.cpu_warning
            {
                new_alerts.push(HealthAlert {
                    id: format!("cpu-warning-{}", component_name),
                    severity: AlertSeverity::Warning,
                    component: component_name.clone(),
                    message: format!("High CPU usage: {:.1}%", component.metrics.cpu_usage),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            }

            // Check error rate
            if component.metrics.error_rate
                > self.monitoring_config.alert_thresholds.error_rate_critical
            {
                new_alerts.push(HealthAlert {
                    id: format!("error-critical-{}", component_name),
                    severity: AlertSeverity::Critical,
                    component: component_name.clone(),
                    message: format!("Critical error rate: {:.1}", component.metrics.error_rate),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            }
        }

        // Add new alerts to the dashboard
        if !new_alerts.is_empty() {
            drop(dashboard); // Release read lock
            let mut dashboard = self.dashboard_data.write().await;
            dashboard.alerts.extend(new_alerts);

            // Limit alert count
            let max_alerts = self.monitoring_config.max_alerts;
            if dashboard.alerts.len() > max_alerts {
                let alerts_len = dashboard.alerts.len();
                dashboard.alerts.drain(0..alerts_len - max_alerts);
            }
        }

        Ok(())
    }

    pub async fn generate_recommendations(&self) -> Result<()> {
        if !self.monitoring_config.enable_recommendations {
            return Ok(());
        }

        let dashboard = self.dashboard_data.read().await;
        let mut recommendations = Vec::new();

        // Analyze cache hit rate
        if dashboard.metrics.cache_hit_rate < 0.7 {
            recommendations.push(PerformanceRecommendation {
                id: "cache-hit-rate".to_string(),
                component: "cache".to_string(),
                recommendation: "Consider increasing cache size or adjusting TTL settings"
                    .to_string(),
                impact: ImpactLevel::High,
                effort: EffortLevel::Low,
                timestamp: SystemTime::now(),
            });
        }

        // Analyze error rate
        if dashboard.metrics.error_rate > 2.0 {
            recommendations.push(PerformanceRecommendation {
                id: "error-rate".to_string(),
                component: "processor".to_string(),
                recommendation: "Review error logs and implement additional error handling"
                    .to_string(),
                impact: ImpactLevel::High,
                effort: EffortLevel::Medium,
                timestamp: SystemTime::now(),
            });
        }

        // Update recommendations in dashboard
        if !recommendations.is_empty() {
            drop(dashboard); // Release read lock
            let mut dashboard = self.dashboard_data.write().await;
            dashboard.recommendations = recommendations;
        }

        Ok(())
    }

    async fn cleanup_old_data(&self) -> Result<()> {
        let retention_cutoff = SystemTime::now() - self.monitoring_config.retention_period;

        // Cleanup alert history
        {
            let mut alert_history = self.alert_history.write().await;
            alert_history.retain(|alert| alert.timestamp > retention_cutoff);
        }

        // Cleanup component history
        {
            let mut component_history = self.component_history.write().await;
            for history in component_history.values_mut() {
                history.retain(|entry| entry.last_check > retention_cutoff);

                // Limit history size
                if history.len() > self.monitoring_config.max_history_entries {
                    history.drain(0..history.len() - self.monitoring_config.max_history_entries);
                }
            }
        }

        Ok(())
    }

    pub async fn export_dashboard_json(&self) -> Result<String> {
        let dashboard = self.dashboard_data.read().await;
        Ok(serde_json::to_string_pretty(&*dashboard)?)
    }

    pub async fn export_metrics_prometheus(&self) -> Result<String> {
        let dashboard = self.dashboard_data.read().await;

        let mut prometheus_output = String::new();

        // Overall metrics
        prometheus_output.push_str(&format!(
            "# HELP lsp_bridge_files_processed_total Total files processed\n\
             # TYPE lsp_bridge_files_processed_total counter\n\
             lsp_bridge_files_processed_total {}\n",
            dashboard.metrics.files_processed_total
        ));

        prometheus_output.push_str(&format!(
            "# HELP lsp_bridge_cache_hit_rate Cache hit rate\n\
             # TYPE lsp_bridge_cache_hit_rate gauge\n\
             lsp_bridge_cache_hit_rate {}\n",
            dashboard.metrics.cache_hit_rate
        ));

        prometheus_output.push_str(&format!(
            "# HELP lsp_bridge_error_rate Error rate\n\
             # TYPE lsp_bridge_error_rate gauge\n\
             lsp_bridge_error_rate {}\n",
            dashboard.metrics.error_rate
        ));

        // Component metrics
        for (component_name, component) in &dashboard.components {
            prometheus_output.push_str(&format!(
                "# HELP lsp_bridge_component_health_score Component health score\n\
                 # TYPE lsp_bridge_component_health_score gauge\n\
                 lsp_bridge_component_health_score{{component=\"{}\"}} {}\n",
                component_name, component.score
            ));

            prometheus_output.push_str(&format!(
                "# HELP lsp_bridge_component_cpu_usage Component CPU usage\n\
                 # TYPE lsp_bridge_component_cpu_usage gauge\n\
                 lsp_bridge_component_cpu_usage{{component=\"{}\"}} {}\n",
                component_name, component.metrics.cpu_usage
            ));

            prometheus_output.push_str(&format!(
                "# HELP lsp_bridge_component_memory_usage Component memory usage\n\
                 # TYPE lsp_bridge_component_memory_usage gauge\n\
                 lsp_bridge_component_memory_usage{{component=\"{}\"}} {}\n",
                component_name, component.metrics.memory_usage
            ));
        }

        Ok(prometheus_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::SimpleEnhancedConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_health_monitor_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = SimpleEnhancedConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let processor = Arc::new(SimpleEnhancedProcessor::new(config).await?);
        let monitor = HealthMonitor::new(processor, None).await?;

        let dashboard = monitor.get_dashboard().await;
        assert!(matches!(
            dashboard.overall_status,
            SystemHealthStatus::Healthy
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_dashboard_updates() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = SimpleEnhancedConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let processor = Arc::new(SimpleEnhancedProcessor::new(config).await?);
        let monitor = HealthMonitor::new(processor, None).await?;

        // Update dashboard manually
        monitor.update_dashboard().await?;

        let dashboard = monitor.get_dashboard().await;
        assert!(!dashboard.components.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_export_formats() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = SimpleEnhancedConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let processor = Arc::new(SimpleEnhancedProcessor::new(config).await?);
        let monitor = HealthMonitor::new(processor, None).await?;

        // Test JSON export
        let json_output = monitor.export_dashboard_json().await?;
        assert!(!json_output.is_empty());

        // Test Prometheus export
        let prometheus_output = monitor.export_metrics_prometheus().await?;
        assert!(!prometheus_output.is_empty());

        Ok(())
    }
}
