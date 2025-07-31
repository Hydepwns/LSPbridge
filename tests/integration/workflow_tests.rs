//! End-to-end tests for common LSP Bridge workflows
//! 
//! These tests simulate real-world usage scenarios to ensure
//! the tool works correctly from start to finish.

use lsp_bridge::{
    capture::DiagnosticsCapture,
    // TODO: Re-enable when CLI modules are available
    // cli::{Commands, ExportFormat},
    core::{
        // config::UnifiedConfig,
        Diagnostic, DiagnosticSeverity, Position, Range,
        SimpleEnhancedProcessor, SimpleEnhancedConfig, PrivacyPolicy,
    },
    // TODO: Re-enable when export is available
    // export::ExportService,
    // privacy::{PrivacyConfig},
    project::ProjectAnalyzer,
    query::QueryEngine,
};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Test the complete workflow: capture -> filter -> export
#[tokio::test]
#[ignore] // TODO: Re-enable when capture/export modules are available
async fn test_capture_filter_export_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // 1. Set up project structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).await?;
    
    let main_file = src_dir.join("main.rs");
    fs::write(&main_file, r#"
fn main() {
    let x: i32 = "not a number";  // Type error
    println!("Hello, {}", undefined_var);  // Undefined variable
    
    let mut data = vec![1, 2, 3];
    let ref1 = &mut data;
    let ref2 = &mut data;  // Borrow checker error
}
"#).await?;
    
    // 2. Create diagnostics (simulating LSP output)
    let diagnostics = vec![
        Diagnostic {
            id: "1".to_string(),
            file: main_file.to_str().unwrap().to_string(),
            range: Range {
                start: Position { line: 2, character: 18 },
                end: Position { line: 2, character: 32 },
            },
            severity: DiagnosticSeverity::Error,
            message: "mismatched types: expected `i32`, found `&str`".to_string(),
            code: Some("E0308".to_string()),
            source: "rustc".to_string(),
            related_information: None,
            tags: None,
            data: None,
        },
        Diagnostic {
            id: "2".to_string(),
            file: main_file.to_str().unwrap().to_string(),
            range: Range {
                start: Position { line: 3, character: 26 },
                end: Position { line: 3, character: 39 },
            },
            severity: DiagnosticSeverity::Error,
            message: "cannot find value `undefined_var` in this scope".to_string(),
            code: Some("E0425".to_string()),
            source: "rustc".to_string(),
            related_information: None,
            tags: None,
            data: None,
        },
    ];
    
    // 3. Capture diagnostics with privacy filtering
    let mut capture = DiagnosticsCapture::new();
    let privacy_policy = PrivacyPolicy::default();
    // TODO: Configure privacy policy with FilterLevel when supported
    capture.set_privacy_policy(privacy_policy);
    
    let raw_diagnostics = lsp_bridge::core::types::RawDiagnostics {
        source: "rustc".to_string(),
        data: serde_json::json!({ "diagnostics": diagnostics }),
        timestamp: chrono::Utc::now(),
        workspace: Some(lsp_bridge::core::WorkspaceInfo {
            name: "test_project".to_string(),
            root_path: temp_dir.path().to_string_lossy().to_string(),
            language: Some("rust".to_string()),
            version: Some("0.1.0".to_string()),
        }),
    };
    
    let snapshot = capture.process_diagnostics(raw_diagnostics)?;
    
    // 4. Export to different formats
    let export_service = ExportService::new();
    
    // Export to Markdown
    let markdown_output = export_service.to_markdown(&snapshot)?;
    assert!(markdown_output.contains("## Diagnostics Summary"));
    assert!(markdown_output.contains("mismatched types"));
    
    // Export to JSON
    let json_output = export_service.to_json(&snapshot)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_output)?;
    assert_eq!(parsed["total_diagnostics"], 2);
    
    // 5. Verify privacy filtering worked
    // In standard mode, file paths should be relative
    assert!(!markdown_output.contains(temp_dir.path().to_str().unwrap()));
    
    Ok(())
}

/// Test the query workflow for finding specific diagnostics
#[tokio::test]
#[ignore] // TODO: Re-enable when query module is available
async fn test_diagnostic_query_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("diagnostics.db");
    
    // 1. Create query engine
    let mut engine = QueryEngine::new(Some(db_path)).await?;
    
    // 2. Store some diagnostics
    let diagnostics = vec![
        ("type_error", DiagnosticSeverity::Error, "Type 'string' is not assignable to type 'number'"),
        ("unused_var", DiagnosticSeverity::Warning, "Variable 'temp' is declared but never used"),
        ("deprecated", DiagnosticSeverity::Warning, "Function 'oldAPI' is deprecated"),
        ("syntax_error", DiagnosticSeverity::Error, "Unexpected token '}'"),
    ];
    
    for (code, severity, message) in diagnostics {
        let diag = Diagnostic {
            id: code.to_string(),
            file: format!("test_{}.ts", code),
            range: Range {
                start: Position { line: 10, character: 0 },
                end: Position { line: 10, character: 10 },
            },
            severity,
            message: message.to_string(),
            code: Some(code.to_string()),
            source: "typescript".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };
        engine.store_diagnostic(&diag).await?;
    }
    
    // 3. Query diagnostics
    let options = QueryOptions {
        severity: Some(DiagnosticSeverity::Error),
        pattern: None,
        limit: 10,
        language: None,
    };
    
    let results = engine.query_diagnostics(&options).await?;
    assert_eq!(results.len(), 2); // Should find 2 errors
    
    // 4. Query with pattern
    let options_with_pattern = QueryOptions {
        severity: None,
        pattern: Some("deprecated".to_string()),
        limit: 10,
        language: None,
    };
    
    let pattern_results = engine.query_diagnostics(&options_with_pattern).await?;
    assert_eq!(pattern_results.len(), 1);
    assert!(pattern_results[0].message.contains("deprecated"));
    
    Ok(())
}

/// Test the project analysis workflow
#[tokio::test]
#[ignore] // TODO: Re-enable when project analyzer is available
async fn test_project_analysis_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // 1. Create a project structure
    fs::write(temp_dir.path().join("Cargo.toml"), r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#).await?;
    
    fs::create_dir(temp_dir.path().join("src")).await?;
    fs::write(temp_dir.path().join("src/lib.rs"), r#"
pub mod utils;
pub mod core;

pub use utils::helper;
"#).await?;
    
    // 2. Analyze project
    let analyzer = ProjectAnalyzer::new()?;
    let analysis = analyzer.analyze_directory(temp_dir.path()).await?;
    
    // 3. Verify analysis results
    assert_eq!(analysis.project_type, lsp_bridge::project::ProjectType::Rust);
    assert!(analysis.build_system.is_some());
    assert_eq!(analysis.build_system.unwrap(), lsp_bridge::project::BuildSystem::Cargo);
    
    // Check dependencies
    assert!(analysis.dependencies.contains(&"serde".to_string()));
    assert!(analysis.dependencies.contains(&"tokio".to_string()));
    
    Ok(())
}

/// Test the enhanced processing workflow with caching
#[tokio::test]
async fn test_enhanced_processing_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // 1. Create processor with caching enabled
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().join("cache"),
        enable_persistent_cache: true,
        enable_metrics: true,
        ..Default::default()
    };
    
    let processor = SimpleEnhancedProcessor::new(config).await?;
    
    // 2. Create test files
    let files = vec![
        ("file1.rs", "fn main() { println!(\"Hello\"); }"),
        ("file2.rs", "fn test() { assert_eq!(1, 1); }"),
        ("file3.rs", "struct User { name: String }"),
    ];
    
    for (name, content) in &files {
        fs::write(temp_dir.path().join(name), content).await?;
    }
    
    let file_paths: Vec<PathBuf> = files.iter()
        .map(|(name, _)| temp_dir.path().join(name))
        .collect();
    
    // 3. First processing run
    let changed_files_1 = processor.detect_changed_files(&file_paths).await?;
    assert_eq!(changed_files_1.len(), 3); // All files are new
    
    // 4. Process again without changes
    let changed_files_2 = processor.detect_changed_files(&file_paths).await?;
    assert_eq!(changed_files_2.len(), 0); // No files changed
    
    // 5. Modify one file
    fs::write(
        temp_dir.path().join("file1.rs"),
        "fn main() { println!(\"Hello, World!\"); }"
    ).await?;
    
    let changed_files_3 = processor.detect_changed_files(&file_paths).await?;
    assert_eq!(changed_files_3.len(), 1); // Only one file changed
    assert!(changed_files_3[0].ends_with("file1.rs"));
    
    // 6. Check performance metrics
    let summary = processor.get_performance_summary().await?;
    assert!(summary.core_cache_files >= 3);
    assert_eq!(summary.core_cache_hit_rate, 0.5); // 50% hit rate (3 misses, 3 hits)
    
    Ok(())
}

/// Test the multi-language workflow
#[tokio::test]
#[ignore] // TODO: Re-enable when project/capture modules are available
async fn test_multi_language_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // 1. Create files in different languages
    let rust_file = temp_dir.path().join("backend.rs");
    fs::write(&rust_file, r#"
fn process_user(id: u64) -> Result<User, Error> {
    let user = fetch_user(id)?;
    Ok(user)
}
"#).await?;
    
    let ts_file = temp_dir.path().join("frontend.ts");
    fs::write(&ts_file, r#"
interface User {
    id: number;
    name: string;
}

async function fetchUser(id: number): Promise<User> {
    const response = await fetch(`/api/users/${id}`);
    return response.json();
}
"#).await?;
    
    let py_file = temp_dir.path().join("scripts.py");
    fs::write(&py_file, r#"
def analyze_users(user_list):
    return [u for u in user_list if u.active]
"#).await?;
    
    // 2. Analyze each file
    let analyzer = ProjectAnalyzer::new()?;
    
    let rust_lang = analyzer.detect_language(&rust_file)?;
    assert_eq!(rust_lang, Some(lsp_bridge::project::Language::Rust));
    
    let ts_lang = analyzer.detect_language(&ts_file)?;
    assert_eq!(ts_lang, Some(lsp_bridge::project::Language::TypeScript));
    
    let py_lang = analyzer.detect_language(&py_file)?;
    assert_eq!(py_lang, Some(lsp_bridge::project::Language::Python));
    
    // 3. Create diagnostics for each language
    let diagnostics = vec![
        Diagnostic {
            id: "1".to_string(),
            file: rust_file.to_str().unwrap().to_string(),
            range: Range {
                start: Position { line: 2, character: 15 },
                end: Position { line: 2, character: 25 },
            },
            severity: DiagnosticSeverity::Error,
            message: "cannot find function `fetch_user`".to_string(),
            code: Some("E0425".to_string()),
            source: "rust-analyzer".to_string(),
            related_information: None,
            tags: None,
            data: None,
        },
        Diagnostic {
            id: "2".to_string(),
            file: ts_file.to_str().unwrap().to_string(),
            range: Range {
                start: Position { line: 7, character: 30 },
                end: Position { line: 7, character: 35 },
            },
            severity: DiagnosticSeverity::Error,
            message: "Cannot find name 'fetch'.".to_string(),
            code: Some("2304".to_string()),
            source: "typescript".to_string(),
            related_information: None,
            tags: None,
            data: None,
        },
    ];
    
    // 4. Export with language grouping
    let capture = DiagnosticsCapture::new();
    let snapshot = capture.create_snapshot(diagnostics);
    
    let export_service = ExportService::new();
    let markdown = export_service.to_markdown(&snapshot)?;
    
    // Verify both languages are represented
    assert!(markdown.contains("rust-analyzer"));
    assert!(markdown.contains("typescript"));
    
    Ok(())
}

/// Test error recovery workflow
#[tokio::test]
async fn test_error_recovery_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // 1. Create processor with error recovery
    let config = SimpleEnhancedConfig {
        cache_dir: temp_dir.path().join("cache"),
        enable_error_recovery: true,
        ..Default::default()
    };
    
    let processor = SimpleEnhancedProcessor::new(config).await?;
    
    // 2. Test with problematic files
    let problem_files = vec![
        temp_dir.path().join("nonexistent.rs"),
        temp_dir.path().join("huge_file.rs"),
        temp_dir.path().join("binary_file.bin"),
    ];
    
    // Create huge file
    let huge_content = "x".repeat(10_000_000); // 10MB
    fs::write(&problem_files[1], huge_content).await?;
    
    // Create binary file
    fs::write(&problem_files[2], vec![0u8, 1, 2, 3, 255, 254]).await?;
    
    // 3. Process should handle errors gracefully
    let result = processor.detect_changed_files(&problem_files).await;
    assert!(result.is_ok()); // Should not panic
    
    // 4. Check error recovery system
    let health_check = processor.check_health().await?;
    assert!(health_check.is_healthy);
    
    Ok(())
}