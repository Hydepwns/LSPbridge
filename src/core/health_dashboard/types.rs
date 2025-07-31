use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

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