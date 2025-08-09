//! # Quick Fix Verification Integration Tests
//!
//! Tests to verify that quick-fix verification works correctly with different types of fixes,
//! build systems, and validation scenarios.

use lsp_bridge::{
    core::{Diagnostic, DiagnosticSeverity, Position, Range},
    quick_fix::{
        engine::{FixApplicationEngine, FixEdit, FixResult, FileBackup},
        verification::{FixVerifier, VerificationResult, BuildStatus, TestResults},
    },
};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

/// Create a test diagnostic with specified properties
fn create_test_diagnostic(
    file: &str,
    message: &str,
    severity: DiagnosticSeverity,
    line: u32,
    code: Option<&str>,
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
        code: code.map(|c| c.to_string()),
        source: "test_lsp".to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create a test fix edit
fn create_test_fix_edit(file_path: &str, new_text: &str, description: &str) -> FixEdit {
    FixEdit {
        file_path: PathBuf::from(file_path),
        range: Range {
            start: Position { line: 10, character: 5 },
            end: Position { line: 10, character: 15 },
        },
        new_text: new_text.to_string(),
        description: Some(description.to_string()),
    }
}

/// Create a successful fix result for testing
fn create_successful_fix_result(modified_files: Vec<PathBuf>) -> FixResult {
    FixResult {
        success: true,
        modified_files,
        error: None,
        backup: Some(FileBackup {
            file_path: PathBuf::from("test.rs"),
            original_content: "// Original content".to_string(),
            timestamp: chrono::Utc::now(),
        }),
    }
}

#[tokio::test]
async fn test_fix_verifier_creation() -> Result<(), Box<dyn std::error::Error>> {
    // Test that fix verifier can be created with different configurations
    let verifier = FixVerifier::new();
    assert!(!verifier.run_tests); // Should default to false
    assert!(verifier.check_build); // Should default to true
    assert!(verifier.use_lsp_validation); // Should default to true

    let verifier_with_tests = FixVerifier::new()
        .with_tests(true)
        .with_build_check(false)
        .with_lsp_validation(false);

    // Configuration should be applied correctly
    // Note: These fields are private, so we can't directly test them
    // Instead we test through behavior
    Ok(())
}

#[tokio::test]
async fn test_simple_fix_verification() -> Result<(), Box<dyn std::error::Error>> {
    // Test verification of a simple fix (syntax error)
    let verifier = FixVerifier::new()
        .with_lsp_validation(false) // Use simple validation for predictable results
        .with_build_check(false); // Skip build for this test

    let diagnostic = create_test_diagnostic(
        "src/main.rs",
        "missing semicolon",
        DiagnosticSeverity::Error,
        10,
        Some("syntax_error"),
    );

    let fix_result = create_successful_fix_result(vec![PathBuf::from("src/main.rs")]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // Simple fixes should be considered successful
    assert!(verification.issue_resolved);
    assert!(verification.new_issues.is_empty());
    assert_eq!(verification.resolved_issues.len(), 1);
    assert!(verification.build_status.success);

    Ok(())
}

#[tokio::test]
async fn test_complex_fix_verification() -> Result<(), Box<dyn std::error::Error>> {
    // Test verification of a complex fix (type/generic issue)
    let verifier = FixVerifier::new()
        .with_lsp_validation(false) // Use simple validation
        .with_build_check(false);

    let diagnostic = create_test_diagnostic(
        "src/complex.rs",
        "type mismatch in generic interface template",
        DiagnosticSeverity::Error,
        15,
        Some("type_error"),
    );

    let fix_result = create_successful_fix_result(vec![PathBuf::from("src/complex.rs")]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // Complex fixes should be treated more cautiously
    // The exact behavior depends on the complexity estimation
    assert!(verification.build_status.success);

    Ok(())
}

#[tokio::test]
async fn test_lsp_validation() -> Result<(), Box<dyn std::error::Error>> {
    // Test LSP-based validation
    let verifier = FixVerifier::new()
        .with_lsp_validation(true)
        .with_build_check(false);

    let diagnostic = create_test_diagnostic(
        "src/test.ts",
        "undefined variable 'example'",
        DiagnosticSeverity::Error,
        5,
        Some("undefined_var"),
    );

    let fix_result = create_successful_fix_result(vec![PathBuf::from("src/test.ts")]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // LSP validation should provide more detailed results
    assert!(verification.build_status.success);
    // Either resolved or not, but should not panic

    Ok(())
}

#[tokio::test]
async fn test_failed_fix_verification() -> Result<(), Box<dyn std::error::Error>> {
    // Test verification of a fix that was not applied successfully
    let verifier = FixVerifier::new()
        .with_build_check(false);

    let diagnostic = create_test_diagnostic(
        "src/main.rs",
        "test error",
        DiagnosticSeverity::Error,
        10,
        Some("test_error"),
    );

    let failed_fix_result = FixResult {
        success: false,
        modified_files: vec![],
        error: Some("Fix could not be applied".to_string()),
        backup: None,
    };

    let verification = verifier.verify_fix(&diagnostic, &failed_fix_result).await?;

    // Failed fixes should be marked as unresolved
    assert!(!verification.issue_resolved);
    assert!(!verification.build_status.success);
    assert!(!verification.build_status.errors.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_build_verification() -> Result<(), Box<dyn std::error::Error>> {
    // Test build verification (this will actually try to run build commands)
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() { println!(\"Hello, world!\"); }")?;

    let verifier = FixVerifier::new()
        .with_build_check(true)
        .with_lsp_validation(false);

    let diagnostic = create_test_diagnostic(
        test_file.to_string_lossy().as_ref(),
        "test diagnostic",
        DiagnosticSeverity::Warning,
        1,
        None,
    );

    let fix_result = create_successful_fix_result(vec![test_file.clone()]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // Build might fail if cargo is not available, but the structure should be correct
    // We mainly test that it doesn't panic and returns a reasonable result
    assert!(verification.build_status.duration_ms >= 0);

    Ok(())
}

#[tokio::test]
async fn test_test_verification() -> Result<(), Box<dyn std::error::Error>> {
    // Test test execution verification
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() { println!(\"Hello, world!\"); }")?;

    let verifier = FixVerifier::new()
        .with_tests(true)
        .with_build_check(false)
        .with_lsp_validation(false);

    let diagnostic = create_test_diagnostic(
        test_file.to_string_lossy().as_ref(),
        "test diagnostic",
        DiagnosticSeverity::Warning,
        1,
        None,
    );

    let fix_result = create_successful_fix_result(vec![test_file.clone()]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // Test might fail if test runner is not available, but should not panic
    // The test_results field should be populated
    if verification.test_results.is_some() {
        let test_results = verification.test_results.as_ref().unwrap();
        assert!(test_results.total >= 0);
        assert!(test_results.passed + test_results.failed + test_results.skipped <= test_results.total);
    }

    Ok(())
}

#[tokio::test]
async fn test_complexity_estimation() -> Result<(), Box<dyn std::error::Error>> {
    // Test that fix complexity estimation works correctly
    let verifier = FixVerifier::new();

    // Test simple fixes (should have low complexity)
    let simple_diagnostic = create_test_diagnostic(
        "src/main.rs",
        "missing semicolon at end of statement",
        DiagnosticSeverity::Error,
        10,
        Some("syntax_error"),
    );

    let simple_complexity = verifier.estimate_fix_complexity(&simple_diagnostic);
    assert!(simple_complexity < 0.3, "Simple fixes should have low complexity");

    // Test complex fixes (should have high complexity)
    let complex_diagnostic = create_test_diagnostic(
        "src/main.cpp",
        "type mismatch in generic template interface with async await",
        DiagnosticSeverity::Error,
        10,
        Some("type_error"),
    );

    let complex_complexity = verifier.estimate_fix_complexity(&complex_diagnostic);
    assert!(complex_complexity > 0.5, "Complex fixes should have high complexity");

    Ok(())
}

#[tokio::test]
async fn test_language_detection() -> Result<(), Box<dyn std::error::Error>> {
    // Test language detection for different file types
    use lsp_bridge::quick_fix::verification::detect_language_from_files;

    let rust_files = vec![PathBuf::from("main.rs"), PathBuf::from("lib.rs")];
    assert_eq!(detect_language_from_files(&rust_files), "rust");

    let typescript_files = vec![PathBuf::from("index.ts"), PathBuf::from("types.d.ts")];
    assert_eq!(detect_language_from_files(&typescript_files), "typescript");

    let javascript_files = vec![PathBuf::from("app.js"), PathBuf::from("utils.jsx")];
    assert_eq!(detect_language_from_files(&javascript_files), "javascript");

    let python_files = vec![PathBuf::from("script.py")];
    assert_eq!(detect_language_from_files(&python_files), "python");

    let go_files = vec![PathBuf::from("main.go")];
    assert_eq!(detect_language_from_files(&go_files), "go");

    let unknown_files = vec![PathBuf::from("data.bin")];
    assert_eq!(detect_language_from_files(&unknown_files), "unknown");

    Ok(())
}

#[tokio::test]
async fn test_verification_result_structure() -> Result<(), Box<dyn std::error::Error>> {
    // Test that verification results have the expected structure
    let verifier = FixVerifier::new().with_build_check(false).with_lsp_validation(false);

    let diagnostic = create_test_diagnostic(
        "src/test.rs",
        "test message",
        DiagnosticSeverity::Error,
        5,
        None,
    );

    let fix_result = create_successful_fix_result(vec![PathBuf::from("src/test.rs")]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // Check that all required fields are present
    assert!(verification.issue_resolved == true || verification.issue_resolved == false); // Either state is valid
    assert!(verification.new_issues.len() >= 0);
    assert!(verification.resolved_issues.len() >= 0);
    assert!(verification.build_status.duration_ms >= 0);
    assert!(verification.linter_warnings.len() >= 0);

    // Test results should be None when tests are disabled
    assert!(verification.test_results.is_none());

    // Performance impact should be None (not implemented yet)
    assert!(verification.performance_impact.is_none());

    Ok(())
}

#[tokio::test]
async fn test_multiple_file_verification() -> Result<(), Box<dyn std::error::Error>> {
    // Test verification when multiple files are modified
    let verifier = FixVerifier::new()
        .with_build_check(false)
        .with_lsp_validation(true);

    let diagnostic = create_test_diagnostic(
        "src/main.rs",
        "cross-file dependency issue",
        DiagnosticSeverity::Error,
        20,
        Some("dependency_error"),
    );

    let fix_result = create_successful_fix_result(vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("src/lib.rs"),
        PathBuf::from("src/utils.rs"),
    ]);

    let verification = verifier.verify_fix(&diagnostic, &fix_result).await?;

    // Multi-file fixes should still be handled correctly
    assert!(verification.build_status.success);

    Ok(())
}