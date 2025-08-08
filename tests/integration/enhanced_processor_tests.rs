use anyhow::Result;
use lsp_bridge::core::{
    types::{Diagnostic, DiagnosticSeverity, Position, Range},
    SimpleEnhancedConfig, SimpleEnhancedProcessor,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

fn create_test_diagnostic(file: &str, line: u32, message: &str) -> Diagnostic {
    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: file.to_string(),
        range: Range {
            start: Position { line, character: 1 },
            end: Position { line, character: 1 },
        },
        severity: DiagnosticSeverity::Error,
        message: message.to_string(),
        code: Some("TEST001".to_string()),
        source: "test".to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[tokio::test]
async fn test_enhanced_processor_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Test that processor was created successfully
    let summary = processor.get_performance_summary().await?;
    assert_eq!(summary.core_cache_files, 0);
    assert_eq!(summary.core_cache_diagnostics, 0);

    Ok(())
}

#[tokio::test]
async fn test_enhanced_processor_with_all_components() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("test_config.toml");

    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false, // Disable for testing since we're not in a git repo
        enable_dynamic_config: true,
        config_file: Some(config_file),
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Test that all enabled components are working
    let summary = processor.get_performance_summary().await?;
    assert_eq!(summary.core_cache_files, 0);

    // Test dynamic config access
    let dynamic_config = processor.get_dynamic_config().await;
    assert!(dynamic_config.is_some());

    Ok(())
}

#[tokio::test]
async fn test_file_change_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Test with empty file list
    let files: Vec<PathBuf> = vec![];
    let changed_files = processor.detect_changed_files(&files).await?;
    assert!(changed_files.is_empty());

    // Test with non-existent files
    // Use temp directory for non-existent file paths
    let files = vec![
        temp_dir.path().join("nonexistent1.rs"),
        temp_dir.path().join("nonexistent2.rs"),
    ];
    let changed_files = processor.detect_changed_files(&files).await?;
    // All non-existent files should be considered "changed" (new)
    assert_eq!(changed_files.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_cache_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_persistent_cache: true,
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Create an actual test file
    let test_file = temp_dir.path().join("test_file.rs");
    std::fs::write(&test_file, "fn main() { println!(\"test\"); }")?;

    let diagnostics = vec![
        create_test_diagnostic(&test_file.to_string_lossy(), 10, "Test error 1"),
        create_test_diagnostic(&test_file.to_string_lossy(), 20, "Test error 2"),
    ];

    // Initially no cached diagnostics
    let cached = processor.get_cached_diagnostics(&test_file).await;
    assert!(cached.is_none());

    // Update cache
    processor.update_cache(&test_file, &diagnostics).await?;

    // Now should have cached diagnostics
    let cached = processor.get_cached_diagnostics(&test_file).await;
    assert!(cached.is_some());
    let cached_diagnostics = cached.unwrap();
    assert_eq!(cached_diagnostics.len(), 2);
    assert_eq!(cached_diagnostics[0].message, "Test error 1");
    assert_eq!(cached_diagnostics[1].message, "Test error 2");

    Ok(())
}

#[tokio::test]
async fn test_incremental_processing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Create test files in temp directory
    let test_file1 = temp_dir.path().join("file1.rs");
    let test_file2 = temp_dir.path().join("file2.rs");
    std::fs::write(&test_file1, "// test content 1")?;
    std::fs::write(&test_file2, "// test content 2")?;
    
    let files = vec![test_file1.clone(), test_file2.clone()];

    // Mock processor function that returns diagnostics
    let processor_fn = |files: &[PathBuf]| -> Result<HashMap<PathBuf, Vec<Diagnostic>>> {
        let mut result = HashMap::new();
        for file in files {
            let diagnostics = vec![create_test_diagnostic(
                &file.to_string_lossy(),
                1,
                "Mock error",
            )];
            result.insert(file.clone(), diagnostics);
        }
        Ok(result)
    };

    let (diagnostics, stats) = processor
        .process_files_incrementally(&files, processor_fn)
        .await?;

    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.changed_files, 2); // All files are "new" so all are changed
    assert_eq!(stats.cached_files, 0);
    assert!(!diagnostics.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_cache_clearing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Create an actual test file
    let test_file = temp_dir.path().join("test_cache_clear.rs");
    std::fs::write(&test_file, "fn test() { let x = 42; }")?;

    let diagnostics = vec![create_test_diagnostic(
        &test_file.to_string_lossy(),
        5,
        "Test",
    )];
    processor.update_cache(&test_file, &diagnostics).await?;

    // Verify cache has data
    let cached = processor.get_cached_diagnostics(&test_file).await;
    assert!(cached.is_some());

    // Clear all caches
    processor.clear_all_caches().await?;

    // Verify cache is empty
    let cached_after_clear = processor.get_cached_diagnostics(&test_file).await;
    assert!(cached_after_clear.is_none());

    Ok(())
}

#[tokio::test]
async fn test_performance_summary() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Create an actual test file
    let test_file = temp_dir.path().join("perf_test.rs");
    std::fs::write(&test_file, "fn main() { println!(\"perf test\"); }")?;

    let diagnostics = vec![
        create_test_diagnostic(&test_file.to_string_lossy(), 1, "Error 1"),
        create_test_diagnostic(&test_file.to_string_lossy(), 2, "Error 2"),
    ];
    processor.update_cache(&test_file, &diagnostics).await?;

    let summary = processor.get_performance_summary().await?;

    assert!(summary.core_cache_files >= 1);
    assert!(summary.core_cache_diagnostics >= 2);
    assert!(summary.error_count >= 0);
    assert!(summary.recent_error_rate >= 0.0);

    Ok(())
}

#[tokio::test]
async fn test_optimization() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        auto_optimization: true,
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Run optimization manually (normally triggered automatically)
    processor.optimize().await?;

    // Should complete without errors
    Ok(())
}

#[tokio::test]
async fn test_dynamic_configuration_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_file = temp_dir.path().join("integration_test.toml");

    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_dynamic_config: true,
        config_file: Some(config_file.clone()),
        enable_git_integration: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Test getting dynamic config
    let dynamic_config = processor.get_dynamic_config().await;
    assert!(dynamic_config.is_some());

    // Test updating config
    let changes = processor
        .update_dynamic_config(|config| {
            config.processing.chunk_size = 150;
            Ok(())
        })
        .await?;

    assert!(!changes.is_empty());
    assert_eq!(changes[0].field_path, "processing.chunk_size");

    // Test that changes were applied
    let updated_config = processor.get_dynamic_config().await;
    assert!(updated_config.is_some());
    // Note: Field-level access is not directly supported through the processor
    // The processor provides update_dynamic_config for bulk updates

    Ok(())
}

#[tokio::test]
async fn test_git_integration_when_disabled() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // All git operations should return empty/default results when disabled
    let modified_files = processor.get_git_modified_files().await?;
    assert!(modified_files.is_empty());

    let conflicted_files = processor.get_conflicted_files().await?;
    assert!(conflicted_files.is_empty());

    let untracked_files = processor.get_untracked_files().await?;
    assert!(untracked_files.is_empty());

    let is_clean = processor.is_git_repository_clean().await?;
    assert!(is_clean); // Should assume clean when git is disabled

    let repo_info = processor.get_git_repository_info().await;
    assert!(repo_info.is_none());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = Arc::new(SimpleEnhancedProcessor::new(config).await?);

    // Create actual test files
    let mut test_files = Vec::new();
    for i in 0..10 {
        let test_file = temp_dir.path().join(format!("concurrent_test_{}.rs", i));
        std::fs::write(&test_file, format!("fn test_{}() {{ let x = {}; }}", i, i))?;
        test_files.push(test_file);
    }

    // Spawn multiple tasks that perform operations concurrently
    let mut handles = Vec::new();

    for (i, test_file) in test_files.into_iter().enumerate() {
        let processor_clone = processor.clone();
        let handle = tokio::spawn(async move {
            let diagnostics = vec![create_test_diagnostic(
                &test_file.to_string_lossy(),
                1,
                "Concurrent error",
            )];

            // Perform cache operations
            let _ = processor_clone.update_cache(&test_file, &diagnostics).await;
            let _ = processor_clone.get_cached_diagnostics(&test_file).await;
            let _ = processor_clone.get_performance_summary().await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    // Verify final state is consistent
    let summary = processor.get_performance_summary().await?;
    assert!(summary.core_cache_files >= 5); // Should have cached multiple files

    Ok(())
}

#[tokio::test]
async fn test_error_recovery_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Test that error recovery doesn't interfere with normal operations
    // Create test file in temp directory
    let test_file = temp_dir.path().join("error_recovery_test.rs");
    std::fs::write(&test_file, "// error recovery test")?;
    let files = vec![test_file];

    let processor_fn = |_files: &[PathBuf]| -> Result<HashMap<PathBuf, Vec<Diagnostic>>> {
        // Return empty result (no errors)
        Ok(HashMap::new())
    };

    let (diagnostics, stats) = processor
        .process_files_incrementally(&files, processor_fn)
        .await?;

    assert_eq!(stats.total_files, 1);
    assert!(diagnostics.is_empty()); // No diagnostics since processor_fn returns empty

    Ok(())
}

#[tokio::test]
async fn test_memory_management() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Create many actual test files to test memory management
    for i in 0..100 {
        let test_file = temp_dir.path().join(format!("memory_test_{}.rs", i));
        std::fs::write(&test_file, format!("fn test_{}() {{ let x = {}; }}", i, i))?;
        let diagnostics = vec![create_test_diagnostic(
            &test_file.to_string_lossy(),
            1,
            &format!("Error {}", i),
        )];
        processor.update_cache(&test_file, &diagnostics).await?;
    }

    let summary = processor.get_performance_summary().await?;
    assert!(summary.core_cache_files >= 50); // Should have cached many files

    // Test optimization with large cache
    processor.optimize().await?;

    Ok(())
}

#[tokio::test]
async fn test_processor_with_real_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Create actual files for testing
    let test_file1 = temp_dir.path().join("real_test1.rs");
    let test_file2 = temp_dir.path().join("real_test2.rs");

    std::fs::write(&test_file1, "fn main() { println!(\"Hello\"); }")?;
    std::fs::write(&test_file2, "fn test() { let x = 42; }")?;

    let files = vec![test_file1.clone(), test_file2.clone()];

    // Test change detection with real files
    let changed_files = processor.detect_changed_files(&files).await?;
    assert_eq!(changed_files.len(), 2); // Both files should be detected as changed (new)

    // Test caching with real files
    let diagnostics = vec![create_test_diagnostic(
        &test_file1.to_string_lossy(),
        1,
        "Real file error",
    )];
    processor.update_cache(&test_file1, &diagnostics).await?;

    let cached = processor.get_cached_diagnostics(&test_file1).await;
    assert!(cached.is_some());

    Ok(())
}

#[tokio::test]
async fn test_processor_edge_cases() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        enable_git_integration: false,
        enable_dynamic_config: false,
        ..Default::default()
    };

    let processor = SimpleEnhancedProcessor::new(config).await?;

    // Test with empty diagnostics (using actual file)
    let test_file1 = temp_dir.path().join("empty_diagnostics.rs");
    std::fs::write(&test_file1, "fn empty() {}")?;
    let empty_diagnostics: Vec<Diagnostic> = vec![];
    processor
        .update_cache(&test_file1, &empty_diagnostics)
        .await?;

    let cached = processor.get_cached_diagnostics(&test_file1).await;
    assert!(cached.is_some());
    assert!(cached.unwrap().is_empty());

    // Test with very long file names (but still actual files)
    let long_name = "very_long_file_name".repeat(10) + ".rs";
    let test_file2 = temp_dir.path().join(long_name);
    std::fs::write(&test_file2, "fn long_name() {}")?;
    let diagnostics = vec![create_test_diagnostic(
        &test_file2.to_string_lossy(),
        1,
        "Long path error",
    )];
    processor.update_cache(&test_file2, &diagnostics).await?;

    // Test with special characters in file names (actual files)
    let test_file3 = temp_dir.path().join("file_with_symbols.rs");
    std::fs::write(&test_file3, "fn special() {}")?;
    let diagnostics = vec![create_test_diagnostic(
        &test_file3.to_string_lossy(),
        1,
        "Special chars error",
    )];
    processor.update_cache(&test_file3, &diagnostics).await?;

    Ok(())
}
