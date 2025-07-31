use crate::core::health_dashboard::types::{
    AlertSeverity, AlertThresholds, ComponentHealthMap, HealthAlert,
};
use std::time::SystemTime;

pub struct AlertRulesEngine {
    thresholds: AlertThresholds,
}

impl AlertRulesEngine {
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self { thresholds }
    }

    /// Check component health against alert thresholds and generate alerts
    pub fn check_components(&self, components: &ComponentHealthMap) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();

        for (component_name, component) in components {
            // Check CPU usage
            if component.metrics.cpu_usage > self.thresholds.cpu_critical {
                alerts.push(HealthAlert {
                    id: format!("cpu-critical-{}", component_name),
                    severity: AlertSeverity::Critical,
                    component: component_name.clone(),
                    message: format!("Critical CPU usage: {:.1}%", component.metrics.cpu_usage),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            } else if component.metrics.cpu_usage > self.thresholds.cpu_warning {
                alerts.push(HealthAlert {
                    id: format!("cpu-warning-{}", component_name),
                    severity: AlertSeverity::Warning,
                    component: component_name.clone(),
                    message: format!("High CPU usage: {:.1}%", component.metrics.cpu_usage),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            }

            // Check memory usage
            if component.metrics.memory_usage > self.thresholds.memory_critical {
                alerts.push(HealthAlert {
                    id: format!("memory-critical-{}", component_name),
                    severity: AlertSeverity::Critical,
                    component: component_name.clone(),
                    message: format!(
                        "Critical memory usage: {:.1}%",
                        component.metrics.memory_usage
                    ),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            } else if component.metrics.memory_usage > self.thresholds.memory_warning {
                alerts.push(HealthAlert {
                    id: format!("memory-warning-{}", component_name),
                    severity: AlertSeverity::Warning,
                    component: component_name.clone(),
                    message: format!("High memory usage: {:.1}%", component.metrics.memory_usage),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            }

            // Check error rate
            if component.metrics.error_rate > self.thresholds.error_rate_critical {
                alerts.push(HealthAlert {
                    id: format!("error-critical-{}", component_name),
                    severity: AlertSeverity::Critical,
                    component: component_name.clone(),
                    message: format!("Critical error rate: {:.1}", component.metrics.error_rate),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            } else if component.metrics.error_rate > self.thresholds.error_rate_warning {
                alerts.push(HealthAlert {
                    id: format!("error-warning-{}", component_name),
                    severity: AlertSeverity::Warning,
                    component: component_name.clone(),
                    message: format!("High error rate: {:.1}", component.metrics.error_rate),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            }

            // Check response time
            if component.metrics.response_time > self.thresholds.response_time_critical {
                alerts.push(HealthAlert {
                    id: format!("response-critical-{}", component_name),
                    severity: AlertSeverity::Critical,
                    component: component_name.clone(),
                    message: format!(
                        "Critical response time: {:?}",
                        component.metrics.response_time
                    ),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            } else if component.metrics.response_time > self.thresholds.response_time_warning {
                alerts.push(HealthAlert {
                    id: format!("response-warning-{}", component_name),
                    severity: AlertSeverity::Warning,
                    component: component_name.clone(),
                    message: format!("High response time: {:?}", component.metrics.response_time),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    resolution_time: None,
                });
            }
        }

        alerts
    }

    /// Merge new alerts with existing ones, avoiding duplicates
    pub fn merge_alerts(
        existing: &mut Vec<HealthAlert>,
        new_alerts: Vec<HealthAlert>,
        max_alerts: usize,
    ) {
        for new_alert in new_alerts {
            // Check if this alert already exists
            let exists = existing.iter().any(|a| a.id == new_alert.id && !a.resolved);
            if !exists {
                existing.push(new_alert);
            }
        }

        // Limit alert count
        if existing.len() > max_alerts {
            let excess = existing.len() - max_alerts;
            existing.drain(0..excess);
        }
    }
}