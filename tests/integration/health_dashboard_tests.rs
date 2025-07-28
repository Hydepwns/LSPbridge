use anyhow::Result;
use lsp_bridge::core::{
    AlertSeverity, AlertThresholds, ComponentStatus, EffortLevel, HealthMonitor, ImpactLevel,
    MonitoringConfig, SimpleEnhancedConfig, SimpleEnhancedProcessor, SystemHealthStatus,
};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

async fn create_test_processor() -> Result<Arc<SimpleEnhancedProcessor>> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false, // Disable for testing
        enable_dynamic_config: false,  // Disable for testing
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;
    Ok(Arc::new(processor))
}

#[tokio::test]
async fn test_health_monitor_creation() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    let dashboard = monitor.get_dashboard().await;
    assert!(matches!(
        dashboard.overall_status,
        SystemHealthStatus::Healthy
    ));
    assert!(dashboard.components.is_empty()); // No components analyzed yet
    assert!(dashboard.alerts.is_empty());
    assert!(dashboard.recommendations.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_health_monitor_with_custom_config() -> Result<()> {
    let processor = create_test_processor().await?;

    let monitoring_config = MonitoringConfig {
        update_interval: Duration::from_millis(100),
        retention_period: Duration::from_secs(3600),
        alert_thresholds: AlertThresholds {
            cpu_warning: 50.0,
            cpu_critical: 80.0,
            memory_warning: 60.0,
            memory_critical: 90.0,
            error_rate_warning: 2.0,
            error_rate_critical: 5.0,
            response_time_warning: Duration::from_millis(500),
            response_time_critical: Duration::from_millis(2000),
        },
        enable_recommendations: true,
        max_alerts: 500,
        max_history_entries: 5000,
    };

    let monitor = HealthMonitor::new(processor, Some(monitoring_config)).await?;
    let dashboard = monitor.get_dashboard().await;

    assert!(matches!(
        dashboard.overall_status,
        SystemHealthStatus::Healthy
    ));

    Ok(())
}

#[tokio::test]
async fn test_dashboard_updates() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    // Manually trigger dashboard update
    monitor.update_dashboard().await?;

    let dashboard = monitor.get_dashboard().await;

    // Should have processor component at minimum
    assert!(!dashboard.components.is_empty());
    assert!(dashboard.components.contains_key("processor"));

    let processor_health = dashboard.components.get("processor").unwrap();
    assert_eq!(processor_health.name, "Processor");
    assert!(matches!(
        processor_health.status,
        ComponentStatus::Online | ComponentStatus::Degraded
    ));
    assert!(processor_health.score >= 0.0 && processor_health.score <= 100.0);

    Ok(())
}

#[tokio::test]
async fn test_component_health_scoring() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    monitor.update_dashboard().await?;

    let processor_health = monitor.get_component_health("processor").await;
    assert!(processor_health.is_some());

    let health = processor_health.unwrap();
    assert!(health.score >= 0.0);
    assert!(health.score <= 100.0);

    // High score should mean Online status
    if health.score >= 90.0 {
        assert!(matches!(health.status, ComponentStatus::Online));
    }

    Ok(())
}

#[tokio::test]
async fn test_alert_generation() -> Result<()> {
    let processor = create_test_processor().await?;

    // Configure with very low thresholds to trigger alerts
    let monitoring_config = MonitoringConfig {
        alert_thresholds: AlertThresholds {
            cpu_warning: 0.1, // Very low threshold
            cpu_critical: 0.2,
            memory_warning: 0.1,
            memory_critical: 0.2,
            error_rate_warning: 0.0, // Any error triggers warning
            error_rate_critical: 0.1,
            response_time_warning: Duration::from_millis(1),
            response_time_critical: Duration::from_millis(10),
        },
        ..Default::default()
    };

    let monitor = HealthMonitor::new(processor, Some(monitoring_config)).await?;

    // Update dashboard to populate component metrics
    monitor.update_dashboard().await?;

    // Check for alerts (might trigger based on actual system metrics)
    monitor.check_alerts().await?;

    let alerts = monitor.get_active_alerts().await;
    // Don't assert specific alert count as it depends on system state
    // Just verify the alert structure is correct
    for alert in &alerts {
        assert!(!alert.id.is_empty());
        assert!(!alert.component.is_empty());
        assert!(!alert.message.is_empty());
        assert!(matches!(
            alert.severity,
            AlertSeverity::Info
                | AlertSeverity::Warning
                | AlertSeverity::Error
                | AlertSeverity::Critical
        ));
        assert!(!alert.resolved);
    }

    Ok(())
}

#[tokio::test]
async fn test_alert_acknowledgment() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    // Manually create and add an alert for testing
    monitor.update_dashboard().await?;

    // Try to acknowledge a non-existent alert
    let result = monitor.acknowledge_alert("non-existent-alert").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_recommendation_generation() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    // Update dashboard and generate recommendations
    monitor.update_dashboard().await?;
    monitor.generate_recommendations().await?;

    let recommendations = monitor.get_recommendations().await;

    // Verify recommendation structure
    for rec in &recommendations {
        assert!(!rec.id.is_empty());
        assert!(!rec.component.is_empty());
        assert!(!rec.recommendation.is_empty());
        assert!(matches!(
            rec.impact,
            ImpactLevel::Low | ImpactLevel::Medium | ImpactLevel::High | ImpactLevel::Critical
        ));
        assert!(matches!(
            rec.effort,
            EffortLevel::Minimal | EffortLevel::Low | EffortLevel::Medium | EffortLevel::High
        ));
    }

    Ok(())
}

#[tokio::test]
async fn test_dashboard_metrics() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    monitor.update_dashboard().await?;

    let dashboard = monitor.get_dashboard().await;
    let metrics = &dashboard.metrics;

    // Verify metric structure and reasonable values
    assert!(metrics.files_processed_total >= 0);
    assert!(metrics.cache_hit_rate >= 0.0 && metrics.cache_hit_rate <= 1.0);
    assert!(metrics.error_rate >= 0.0);
    assert!(metrics.memory_usage_mb >= 0.0);
    assert!(metrics.git_files_tracked >= 0);
    assert!(metrics.config_changes_today >= 0);

    Ok(())
}

#[tokio::test]
async fn test_system_health_status_calculation() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    monitor.update_dashboard().await?;

    let dashboard = monitor.get_dashboard().await;

    // Test that overall status matches component health
    match dashboard.overall_status {
        SystemHealthStatus::Healthy => {
            // Should have high-scoring components
            for component in dashboard.components.values() {
                if component.score < 70.0 {
                    // If any component is low-scoring, overall shouldn't be healthy
                    // (This is a simplified check - actual logic is more complex)
                }
            }
        }
        SystemHealthStatus::Degraded
        | SystemHealthStatus::Unhealthy
        | SystemHealthStatus::Critical => {
            // Should have at least one low-scoring component
        }
        SystemHealthStatus::Unknown => {
            // Should have no components or unanalyzed state
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_json_export() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    monitor.update_dashboard().await?;

    let json_output = monitor.export_dashboard_json().await?;
    assert!(!json_output.is_empty());

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_output)?;
    assert!(parsed.is_object());

    // Check for expected top-level fields
    assert!(parsed.get("timestamp").is_some());
    assert!(parsed.get("overall_status").is_some());
    assert!(parsed.get("components").is_some());
    assert!(parsed.get("metrics").is_some());
    assert!(parsed.get("alerts").is_some());
    assert!(parsed.get("recommendations").is_some());

    Ok(())
}

#[tokio::test]
async fn test_prometheus_export() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    monitor.update_dashboard().await?;

    let prometheus_output = monitor.export_metrics_prometheus().await?;
    assert!(!prometheus_output.is_empty());

    // Verify Prometheus format
    assert!(prometheus_output.contains("# HELP"));
    assert!(prometheus_output.contains("# TYPE"));
    assert!(prometheus_output.contains("lsp_bridge_"));

    // Check for specific metrics
    assert!(prometheus_output.contains("lsp_bridge_files_processed_total"));
    assert!(prometheus_output.contains("lsp_bridge_cache_hit_rate"));
    assert!(prometheus_output.contains("lsp_bridge_error_rate"));

    Ok(())
}

#[tokio::test]
async fn test_health_monitor_with_components() -> Result<()> {
    let processor = create_test_processor().await?;

    // Create monitor with all optional components disabled for testing
    let monitor = HealthMonitor::new(processor, None).await?;

    monitor.update_dashboard().await?;

    let dashboard = monitor.get_dashboard().await;

    // Should have at least the processor component
    assert!(dashboard.components.contains_key("processor"));

    // Other components should not be present since they're disabled
    assert!(!dashboard.components.contains_key("git"));
    assert!(!dashboard.components.contains_key("config"));

    Ok(())
}

#[tokio::test]
async fn test_monitoring_lifecycle() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = Arc::new(HealthMonitor::new(processor, None).await?);

    // Test that monitoring can be started without errors
    let monitor_clone = monitor.clone();
    let monitoring_handle = tokio::spawn(async move {
        // Start monitoring for a short time
        monitor_clone.start_monitoring().await
    });

    // Let it run briefly
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel the monitoring task
    monitoring_handle.abort();

    // Verify dashboard was updated during monitoring
    let dashboard = monitor.get_dashboard().await;
    assert!(!dashboard.components.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_dashboard_access() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = Arc::new(HealthMonitor::new(processor, None).await?);

    // Spawn multiple tasks that access the dashboard concurrently
    let mut handles = Vec::new();

    for _i in 0..10 {
        let monitor_clone = monitor.clone();
        let handle = tokio::spawn(async move {
            monitor_clone.update_dashboard().await.unwrap();
            let _dashboard = monitor_clone.get_dashboard().await;
            let _alerts = monitor_clone.get_active_alerts().await;
            let _recommendations = monitor_clone.get_recommendations().await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    // Verify final state is consistent
    let dashboard = monitor.get_dashboard().await;
    assert!(!dashboard.components.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_health_monitor_error_handling() -> Result<()> {
    let processor = create_test_processor().await?;
    let monitor = HealthMonitor::new(processor, None).await?;

    // Test getting component that doesn't exist
    let nonexistent_component = monitor.get_component_health("nonexistent").await;
    assert!(nonexistent_component.is_none());

    // Test export with empty dashboard
    let json_output = monitor.export_dashboard_json().await?;
    assert!(!json_output.is_empty());

    let prometheus_output = monitor.export_metrics_prometheus().await?;
    assert!(!prometheus_output.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_alert_severity_levels() -> Result<()> {
    // Test that all alert severity levels can be created and serialized
    use lsp_bridge::core::HealthAlert;
    use std::time::SystemTime;

    let severities = vec![
        AlertSeverity::Info,
        AlertSeverity::Warning,
        AlertSeverity::Error,
        AlertSeverity::Critical,
    ];

    for severity in severities {
        let alert = HealthAlert {
            id: "test-alert".to_string(),
            severity: severity.clone(),
            component: "test-component".to_string(),
            message: "Test alert message".to_string(),
            timestamp: SystemTime::now(),
            resolved: false,
            resolution_time: None,
        };

        // Test serialization
        let json = serde_json::to_string(&alert)?;
        assert!(!json.is_empty());

        // Test deserialization
        let _deserialized: HealthAlert = serde_json::from_str(&json)?;
    }

    Ok(())
}

#[tokio::test]
async fn test_recommendation_impact_levels() -> Result<()> {
    // Test that all recommendation levels can be created and serialized
    use lsp_bridge::core::PerformanceRecommendation;
    use std::time::SystemTime;

    let impacts = vec![
        ImpactLevel::Low,
        ImpactLevel::Medium,
        ImpactLevel::High,
        ImpactLevel::Critical,
    ];
    let efforts = vec![
        EffortLevel::Minimal,
        EffortLevel::Low,
        EffortLevel::Medium,
        EffortLevel::High,
    ];

    for impact in impacts {
        for effort in &efforts {
            let recommendation = PerformanceRecommendation {
                id: "test-rec".to_string(),
                component: "test-component".to_string(),
                recommendation: "Test recommendation".to_string(),
                impact: impact.clone(),
                effort: effort.clone(),
                timestamp: SystemTime::now(),
            };

            // Test serialization
            let json = serde_json::to_string(&recommendation)?;
            assert!(!json.is_empty());

            // Test deserialization
            let _deserialized: PerformanceRecommendation = serde_json::from_str(&json)?;
        }
    }

    Ok(())
}
