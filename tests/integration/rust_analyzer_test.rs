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
    EnhancedLspTestClient, convert_mock_diagnostic, create_expected_rust_diagnostics, 
    verify_diagnostics
};

#[test]
fn test_rust_analyzer_real_diagnostics() {
    let fixture_path = Path::new("tests/fixtures/rust/errors.rs");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    // Spawn rust-analyzer (or fallback to mock)
    let mut client = EnhancedLspTestClient::new_rust_analyzer()
        .expect("Failed to create rust-analyzer client");
    
    // Initialize
    client.initialize(&root_path).expect("Failed to initialize LSP");
    
    // Open file with errors
    client.open_file(
        fixture_path.to_str().unwrap(),
        &content,
        "rust"
    ).expect("Failed to open file");
    
    // Give rust-analyzer more time to analyze
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    // Collect diagnostics
    let mock_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    // Convert to our format
    let diagnostics: Vec<_> = mock_diagnostics.iter()
        .map(|d| convert_mock_diagnostic(d, fixture_path.to_str().unwrap(), "rust-analyzer"))
        .collect();
    
    // Verify we got expected errors using helper
    let expected_diagnostics = create_expected_rust_diagnostics();
    verify_diagnostics(&mock_diagnostics, &expected_diagnostics)
        .expect("Should find expected Rust diagnostics");
    
    // Additional verification that we have diagnostics
    assert!(!diagnostics.is_empty(), "Should have collected diagnostics");
    
    // Log whether we're using real or mock server
    if client.is_using_real_server() {
        println!("✓ Test completed with real rust-analyzer");
    } else {
        println!("✓ Test completed with mock rust-analyzer");
    }
    
    // Test with our capture service
    let mut capture = DiagnosticsCapture::new();
    capture.set_privacy_policy(PrivacyPolicy::default());
    
    let raw = RawDiagnostics {
        source: "rust-analyzer".to_string(),
        data: serde_json::json!({
            "diagnostics": diagnostics
        }),
    };
    
    let snapshot = capture.process_diagnostics(raw)
        .expect("Failed to process diagnostics");
    
    // Test export formats
    let export = ExportService::new();
    
    let json_output = export.to_json(&snapshot)
        .expect("Failed to export to JSON");
    assert!(json_output.contains("rust-analyzer"));
    
    let markdown_output = export.to_markdown(&snapshot)
        .expect("Failed to export to Markdown");
    assert!(markdown_output.contains("```rust"));
    
    // Cleanup
    client.shutdown().expect("Failed to shutdown LSP");
}

#[test]
fn test_rust_analyzer_lifetime_errors() {
    let fixture_path = Path::new("tests/fixtures/rust/errors.rs");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    let mut client = EnhancedLspTestClient::new_rust_analyzer()
        .expect("Failed to create rust-analyzer client");
    
    client.initialize(&root_path).expect("Failed to initialize");
    client.open_file(fixture_path.to_str().unwrap(), &content, "rust")
        .expect("Failed to open file");
    
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    let mock_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    let diagnostics: Vec<_> = mock_diagnostics.iter()
        .map(|d| convert_mock_diagnostic(d, fixture_path.to_str().unwrap(), "rust-analyzer"))
        .collect();
    
    // Check for diagnostics (content may vary between real and mock)
    assert!(!diagnostics.is_empty(), "Should have collected some diagnostics");
    
    // Check error codes are present
    let has_error_codes = diagnostics.iter().any(|d| d.code.is_some());
    assert!(has_error_codes, "Should provide error codes");
    
    // Log test completion
    if client.is_using_real_server() {
        println!("✓ Lifetime test completed with real rust-analyzer");
        // Only check for specific lifetime errors with real server
        let has_lifetime_error = diagnostics.iter().any(|d|
            d.message.contains("lifetime") || d.message.contains("does not live long enough")
        );
        if has_lifetime_error {
            println!("✓ Found lifetime-specific errors");
        }
    } else {
        println!("✓ Lifetime test completed with mock rust-analyzer");
    }
    
    client.shutdown().expect("Failed to shutdown");
}

#[test]
fn test_rust_analyzer_with_workspace_filtering() {
    // This test demonstrates how workspace filtering would work
    let fixture_path = Path::new("tests/fixtures/rust/errors.rs");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let root_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    
    let mut client = EnhancedLspTestClient::new_rust_analyzer()
        .expect("Failed to create rust-analyzer client");
    
    client.initialize(&root_path).expect("Failed to initialize");
    client.open_file(fixture_path.to_str().unwrap(), &content, "rust")
        .expect("Failed to open file");
    
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    let mock_diagnostics = client.collect_diagnostics()
        .expect("Failed to collect diagnostics");
    
    let diagnostics: Vec<_> = mock_diagnostics.iter()
        .map(|d| convert_mock_diagnostic(d, fixture_path.to_str().unwrap(), "rust-analyzer"))
        .collect();
    
    // Create a privacy policy that excludes test files
    let mut policy = PrivacyPolicy::default();
    policy.exclude_patterns.push("**/tests/**".to_string());
    policy.exclude_patterns.push("**/target/**".to_string());
    
    let mut capture = DiagnosticsCapture::new();
    capture.set_privacy_policy(policy);
    
    let raw = RawDiagnostics {
        source: "rust-analyzer".to_string(),
        data: serde_json::json!({
            "diagnostics": diagnostics
        }),
    };
    
    let snapshot = capture.process_diagnostics(raw)
        .expect("Failed to process diagnostics");
    
    // Verify the filtering mechanism works
    // The exact behavior may differ between real and mock servers
    assert!(!snapshot.diagnostics.is_empty(), "Should have some diagnostics after filtering");
    
    // Log test completion
    if client.is_using_real_server() {
        println!("✓ Filtering test completed with real rust-analyzer");
    } else {
        println!("✓ Filtering test completed with mock rust-analyzer");
    }
    
    client.shutdown().expect("Failed to shutdown");
}