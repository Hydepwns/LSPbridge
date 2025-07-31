//! # Health Dashboard
//!
//! This module provides comprehensive health monitoring and visualization capabilities
//! for the LSP Bridge system, including metrics collection, alert management, and
//! performance recommendations.
//!
//! ## Key Components
//!
//! - **HealthMonitor**: Main monitoring engine that coordinates all health checks
//! - **MetricsCollector**: Collects health metrics from various system components
//! - **AlertRulesEngine**: Evaluates metrics against thresholds and generates alerts
//! - **DashboardRenderer**: Exports health data in various formats (JSON, Prometheus, etc.)

pub mod alerts;
pub mod metrics;
pub mod types;
pub mod visualization;

pub use types::*;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::core::{
    DynamicConfigManager, ErrorRecoverySystem, GitIntegration, MetricsCollector as CoreMetricsCollector,
    SimpleEnhancedProcessor,
};

use alerts::{AlertNotifier, AlertRulesEngine};
use metrics::{MetricsAggregator, MetricsCollector};
use visualization::{DashboardComponents, DashboardRenderer};

pub struct HealthMonitor {
    processor: Arc<SimpleEnhancedProcessor>,
    metrics_collector: Option<Arc<CoreMetricsCollector>>,
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
    
    // Components
    alert_engine: AlertRulesEngine,
}

impl HealthMonitor {
    pub async fn new(
        processor: Arc<SimpleEnhancedProcessor>,
        config: Option<MonitoringConfig>,
    ) -> Result<Self> {
        let monitoring_config = config.unwrap_or_default();
        let alert_engine = AlertRulesEngine::new(monitoring_config.alert_thresholds.clone());

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
            metrics_collector: None,
            config_manager: None,
            git_integration: None,
            error_recovery: None,
            dashboard_data: Arc::new(RwLock::new(initial_dashboard)),
            start_time: Instant::now(),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            component_history: Arc::new(RwLock::new(HashMap::new())),
            monitoring_config,
            alert_engine,
        };

        info!("Health monitor initialized");
        Ok(monitor)
    }

    // Builder pattern methods
    pub fn with_metrics_collector(mut self, collector: Arc<CoreMetricsCollector>) -> Self {
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

    /// Start the monitoring loop
    pub async fn start_monitoring(self: Arc<Self>) -> Result<()> {
        info!("Starting health monitoring");

        let update_interval = self.monitoring_config.update_interval;

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

    // Public API methods
    pub async fn get_dashboard(&self) -> HealthDashboard {
        self.dashboard_data.read().await.clone()
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

    // Dashboard update methods
    async fn update_dashboard(&self) -> Result<()> {
        let mut dashboard = self.dashboard_data.write().await;

        // Update timestamp and uptime
        dashboard.timestamp = SystemTime::now();
        dashboard.metrics.uptime = self.start_time.elapsed();

        // Update component health
        self.update_component_health(&mut dashboard).await?;

        // Update overall metrics
        self.update_dashboard_metrics(&mut dashboard).await?;

        // Calculate overall status
        dashboard.overall_status = MetricsAggregator::calculate_overall_status(&dashboard.components);

        debug!("Dashboard updated");
        Ok(())
    }

    async fn update_component_health(&self, dashboard: &mut HealthDashboard) -> Result<()> {
        // Core processor health
        let processor_health = MetricsCollector::collect_processor_health(&self.processor).await?;
        dashboard
            .components
            .insert("processor".to_string(), processor_health);

        // Metrics collector health
        if let Some(metrics) = &self.metrics_collector {
            let metrics_health = MetricsCollector::collect_metrics_health(metrics).await?;
            dashboard
                .components
                .insert("metrics".to_string(), metrics_health);
        }

        // Configuration manager health
        if let Some(config) = &self.config_manager {
            let config_health = MetricsCollector::collect_config_health(config).await?;
            dashboard
                .components
                .insert("config".to_string(), config_health);
        }

        // Git integration health
        if let Some(git) = &self.git_integration {
            let git_health = MetricsCollector::collect_git_health(git).await?;
            dashboard.components.insert("git".to_string(), git_health);
        }

        // Error recovery health
        if let Some(recovery) = &self.error_recovery {
            let recovery_health = MetricsCollector::collect_recovery_health(recovery).await?;
            dashboard
                .components
                .insert("error_recovery".to_string(), recovery_health);
        }

        Ok(())
    }

    async fn update_dashboard_metrics(&self, dashboard: &mut HealthDashboard) -> Result<()> {
        dashboard.metrics = MetricsAggregator::aggregate_dashboard_metrics(
            &self.processor,
            self.git_integration.as_deref(),
            self.start_time.elapsed(),
        )
        .await?;

        Ok(())
    }

    async fn check_alerts(&self) -> Result<()> {
        let dashboard = self.dashboard_data.read().await;
        let new_alerts = self.alert_engine.check_components(&dashboard.components);

        if !new_alerts.is_empty() {
            // Notify about new alerts
            AlertNotifier::notify_alerts(&new_alerts);

            // Add to dashboard
            drop(dashboard); // Release read lock
            let mut dashboard = self.dashboard_data.write().await;
            AlertRulesEngine::merge_alerts(
                &mut dashboard.alerts,
                new_alerts,
                self.monitoring_config.max_alerts,
            );
        }

        Ok(())
    }

    async fn generate_recommendations(&self) -> Result<()> {
        if !self.monitoring_config.enable_recommendations {
            return Ok(());
        }

        let dashboard = self.dashboard_data.read().await;
        let recommendations = DashboardComponents::generate_recommendations(&dashboard.metrics);

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

                if history.len() > self.monitoring_config.max_history_entries {
                    history.drain(0..history.len() - self.monitoring_config.max_history_entries);
                }
            }
        }

        Ok(())
    }

    // Export methods
    pub async fn export_dashboard_json(&self) -> Result<String> {
        let dashboard = self.dashboard_data.read().await;
        DashboardRenderer::render_json(&dashboard)
    }

    pub async fn export_metrics_prometheus(&self) -> Result<String> {
        let dashboard = self.dashboard_data.read().await;
        Ok(DashboardRenderer::render_prometheus(&dashboard))
    }

    pub async fn export_dashboard_text(&self) -> Result<String> {
        let dashboard = self.dashboard_data.read().await;
        Ok(DashboardRenderer::render_text(&dashboard))
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

        let json_output = monitor.export_dashboard_json().await?;
        assert!(!json_output.is_empty());

        let prometheus_output = monitor.export_metrics_prometheus().await?;
        assert!(!prometheus_output.is_empty());

        let text_output = monitor.export_dashboard_text().await?;
        assert!(!text_output.is_empty());

        Ok(())
    }
}