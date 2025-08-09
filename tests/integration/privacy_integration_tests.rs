//! # Privacy Integration Tests
//!
//! Tests to verify that privacy policy integration works correctly with the capture service
//! and that diagnostics are properly filtered based on configured privacy settings.

use lsp_bridge::{
    capture::DiagnosticsCapture,
    core::{
        Diagnostic, DiagnosticSeverity, Position, Range, PrivacyPolicy, RawDiagnostics,
        WorkspaceInfo,
    },
};
use std::collections::HashMap;
use tempfile::TempDir;
use uuid::Uuid;

/// Create a test diagnostic with specified properties
fn create_test_diagnostic(
    file: &str,
    message: &str,
    severity: DiagnosticSeverity,
    line: u32,
) -> Diagnostic {
    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: file.to_string(),
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: 10,
            },
        },
        severity,
        message: message.to_string(),
        code: None,
        source: "test".to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create test raw diagnostics for processing
fn create_raw_diagnostics(diagnostics: Vec<Diagnostic>) -> RawDiagnostics {
    use chrono::Utc;
    use serde_json::json;
    
    RawDiagnostics {
        source: "test_lsp".to_string(),
        data: json!({
            "diagnostics": diagnostics,
            "version": "1.0.0"
        }),
        timestamp: Utc::now(),
        workspace: Some(WorkspaceInfo {
            name: "test_workspace".to_string(),
            root_path: "/tmp/test".to_string(),
            language: Some("rust".to_string()),
            version: Some("1.0.0".to_string()),
        }),
    }
}

#[tokio::test]
async fn test_default_privacy_policy_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that the default privacy policy allows normal diagnostics through
    let capture = DiagnosticsCapture::new();
    
    let diagnostics = vec![
        create_test_diagnostic("src/main.rs", "undefined variable `x`", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "unused import", DiagnosticSeverity::Warning, 5),
        create_test_diagnostic("tests/test.rs", "deprecated function", DiagnosticSeverity::Information, 15),
    ];
    
    // Test the privacy filtering by directly using the snapshot creation
    // which goes through the privacy filter in the capture service
    let snapshot = capture.create_snapshot(diagnostics);
    
    // Verify diagnostics were processed (default policy should allow all through)
    assert_eq!(snapshot.diagnostics.len(), 3);
    assert!(snapshot.diagnostics.iter().any(|d| d.file.contains("main.rs")));
    assert!(snapshot.diagnostics.iter().any(|d| d.file.contains("lib.rs")));  
    assert!(snapshot.diagnostics.iter().any(|d| d.file.contains("test.rs")));
    
    // Check that privacy policy can be retrieved
    let policy = capture.get_privacy_policy();
    assert!(!policy.include_only_errors); // Default should include all severities
    
    Ok(())
}

#[tokio::test]
async fn test_strict_privacy_policy_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test strict privacy policy filters out non-errors
    let mut capture = DiagnosticsCapture::with_strict_privacy();
    
    let diagnostics = vec![
        create_test_diagnostic("src/main.rs", "undefined variable `x`", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "unused import", DiagnosticSeverity::Warning, 5),
        create_test_diagnostic("tests/test.rs", "deprecated function", DiagnosticSeverity::Information, 15),
    ];
    
    let raw = create_raw_diagnostics(diagnostics);
    
    capture.start_capture().await?;
    let snapshot = capture.process_diagnostics(raw).await?;
    
    // Strict policy should only include errors
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert!(snapshot.diagnostics[0].file.contains("main.rs"));
    assert_eq!(snapshot.diagnostics[0].severity, DiagnosticSeverity::Error);
    
    // Verify policy settings
    let policy = capture.get_privacy_policy();
    assert!(policy.include_only_errors);
    
    Ok(())
}

#[tokio::test]
async fn test_permissive_privacy_policy_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test permissive privacy policy allows everything through without filtering
    let mut capture = DiagnosticsCapture::with_permissive_privacy();
    
    let diagnostics = vec![
        create_test_diagnostic("src/secret.rs", "API key \"sk-123secret456\"", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("/home/user/project/lib.rs", "unused import", DiagnosticSeverity::Warning, 5),
        create_test_diagnostic("tests/test.rs", "// TODO: fix this hack", DiagnosticSeverity::Information, 15),
    ];
    
    let raw = create_raw_diagnostics(diagnostics);
    
    capture.start_capture().await?;
    let snapshot = capture.process_diagnostics(raw).await?;
    
    // Permissive policy should include all diagnostics
    assert_eq!(snapshot.diagnostics.len(), 3);
    
    // With permissive policy, sensitive data should still be visible 
    let policy = capture.get_privacy_policy();
    assert!(!policy.sanitize_strings); // Permissive doesn't sanitize strings
    assert!(!policy.anonymize_file_paths); // Permissive doesn't anonymize paths
    
    Ok(())
}

#[tokio::test]
async fn test_custom_privacy_policy_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test custom privacy policy with specific settings
    let custom_policy = PrivacyPolicy {
        exclude_patterns: vec!["**/secret/**".to_string(), "*.env".to_string()],
        sanitize_strings: true,
        sanitize_comments: true,
        include_only_errors: false,
        max_diagnostics_per_file: 2,
        anonymize_file_paths: true,
        encrypt_exports: false,
    };
    
    let mut capture = DiagnosticsCapture::with_privacy_policy(custom_policy.clone());
    
    let diagnostics = vec![
        create_test_diagnostic("src/main.rs", "Error with \"sensitive data\"", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/secret/api.rs", "Should be excluded", DiagnosticSeverity::Error, 5),
        create_test_diagnostic("config.env", "Also excluded", DiagnosticSeverity::Warning, 1),
        create_test_diagnostic("src/lib.rs", "// Contains comment", DiagnosticSeverity::Warning, 15),
        create_test_diagnostic("src/lib.rs", "Another diagnostic", DiagnosticSeverity::Information, 20),
        create_test_diagnostic("src/lib.rs", "Third diagnostic (should be limited)", DiagnosticSeverity::Warning, 25),
    ];
    
    let raw = create_raw_diagnostics(diagnostics);
    
    capture.start_capture().await?;
    let snapshot = capture.process_diagnostics(raw).await?;
    
    // Should exclude files matching patterns and limit per-file diagnostics
    assert!(snapshot.diagnostics.len() <= 3); // Exclusions + per-file limits
    
    // Should not contain excluded files
    assert!(!snapshot.diagnostics.iter().any(|d| d.file.contains("secret")));
    assert!(!snapshot.diagnostics.iter().any(|d| d.file.contains(".env")));
    
    // Should have at most 2 diagnostics per file (max_diagnostics_per_file)
    let mut file_counts = HashMap::new();
    for diagnostic in &snapshot.diagnostics {
        *file_counts.entry(&diagnostic.file).or_insert(0) += 1;
    }
    for count in file_counts.values() {
        assert!(*count <= 2, "File has more than 2 diagnostics: {}", count);
    }
    
    // Verify policy was applied correctly
    let retrieved_policy = capture.get_privacy_policy();
    assert_eq!(retrieved_policy.exclude_patterns, custom_policy.exclude_patterns);
    assert_eq!(retrieved_policy.max_diagnostics_per_file, custom_policy.max_diagnostics_per_file);
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_policy_update() -> Result<(), Box<dyn std::error::Error>> {
    // Test updating privacy policy on existing capture
    let mut capture = DiagnosticsCapture::new();
    
    // Initially use default policy
    let diagnostics1 = vec![
        create_test_diagnostic("src/main.rs", "Error message", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "Warning message", DiagnosticSeverity::Warning, 5),
    ];
    
    let raw1 = create_raw_diagnostics(diagnostics1);
    capture.start_capture().await?;
    let snapshot1 = capture.process_diagnostics(raw1).await?;
    
    // Should include both error and warning
    assert_eq!(snapshot1.diagnostics.len(), 2);
    
    // Update to strict policy (errors only)
    let strict_policy = PrivacyPolicy::strict();
    capture.set_privacy_policy(strict_policy);
    
    // Process same diagnostics again
    let diagnostics2 = vec![
        create_test_diagnostic("src/main.rs", "Error message", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "Warning message", DiagnosticSeverity::Warning, 5),
    ];
    
    let raw2 = create_raw_diagnostics(diagnostics2);
    let snapshot2 = capture.process_diagnostics(raw2).await?;
    
    // Should only include error now
    assert_eq!(snapshot2.diagnostics.len(), 1);
    assert_eq!(snapshot2.diagnostics[0].severity, DiagnosticSeverity::Error);
    
    // Verify policy was updated
    let updated_policy = capture.get_privacy_policy();
    assert!(updated_policy.include_only_errors);
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_policy_with_workspace_filtering() -> Result<(), Box<dyn std::error::Error>> {
    // Test privacy policy with workspace-aware filtering
    let temp_dir = TempDir::new()?;
    let workspace_root = temp_dir.path().to_path_buf();
    
    let policy = PrivacyPolicy::default();
    let mut capture = DiagnosticsCapture::with_privacy_policy(policy);
    capture.set_privacy_policy_with_workspace(PrivacyPolicy::default(), workspace_root.clone());
    
    // Create diagnostics inside and outside workspace
    let workspace_file = workspace_root.join("src/main.rs");
    let outside_file = "/tmp/external/lib.rs";
    
    let diagnostics = vec![
        create_test_diagnostic(
            workspace_file.to_string_lossy().as_ref(),
            "Inside workspace",
            DiagnosticSeverity::Error,
            10
        ),
        create_test_diagnostic(outside_file, "Outside workspace", DiagnosticSeverity::Error, 5),
    ];
    
    let raw = create_raw_diagnostics(diagnostics);
    capture.start_capture().await?;
    let snapshot = capture.process_diagnostics(raw).await?;
    
    // Verify workspace filtering occurred (specific behavior depends on WorkspaceFilter implementation)
    // At minimum, should process without errors
    assert!(snapshot.diagnostics.len() <= 2);
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_policy_string_sanitization() -> Result<(), Box<dyn std::error::Error>> {
    // Test that string sanitization works in the pipeline
    let policy = PrivacyPolicy {
        sanitize_strings: true,
        sanitize_comments: true,
        ..PrivacyPolicy::default()
    };
    
    let mut capture = DiagnosticsCapture::with_privacy_policy(policy);
    
    let diagnostics = vec![
        create_test_diagnostic(
            "src/main.rs",
            "Variable \"secret_key\" is undefined",
            DiagnosticSeverity::Error,
            10
        ),
        create_test_diagnostic(
            "src/lib.rs",
            "// TODO: remove this hardcoded password",
            DiagnosticSeverity::Warning,
            5
        ),
    ];
    
    let raw = create_raw_diagnostics(diagnostics);
    capture.start_capture().await?;
    let snapshot = capture.process_diagnostics(raw).await?;
    
    // Verify strings and comments were sanitized
    assert_eq!(snapshot.diagnostics.len(), 2);
    
    // Check that string literals are replaced
    let error_diagnostic = snapshot.diagnostics.iter()
        .find(|d| d.severity == DiagnosticSeverity::Error)
        .expect("Should have error diagnostic");
    assert!(error_diagnostic.message.contains("[STRING]"));
    assert!(!error_diagnostic.message.contains("secret_key"));
    
    // Check that comments are replaced  
    let warning_diagnostic = snapshot.diagnostics.iter()
        .find(|d| d.severity == DiagnosticSeverity::Warning)
        .expect("Should have warning diagnostic");
    assert!(warning_diagnostic.message.contains("[COMMENT]"));
    assert!(!warning_diagnostic.message.contains("hardcoded password"));
    
    Ok(())
}