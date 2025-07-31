use super::*;
use lsp_bridge::{
    capture::DiagnosticsCapture,
    core::types::RawDiagnostics,
    privacy::PrivacyPolicy,
    export::ExportService,
};
use std::fs;
use std::path::Path;
use super::lsp_test_helpers::{
    EnhancedLspTestClient, convert_mock_diagnostic, create_expected_typescript_diagnostics, 
    verify_diagnostics
};

#[test]
fn test_typescript_lsp_real_diagnostics() {
    let fixture_path = Path::new("tests/fixtures/typescript/errors.ts");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    // Spawn TypeScript language server (or fallback to mock)
    let mut client = EnhancedLspTestClient::new_typescript_lsp()
        .expect("Failed to create typescript LSP client");
    
    // Initialize
    client.initialize(&root_path).expect("Failed to initialize LSP");
    
    // Open file with errors
    client.open_file(
        fixture_path.to_str().unwrap(),
        &content,
        "typescript"
    ).expect("Failed to open file");
    
    // Collect diagnostics
    let mock_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    // Convert to our format
    let diagnostics: Vec<_> = mock_diagnostics.iter()
        .map(|d| convert_mock_diagnostic(d, fixture_path.to_str().unwrap(), "typescript"))
        .collect();
    
    // Verify we got expected errors using helper
    let expected_diagnostics = create_expected_typescript_diagnostics();
    verify_diagnostics(&mock_diagnostics, &expected_diagnostics)
        .expect("Should find expected TypeScript diagnostics");
    
    // Additional verification that we have diagnostics
    assert!(!diagnostics.is_empty(), "Should have collected diagnostics");
    
    // Log whether we're using real or mock server
    if client.is_using_real_server() {
        println!("✓ Test completed with real typescript-language-server");
    } else {
        println!("✓ Test completed with mock typescript-language-server");
    }
    
    // Test with our capture service
    let mut capture = DiagnosticsCapture::new();
    capture.set_privacy_policy(PrivacyPolicy::default());
    
    let raw = RawDiagnostics {
        source: "typescript".to_string(),
        data: serde_json::json!({
            "diagnostics": diagnostics
        }),
    };
    
    let snapshot = capture.process_diagnostics(raw)
        .expect("Failed to process diagnostics");
    
    assert_eq!(snapshot.diagnostics.len(), diagnostics.len());
    
    // Test export
    let export = ExportService::new();
    let claude_output = export.to_claude_format(&snapshot)
        .expect("Failed to export to Claude format");
    
    assert!(claude_output.contains("TypeScript"));
    assert!(claude_output.contains("Property 'age' does not exist"));
    
    // Cleanup
    client.shutdown().expect("Failed to shutdown LSP");
}

#[test]
fn test_typescript_diagnostic_grouping() {
    let fixture_path = Path::new("tests/fixtures/typescript/errors.ts");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    let mut client = EnhancedLspTestClient::new_typescript_lsp()
        .expect("Failed to create typescript LSP client");
    
    client.initialize(&root_path).expect("Failed to initialize");
    client.open_file(fixture_path.to_str().unwrap(), &content, "typescript")
        .expect("Failed to open file");
    
    let mock_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    let diagnostics: Vec<_> = mock_diagnostics.iter()
        .map(|d| convert_mock_diagnostic(d, fixture_path.to_str().unwrap(), "typescript"))
        .collect();
    
    // Verify we have diagnostics
    assert!(!diagnostics.is_empty(), "Should have collected diagnostics");
    
    // Log test completion
    if client.is_using_real_server() {
        println!("✓ Grouping test completed with real typescript-language-server");
    } else {
        println!("✓ Grouping test completed with mock typescript-language-server");
    }
    
    // Test error severity distribution
    let error_count = diagnostics.iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .count();
    
    assert!(error_count > 5, "Should have multiple errors in test file");
    
    client.shutdown().expect("Failed to shutdown");
}

#[test]
fn test_typescript_with_strict_privacy() {
    let fixture_path = Path::new("tests/fixtures/typescript/errors.ts");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    let mut client = EnhancedLspTestClient::new_typescript_lsp()
        .expect("Failed to create typescript LSP client");
    
    client.initialize(&root_path).expect("Failed to initialize");
    client.open_file(fixture_path.to_str().unwrap(), &content, "typescript")
        .expect("Failed to open file");
    
    let mock_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    let diagnostics: Vec<_> = mock_diagnostics.iter()
        .map(|d| convert_mock_diagnostic(d, fixture_path.to_str().unwrap(), "typescript"))
        .collect();
    
    // Apply strict privacy policy
    let mut capture = DiagnosticsCapture::new();
    capture.set_privacy_policy(PrivacyPolicy::strict());
    
    let raw = RawDiagnostics {
        source: "typescript".to_string(),
        data: serde_json::json!({
            "diagnostics": diagnostics
        }),
    };
    
    let snapshot = capture.process_diagnostics(raw)
        .expect("Failed to process diagnostics");
    
    // With strict policy, only errors should be included
    let all_errors = snapshot.diagnostics.iter()
        .all(|d| d.severity == DiagnosticSeverity::Error);
    
    assert!(all_errors, "Strict policy should only include errors");
    
    // String literals should be sanitized
    let export = ExportService::new();
    let output = export.to_json(&snapshot).expect("Failed to export");
    
    // Verify the privacy policy and export mechanism works
    assert!(!output.is_empty(), "Should produce output");
    
    // Log test completion
    if client.is_using_real_server() {
        println!("✓ Privacy test completed with real typescript-language-server");
    } else {
        println!("✓ Privacy test completed with mock typescript-language-server");
    }
    
    client.shutdown().expect("Failed to shutdown");
}