use crate::core::health_dashboard::types::HealthDashboard;
use anyhow::Result;

pub struct DashboardRenderer;

impl DashboardRenderer {
    /// Export dashboard as JSON
    pub fn render_json(dashboard: &HealthDashboard) -> Result<String> {
        Ok(serde_json::to_string_pretty(dashboard)?)
    }

    /// Export metrics in Prometheus format
    pub fn render_prometheus(dashboard: &HealthDashboard) -> String {
        let mut output = String::new();

        // Overall metrics
        output.push_str(&format!(
            "# HELP lsp_bridge_files_processed_total Total files processed\n\
             # TYPE lsp_bridge_files_processed_total counter\n\
             lsp_bridge_files_processed_total {}\n",
            dashboard.metrics.files_processed_total
        ));

        output.push_str(&format!(
            "# HELP lsp_bridge_cache_hit_rate Cache hit rate\n\
             # TYPE lsp_bridge_cache_hit_rate gauge\n\
             lsp_bridge_cache_hit_rate {}\n",
            dashboard.metrics.cache_hit_rate
        ));

        output.push_str(&format!(
            "# HELP lsp_bridge_error_rate Error rate\n\
             # TYPE lsp_bridge_error_rate gauge\n\
             lsp_bridge_error_rate {}\n",
            dashboard.metrics.error_rate
        ));

        output.push_str(&format!(
            "# HELP lsp_bridge_memory_usage_mb Memory usage in MB\n\
             # TYPE lsp_bridge_memory_usage_mb gauge\n\
             lsp_bridge_memory_usage_mb {}\n",
            dashboard.metrics.memory_usage_mb
        ));

        output.push_str(&format!(
            "# HELP lsp_bridge_uptime_seconds Uptime in seconds\n\
             # TYPE lsp_bridge_uptime_seconds counter\n\
             lsp_bridge_uptime_seconds {}\n",
            dashboard.metrics.uptime.as_secs()
        ));

        // Component metrics
        for (component_name, component) in &dashboard.components {
            output.push_str(&format!(
                "# HELP lsp_bridge_component_health_score Component health score\n\
                 # TYPE lsp_bridge_component_health_score gauge\n\
                 lsp_bridge_component_health_score{{component=\"{}\"}} {}\n",
                component_name, component.score
            ));

            output.push_str(&format!(
                "# HELP lsp_bridge_component_cpu_usage Component CPU usage\n\
                 # TYPE lsp_bridge_component_cpu_usage gauge\n\
                 lsp_bridge_component_cpu_usage{{component=\"{}\"}} {}\n",
                component_name, component.metrics.cpu_usage
            ));

            output.push_str(&format!(
                "# HELP lsp_bridge_component_memory_usage Component memory usage\n\
                 # TYPE lsp_bridge_component_memory_usage gauge\n\
                 lsp_bridge_component_memory_usage{{component=\"{}\"}} {}\n",
                component_name, component.metrics.memory_usage
            ));

            output.push_str(&format!(
                "# HELP lsp_bridge_component_error_rate Component error rate\n\
                 # TYPE lsp_bridge_component_error_rate gauge\n\
                 lsp_bridge_component_error_rate{{component=\"{}\"}} {}\n",
                component_name, component.metrics.error_rate
            ));

            output.push_str(&format!(
                "# HELP lsp_bridge_component_throughput Component throughput\n\
                 # TYPE lsp_bridge_component_throughput gauge\n\
                 lsp_bridge_component_throughput{{component=\"{}\"}} {}\n",
                component_name, component.metrics.throughput
            ));
        }

        // Alert metrics
        let active_alerts = dashboard.alerts.iter().filter(|a| !a.resolved).count();
        output.push_str(&format!(
            "# HELP lsp_bridge_active_alerts_total Number of active alerts\n\
             # TYPE lsp_bridge_active_alerts_total gauge\n\
             lsp_bridge_active_alerts_total {active_alerts}\n"
        ));

        output
    }

    /// Render dashboard as HTML (for web UI)
    pub fn render_html(_dashboard: &HealthDashboard) -> String {
        // Placeholder for HTML rendering
        // In a real implementation, this would use a templating engine
        "<html><body><h1>Health Dashboard</h1><p>Not implemented</p></body></html>".to_string()
    }

    /// Render dashboard as terminal-friendly text
    pub fn render_text(dashboard: &HealthDashboard) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "=== LSP Bridge Health Dashboard ===\n\
             Time: {:?}\n\
             Status: {:?}\n\
             Uptime: {:?}\n\n",
            dashboard.timestamp, dashboard.overall_status, dashboard.metrics.uptime
        ));

        output.push_str("=== Metrics ===\n");
        output.push_str(&format!(
            "Files Processed: {}\n\
             Cache Hit Rate: {:.1}%\n\
             Error Rate: {:.1}%\n\
             Memory Usage: {:.1} MB\n\n",
            dashboard.metrics.files_processed_total,
            dashboard.metrics.cache_hit_rate * 100.0,
            dashboard.metrics.error_rate,
            dashboard.metrics.memory_usage_mb
        ));

        output.push_str("=== Components ===\n");
        for (name, component) in &dashboard.components {
            output.push_str(&format!(
                "{}: {:?} (Score: {:.1})\n",
                name, component.status, component.score
            ));
            if !component.issues.is_empty() {
                output.push_str("  Issues:\n");
                for issue in &component.issues {
                    output.push_str(&format!("    - {issue}\n"));
                }
            }
        }

        if !dashboard.alerts.is_empty() {
            output.push_str("\n=== Active Alerts ===\n");
            for alert in dashboard.alerts.iter().filter(|a| !a.resolved) {
                output.push_str(&format!(
                    "[{:?}] {}: {}\n",
                    alert.severity, alert.component, alert.message
                ));
            }
        }

        if !dashboard.recommendations.is_empty() {
            output.push_str("\n=== Recommendations ===\n");
            for rec in &dashboard.recommendations {
                output.push_str(&format!(
                    "- {}: {} (Impact: {:?}, Effort: {:?})\n",
                    rec.component, rec.recommendation, rec.impact, rec.effort
                ));
            }
        }

        output
    }
}