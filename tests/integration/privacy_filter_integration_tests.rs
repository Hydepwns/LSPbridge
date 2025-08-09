//! # Privacy Filter Integration Tests
//!
//! Tests to verify that privacy policy integration works correctly with the capture service
//! and that diagnostics are properly filtered based on configured privacy settings.
//! This focuses on the actual integration between privacy filters and capture services.

use lsp_bridge::{
    capture::{CaptureService, MemoryCache},
    core::{
        Diagnostic, DiagnosticSeverity, Position, Range, PrivacyPolicy,
        FormatConverter as FormatConverterTrait,
        PrivacyFilter as PrivacyFilterTrait,
    },
    format::format_converter::FormatConverter,
    privacy::PrivacyFilter,
};
use std::collections::HashMap;
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

#[tokio::test]
async fn test_privacy_filter_default_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that default privacy policy works in capture service integration
    let cache = MemoryCache::new(100, 3600);
    let privacy_filter = PrivacyFilter::with_default_policy();
    let format_converter = FormatConverter::new();
    
    let capture_service = CaptureService::new(cache, privacy_filter, format_converter);
    
    let diagnostics = vec![
        create_test_diagnostic("src/main.rs", "undefined variable `x`", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "unused import", DiagnosticSeverity::Warning, 5),
        create_test_diagnostic("tests/test.rs", "deprecated function", DiagnosticSeverity::Information, 15),
    ];
    
    // Test privacy filter directly
    let policy = capture_service.get_privacy_policy();
    assert!(!policy.include_only_errors); // Default policy should include all severities
    assert!(policy.sanitize_strings); // Default policy DOES sanitize for security
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_filter_strict_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that strict privacy policy works in capture service integration
    let cache = MemoryCache::new(100, 3600);
    let privacy_filter = PrivacyFilter::with_strict_policy();
    let format_converter = FormatConverter::new();
    
    let capture_service = CaptureService::new(cache, privacy_filter, format_converter);
    
    // Test privacy filter configuration
    let policy = capture_service.get_privacy_policy();
    assert!(policy.include_only_errors); // Strict policy should only include errors
    assert!(policy.sanitize_strings); // Strict policy should sanitize
    assert!(policy.anonymize_file_paths); // Strict policy should anonymize paths
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_filter_permissive_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that permissive privacy policy works in capture service integration  
    let cache = MemoryCache::new(100, 3600);
    let privacy_filter = PrivacyFilter::with_permissive_policy();
    let format_converter = FormatConverter::new();
    
    let capture_service = CaptureService::new(cache, privacy_filter, format_converter);
    
    // Test privacy filter configuration
    let policy = capture_service.get_privacy_policy();
    assert!(!policy.include_only_errors); // Permissive policy should include all severities
    assert!(!policy.sanitize_strings); // Permissive policy should not sanitize
    assert!(!policy.anonymize_file_paths); // Permissive policy should not anonymize
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_filter_direct_filtering() -> Result<(), Box<dyn std::error::Error>> {
    // Test that privacy filter actually filters diagnostics as expected
    let diagnostics = vec![
        create_test_diagnostic("src/main.rs", "undefined variable `x`", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "unused import", DiagnosticSeverity::Warning, 5),
        create_test_diagnostic("tests/test.rs", "deprecated function", DiagnosticSeverity::Information, 15),
    ];
    
    // Test with default policy (should include all)
    let default_filter = PrivacyFilter::with_default_policy();
    let default_filtered = default_filter.apply(diagnostics.clone())?;
    assert_eq!(default_filtered.len(), 3);
    
    // Test with strict policy (should only include errors)
    let strict_filter = PrivacyFilter::with_strict_policy();
    let strict_filtered = strict_filter.apply(diagnostics.clone())?;
    assert_eq!(strict_filtered.len(), 1);
    assert_eq!(strict_filtered[0].severity, DiagnosticSeverity::Error);
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_filter_string_sanitization() -> Result<(), Box<dyn std::error::Error>> {
    // Test that string sanitization works correctly
    // Test string sanitization only 
    let string_diagnostic = create_test_diagnostic(
        "src/main.rs",
        "Variable \"secret_key\" is undefined", 
        DiagnosticSeverity::Error,
        10
    );
    
    let policy_strings = PrivacyPolicy {
        sanitize_strings: true,
        sanitize_comments: false, // Only test strings
        ..PrivacyPolicy::default()
    };
    
    let privacy_filter = PrivacyFilter::new(policy_strings);
    let filtered = privacy_filter.apply(vec![string_diagnostic])?;
    
    assert_eq!(filtered.len(), 1);
    let error_diagnostic = &filtered[0];
    assert!(error_diagnostic.message.contains("[STRING]"));
    assert!(!error_diagnostic.message.contains("secret_key"));
    
    // Test comment sanitization only
    let comment_diagnostic = create_test_diagnostic(
        "src/lib.rs",
        "// TODO: remove this hardcoded password",
        DiagnosticSeverity::Warning,
        5
    );
    
    let policy_comments = PrivacyPolicy {
        sanitize_strings: false, // Only test comments
        sanitize_comments: true,
        ..PrivacyPolicy::default()
    };
    
    let privacy_filter_comments = PrivacyFilter::new(policy_comments);
    let filtered_comments = privacy_filter_comments.apply(vec![comment_diagnostic])?;
    
    assert_eq!(filtered_comments.len(), 1);
    let warning_diagnostic = &filtered_comments[0];
    assert!(warning_diagnostic.message.contains("[COMMENT]"));
    assert!(!warning_diagnostic.message.contains("hardcoded password"));
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_filter_path_anonymization() -> Result<(), Box<dyn std::error::Error>> {
    // Test that file path anonymization works correctly
    let diagnostics = vec![
        create_test_diagnostic(
            "/home/user/secret-project/src/main.rs",
            "Error message",
            DiagnosticSeverity::Error,
            10
        ),
    ];
    
    // Test with policy that enables path anonymization
    let policy = PrivacyPolicy {
        anonymize_file_paths: true,
        ..PrivacyPolicy::default()
    };
    
    let privacy_filter = PrivacyFilter::new(policy);
    let filtered = privacy_filter.apply(diagnostics)?;
    
    assert_eq!(filtered.len(), 1);
    
    // Check that path is anonymized
    let diagnostic = &filtered[0];
    assert!(diagnostic.file.contains("[DIR_"));
    assert!(diagnostic.file.contains("]/main.rs"));
    assert!(!diagnostic.file.contains("secret-project"));
    assert!(!diagnostic.file.contains("/home/user"));
    
    Ok(())
}

#[tokio::test] 
async fn test_privacy_filter_exclusion_patterns() -> Result<(), Box<dyn std::error::Error>> {
    // Test that exclusion patterns work correctly
    let diagnostics = vec![
        create_test_diagnostic("src/main.rs", "Normal error", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/secret/api.rs", "Should be excluded", DiagnosticSeverity::Error, 5),
        create_test_diagnostic("config.env", "Also excluded", DiagnosticSeverity::Warning, 1),
        create_test_diagnostic("src/lib.rs", "Another normal error", DiagnosticSeverity::Warning, 15),
    ];
    
    // Test with exclusion patterns
    let policy = PrivacyPolicy {
        exclude_patterns: vec!["**/secret/**".to_string(), "*.env".to_string()],
        ..PrivacyPolicy::default()
    };
    
    let privacy_filter = PrivacyFilter::new(policy);
    let filtered = privacy_filter.apply(diagnostics)?;
    
    // Should exclude files matching patterns
    assert_eq!(filtered.len(), 2);
    assert!(!filtered.iter().any(|d| d.file.contains("secret")));
    assert!(!filtered.iter().any(|d| d.file.contains(".env")));
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_filter_max_diagnostics_per_file() -> Result<(), Box<dyn std::error::Error>> {
    // Test that per-file diagnostic limits work correctly
    let diagnostics = vec![
        create_test_diagnostic("src/lib.rs", "First diagnostic", DiagnosticSeverity::Error, 10),
        create_test_diagnostic("src/lib.rs", "Second diagnostic", DiagnosticSeverity::Warning, 15),
        create_test_diagnostic("src/lib.rs", "Third diagnostic (should be limited)", DiagnosticSeverity::Information, 20),
        create_test_diagnostic("src/main.rs", "Different file", DiagnosticSeverity::Error, 5),
    ];
    
    // Test with per-file limit
    let policy = PrivacyPolicy {
        max_diagnostics_per_file: 2,
        ..PrivacyPolicy::default()
    };
    
    let privacy_filter = PrivacyFilter::new(policy);
    let filtered = privacy_filter.apply(diagnostics)?;
    
    // Should limit diagnostics per file, prioritizing by severity
    assert!(filtered.len() <= 3); // 2 from lib.rs + 1 from main.rs
    
    // Count diagnostics per file
    let mut file_counts = HashMap::new();
    for diagnostic in &filtered {
        *file_counts.entry(&diagnostic.file).or_insert(0) += 1;
    }
    
    for count in file_counts.values() {
        assert!(*count <= 2, "File has more than 2 diagnostics: {}", count);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_privacy_policy_get_set_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that privacy policy can be retrieved and compared
    let policy = PrivacyPolicy {
        exclude_patterns: vec!["*.secret".to_string()],
        sanitize_strings: true,
        sanitize_comments: false,
        include_only_errors: true,
        max_diagnostics_per_file: 5,
        anonymize_file_paths: false,
        encrypt_exports: true,
    };
    
    let cache = MemoryCache::new(100, 3600);
    let privacy_filter = PrivacyFilter::new(policy.clone());
    let format_converter = FormatConverter::new();
    
    let capture_service = CaptureService::new(cache, privacy_filter, format_converter);
    
    // Verify that the policy can be retrieved and matches what was set
    let retrieved_policy = capture_service.get_privacy_policy();
    assert_eq!(retrieved_policy.exclude_patterns, policy.exclude_patterns);
    assert_eq!(retrieved_policy.sanitize_strings, policy.sanitize_strings);
    assert_eq!(retrieved_policy.sanitize_comments, policy.sanitize_comments);
    assert_eq!(retrieved_policy.include_only_errors, policy.include_only_errors);
    assert_eq!(retrieved_policy.max_diagnostics_per_file, policy.max_diagnostics_per_file);
    assert_eq!(retrieved_policy.anonymize_file_paths, policy.anonymize_file_paths);
    assert_eq!(retrieved_policy.encrypt_exports, policy.encrypt_exports);
    
    Ok(())
}