use anyhow::Result;
use lsp_bridge::core::DynamicConfigManager;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

#[tokio::test]
async fn test_config_file_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("test_config.toml");

    // Config file doesn't exist yet
    assert!(!config_file.exists());

    let manager = DynamicConfigManager::new(config_file.clone()).await?;

    // Config file should be created with defaults
    assert!(config_file.exists());

    let config = manager.get_config().await;
    assert_eq!(config.processing.parallel_processing, true);
    assert_eq!(config.processing.chunk_size, 100);
    assert_eq!(config.memory.max_memory_mb, 256);

    Ok(())
}

#[tokio::test]
async fn test_config_loading_from_existing_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("existing_config.toml");

    // Create a custom config file
    let custom_config = r#"
[processing]
parallel_processing = false
chunk_size = 50
max_concurrent_files = 500
file_size_limit_mb = 5
timeout_seconds = 15

[cache]
enable_persistent_cache = false
enable_memory_cache = true
cache_dir = "/tmp/test_cache"
max_size_mb = 50
max_entries = 1000
ttl_hours = 12
cleanup_interval_minutes = 30

[memory]
max_memory_mb = 128
max_entries = 10000
eviction_policy = "LRU"
high_water_mark = 0.75
low_water_mark = 0.5
eviction_batch_size = 50
monitoring_interval_seconds = 60

[error_recovery]
enable_circuit_breaker = true
max_retries = 2
initial_delay_ms = 50
max_delay_ms = 2000
backoff_multiplier = 1.5
failure_threshold = 3
success_threshold = 2
timeout_ms = 5000

[git]
enable_git_integration = false
scan_interval_seconds = 60
ignore_untracked = true
track_staged_changes = false
auto_refresh = false

[metrics]
enable_metrics = false
prometheus_port = 8080
collection_interval_seconds = 30
retention_hours = 24
export_format = "json"

[features]
auto_optimization = false
health_monitoring = false
cache_warming = false
advanced_diagnostics = false
experimental_features = false

[performance]
optimization_interval_minutes = 30
health_check_interval_minutes = 10
gc_threshold_mb = 256
max_cpu_usage_percent = 75.0
adaptive_scaling = false
"#;

    fs::write(&config_file, custom_config)?;

    let manager = DynamicConfigManager::new(config_file).await?;
    let config = manager.get_config().await;

    assert_eq!(config.processing.parallel_processing, false);
    assert_eq!(config.processing.chunk_size, 50);
    assert_eq!(config.cache.enable_persistent_cache, false);
    assert_eq!(config.cache.enable_memory_cache, true);
    assert_eq!(config.cache.max_size_mb, 50);
    assert_eq!(config.memory.max_memory_mb, 128);
    assert_eq!(config.memory.eviction_policy, "LRU");
    assert_eq!(config.git.enable_git_integration, false);
    assert_eq!(config.metrics.enable_metrics, false);

    Ok(())
}

#[tokio::test]
async fn test_config_updates() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("update_test.toml");

    let manager = DynamicConfigManager::new(config_file).await?;

    // Test single field update
    let changes = manager
        .update_config(|config| {
            config.processing.chunk_size = 200;
            Ok(())
        })
        .await?;

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field_path, "processing.chunk_size");
    assert_eq!(changes[0].old_value, "100");
    assert_eq!(changes[0].new_value, "200");

    let config = manager.get_config().await;
    assert_eq!(config.processing.chunk_size, 200);

    Ok(())
}

#[tokio::test]
async fn test_config_multiple_updates() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("multi_update.toml");

    let manager = DynamicConfigManager::new(config_file).await?;

    // Test multiple field updates
    let changes = manager
        .update_config(|config| {
            config.processing.parallel_processing = false;
            config.memory.max_memory_mb = 512;
            config.features.auto_optimization = false;
            Ok(())
        })
        .await?;

    assert_eq!(changes.len(), 3);

    // Verify all changes are recorded
    let field_paths: Vec<String> = changes.iter().map(|c| c.field_path.clone()).collect();
    assert!(field_paths.contains(&"processing.parallel_processing".to_string()));
    assert!(field_paths.contains(&"memory.max_memory_mb".to_string()));
    assert!(field_paths.contains(&"features.auto_optimization".to_string()));

    let config = manager.get_config().await;
    assert_eq!(config.processing.parallel_processing, false);
    assert_eq!(config.memory.max_memory_mb, 512);
    assert_eq!(config.features.auto_optimization, false);

    Ok(())
}

#[tokio::test]
async fn test_field_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("field_ops.toml");

    let manager = DynamicConfigManager::new(config_file).await?;

    // Test get field value
    let value = manager.get_field_value("memory.max_memory_mb").await?;
    assert_eq!(value, "256");

    let value = manager
        .get_field_value("processing.parallel_processing")
        .await?;
    assert_eq!(value, "true");

    // Test set field value
    let change = manager
        .set_field_value("memory.max_memory_mb", "512")
        .await?;
    assert_eq!(change.field_path, "memory.max_memory_mb");
    assert_eq!(change.old_value, "256");
    assert_eq!(change.new_value, "512");

    // Verify the change
    let new_value = manager.get_field_value("memory.max_memory_mb").await?;
    assert_eq!(new_value, "512");

    Ok(())
}

#[tokio::test]
async fn test_config_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("validation_test.toml");

    let manager = DynamicConfigManager::new(config_file).await?;

    // Test invalid memory limit (too low)
    let result = manager.set_field_value("memory.max_memory_mb", "32").await;
    assert!(result.is_err());

    // Test invalid port number (too low)
    let result = manager
        .set_field_value("metrics.prometheus_port", "100")
        .await;
    assert!(result.is_err());

    // Test invalid CPU percentage (too high)
    let result = manager
        .set_field_value("performance.max_cpu_usage_percent", "150.0")
        .await;
    assert!(result.is_err());

    // Test valid values
    let result = manager.set_field_value("memory.max_memory_mb", "512").await;
    assert!(result.is_ok());

    let result = manager
        .set_field_value("metrics.prometheus_port", "8080")
        .await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_config_change_notifications() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("notifications.toml");

    let manager = DynamicConfigManager::new(config_file).await?;
    let mut receiver = manager.subscribe_to_changes().await;

    // Make a change
    let _changes = manager
        .update_config(|config| {
            config.processing.chunk_size = 150;
            Ok(())
        })
        .await?;

    // Wait for notification
    let change = timeout(Duration::from_secs(1), receiver.recv()).await??;
    assert_eq!(change.field_path, "processing.chunk_size");
    assert_eq!(change.old_value, "100");
    assert_eq!(change.new_value, "150");

    Ok(())
}

#[tokio::test]
async fn test_config_file_reload() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("reload_test.toml");

    let manager = DynamicConfigManager::new(config_file.clone()).await?;

    // Manually modify the config file
    let modified_config = r#"
[processing]
parallel_processing = false
chunk_size = 75
max_concurrent_files = 500
file_size_limit_mb = 5
timeout_seconds = 15

[cache]
enable_persistent_cache = true
enable_memory_cache = true
cache_dir = "/tmp/test_cache"
max_size_mb = 100
max_entries = 10000
ttl_hours = 24
cleanup_interval_minutes = 60

[memory]
max_memory_mb = 1024
max_entries = 50000
eviction_policy = "Adaptive"
high_water_mark = 0.8
low_water_mark = 0.6
eviction_batch_size = 100
monitoring_interval_seconds = 30

[error_recovery]
enable_circuit_breaker = true
max_retries = 3
initial_delay_ms = 100
max_delay_ms = 5000
backoff_multiplier = 2.0
failure_threshold = 5
success_threshold = 3
timeout_ms = 10000

[git]
enable_git_integration = true
scan_interval_seconds = 30
ignore_untracked = false
track_staged_changes = true
auto_refresh = true

[metrics]
enable_metrics = true
prometheus_port = 9090
collection_interval_seconds = 10
retention_hours = 72
export_format = "prometheus"

[features]
auto_optimization = true
health_monitoring = true
cache_warming = true
advanced_diagnostics = false
experimental_features = false

[performance]
optimization_interval_minutes = 60
health_check_interval_minutes = 5
gc_threshold_mb = 512
max_cpu_usage_percent = 80.0
adaptive_scaling = true
"#;

    fs::write(&config_file, modified_config)?;

    // Reload from file
    let changes = manager.reload_from_file().await?;

    // Should detect multiple changes
    assert!(!changes.is_empty());

    let config = manager.get_config().await;
    assert_eq!(config.processing.parallel_processing, false);
    assert_eq!(config.processing.chunk_size, 75);
    assert_eq!(config.memory.max_memory_mb, 1024);

    Ok(())
}

#[tokio::test]
async fn test_config_save() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("save_test.toml");

    let manager = DynamicConfigManager::new(config_file.clone()).await?;

    // Make some changes
    manager
        .update_config(|config| {
            config.processing.chunk_size = 300;
            config.memory.max_memory_mb = 1024;
            Ok(())
        })
        .await?;

    // Save current config
    manager.save_current_config().await?;

    // Verify file contents
    let file_content = fs::read_to_string(&config_file)?;
    assert!(file_content.contains("chunk_size = 300"));
    assert!(file_content.contains("max_memory_mb = 1024"));

    Ok(())
}

#[tokio::test]
async fn test_config_field_watching() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("watch_test.toml");

    let manager = DynamicConfigManager::new(config_file).await?;

    // Start watching a field
    manager
        .watch_field("memory.max_memory_mb".to_string())
        .await;
    manager
        .watch_field("processing.parallel_processing".to_string())
        .await;

    // This is mainly testing that the watch doesn't panic
    // In a real implementation, you'd test that watched fields generate notifications

    Ok(())
}

#[tokio::test]
async fn test_config_error_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("error_test.toml");

    let manager = DynamicConfigManager::new(config_file.clone()).await?;

    // Test invalid field path
    let result = manager.get_field_value("nonexistent.field").await;
    assert!(result.is_err());

    let result = manager.set_field_value("nonexistent.field", "value").await;
    assert!(result.is_err());

    // Test malformed config file
    fs::write(&config_file, "invalid toml content [ unclosed")?;

    let result = manager.reload_from_file().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_config_type_conversions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("conversions.toml");

    let manager = DynamicConfigManager::new(config_file).await?;
    let config = manager.get_config().await;

    // Test cache config conversion
    let cache_config = config.cache.to_cache_config();
    assert_eq!(cache_config.max_size_mb, config.cache.max_size_mb);
    assert_eq!(cache_config.max_entries, config.cache.max_entries);
    assert_eq!(
        cache_config.ttl,
        Duration::from_secs(config.cache.ttl_hours * 3600)
    );

    // Test memory config conversion
    let memory_config = config.memory.to_memory_config();
    assert_eq!(memory_config.max_memory_mb, config.memory.max_memory_mb);
    assert_eq!(memory_config.max_entries, config.memory.max_entries);
    assert_eq!(memory_config.high_water_mark, config.memory.high_water_mark);

    // Test error recovery config conversion
    let recovery_strategy = config.error_recovery.to_recovery_strategy();
    assert_eq!(
        recovery_strategy.max_retries,
        config.error_recovery.max_retries
    );
    assert_eq!(
        recovery_strategy.backoff_multiplier,
        config.error_recovery.backoff_multiplier
    );

    Ok(())
}

#[tokio::test]
async fn test_config_concurrent_access() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("concurrent.toml");

    let manager = std::sync::Arc::new(DynamicConfigManager::new(config_file).await?);

    // Spawn multiple tasks that modify the config concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let _ = manager_clone
                .update_config(|config| {
                    config.processing.chunk_size = 100 + i * 10;
                    Ok(())
                })
                .await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    // Verify final state is consistent
    let config = manager.get_config().await;
    assert!(config.processing.chunk_size >= 100);
    assert!(config.processing.chunk_size <= 190);

    Ok(())
}

#[tokio::test]
async fn test_config_edge_cases() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("edge_cases.toml");

    let manager = DynamicConfigManager::new(config_file.clone()).await?;

    // Test empty string values
    let result = manager.set_field_value("memory.eviction_policy", "").await;
    assert!(result.is_ok()); // Empty string should be allowed

    // Test very large numbers
    let result = manager
        .set_field_value("memory.max_memory_mb", "999999")
        .await;
    assert!(result.is_err()); // Should be rejected by validation

    // Test negative numbers (should fail parsing)
    let result = manager
        .set_field_value("memory.max_memory_mb", "-100")
        .await;
    assert!(result.is_err());

    // Test non-numeric strings for numeric fields
    let result = manager
        .set_field_value("memory.max_memory_mb", "not_a_number")
        .await;
    assert!(result.is_err());

    // Test boolean values
    let result = manager
        .set_field_value("processing.parallel_processing", "false")
        .await;
    assert!(result.is_ok());

    let result = manager
        .set_field_value("processing.parallel_processing", "true")
        .await;
    assert!(result.is_ok());

    let result = manager
        .set_field_value("processing.parallel_processing", "maybe")
        .await;
    assert!(result.is_err());

    Ok(())
}
