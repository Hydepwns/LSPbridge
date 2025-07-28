use super::*;
use lsp_bridge::{
    capture::DiagnosticsCapture,
    core::types::RawDiagnostics,
    privacy::PrivacyPolicy,
    export::ExportService,
};
use std::fs;
use std::path::Path;

#[test]
#[ignore] // Run with --ignored flag when typescript-language-server is installed
fn test_typescript_lsp_real_diagnostics() {
    let fixture_path = Path::new("tests/fixtures/typescript/errors.ts");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    // Spawn TypeScript language server
    let mut client = LspTestClient::spawn_typescript_lsp()
        .expect("Failed to spawn typescript-language-server. Make sure it's installed: npm install -g typescript-language-server typescript");
    
    // Initialize
    client.initialize(&root_path).expect("Failed to initialize LSP");
    
    // Open file with errors
    client.open_file(
        fixture_path.to_str().unwrap(),
        &content,
        "typescript"
    ).expect("Failed to open file");
    
    // Collect diagnostics
    let lsp_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    // Convert to our format
    let diagnostics: Vec<_> = lsp_diagnostics.iter()
        .map(|d| convert_lsp_diagnostic(d, fixture_path.to_str().unwrap(), "typescript"))
        .collect();
    
    // Verify we got expected errors
    assert!(!diagnostics.is_empty(), "Should have collected diagnostics");
    
    // Check for specific expected errors
    let has_age_error = diagnostics.iter().any(|d| 
        d.message.contains("Property 'age' does not exist on type 'User'")
    );
    assert!(has_age_error, "Should detect missing property error");
    
    let has_unknown_function = diagnostics.iter().any(|d|
        d.message.contains("Cannot find name 'unknownFunction'")
    );
    assert!(has_unknown_function, "Should detect unknown function error");
    
    let has_type_error = diagnostics.iter().any(|d|
        d.message.contains("Type 'string' is not assignable to type 'number'")
    );
    assert!(has_type_error, "Should detect type mismatch error");
    
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
#[ignore]
fn test_typescript_diagnostic_grouping() {
    let fixture_path = Path::new("tests/fixtures/typescript/errors.ts");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    let mut client = LspTestClient::spawn_typescript_lsp()
        .expect("Failed to spawn typescript-language-server");
    
    client.initialize(&root_path).expect("Failed to initialize");
    client.open_file(fixture_path.to_str().unwrap(), &content, "typescript")
        .expect("Failed to open file");
    
    let lsp_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    let diagnostics: Vec<_> = lsp_diagnostics.iter()
        .map(|d| convert_lsp_diagnostic(d, fixture_path.to_str().unwrap(), "typescript"))
        .collect();
    
    // Check for related errors (Calculator class has multiple related errors)
    let calculator_errors: Vec<_> = diagnostics.iter()
        .filter(|d| d.message.contains("history"))
        .collect();
    
    assert!(calculator_errors.len() >= 2, "Should have multiple related 'history' errors");
    
    // Test error severity distribution
    let error_count = diagnostics.iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .count();
    
    assert!(error_count > 5, "Should have multiple errors in test file");
    
    client.shutdown().expect("Failed to shutdown");
}

#[test]
#[ignore]
fn test_typescript_with_strict_privacy() {
    let fixture_path = Path::new("tests/fixtures/typescript/errors.ts");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    let mut client = LspTestClient::spawn_typescript_lsp()
        .expect("Failed to spawn typescript-language-server");
    
    client.initialize(&root_path).expect("Failed to initialize");
    client.open_file(fixture_path.to_str().unwrap(), &content, "typescript")
        .expect("Failed to open file");
    
    let lsp_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    let diagnostics: Vec<_> = lsp_diagnostics.iter()
        .map(|d| convert_lsp_diagnostic(d, fixture_path.to_str().unwrap(), "typescript"))
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
    
    assert!(!output.contains("\"not a number\""), "String literals should be sanitized");
    
    client.shutdown().expect("Failed to shutdown");
}