//! Helper utilities for LSP integration tests
//! 
//! This module provides utilities to detect available LSP servers
//! and fallback to mock servers when real ones aren't available.

use super::mock_lsp_server::{create_rust_analyzer_mock, create_typescript_mock, MockLspServer};
use lsp_bridge::core::{Diagnostic, DiagnosticSeverity, Position, Range};
use std::process::Command;
use uuid::Uuid;

/// Check if a command exists in the system PATH
pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if rust-analyzer is available
pub fn rust_analyzer_available() -> bool {
    command_exists("rust-analyzer")
}

/// Check if typescript-language-server is available
pub fn typescript_lsp_available() -> bool {
    command_exists("typescript-language-server")
}

/// Enhanced LspTestClient that can fallback to mock server
pub struct EnhancedLspTestClient {
    real_client: Option<super::LspTestClient>,
    mock_server: Option<MockLspServer>,
    server_type: LspServerType,
}

#[derive(Debug, Clone)]
pub enum LspServerType {
    RustAnalyzer,
    TypeScriptLsp,
}

impl EnhancedLspTestClient {
    /// Create a new enhanced client for Rust analyzer
    pub fn new_rust_analyzer() -> Result<Self, Box<dyn std::error::Error>> {
        if rust_analyzer_available() {
            println!("✓ Using real rust-analyzer");
            Ok(Self {
                real_client: Some(super::LspTestClient::spawn_rust_analyzer()?),
                mock_server: None,
                server_type: LspServerType::RustAnalyzer,
            })
        } else {
            println!("⚠ rust-analyzer not found, using mock server");
            Ok(Self {
                real_client: None,
                mock_server: Some(create_rust_analyzer_mock()),
                server_type: LspServerType::RustAnalyzer,
            })
        }
    }

    /// Create a new enhanced client for TypeScript LSP
    pub fn new_typescript_lsp() -> Result<Self, Box<dyn std::error::Error>> {
        if typescript_lsp_available() {
            println!("✓ Using real typescript-language-server");
            Ok(Self {
                real_client: Some(super::LspTestClient::spawn_typescript_lsp()?),
                mock_server: None,
                server_type: LspServerType::TypeScriptLsp,
            })
        } else {
            println!("⚠ typescript-language-server not found, using mock server");
            Ok(Self {
                real_client: None,
                mock_server: Some(create_typescript_mock()),
                server_type: LspServerType::TypeScriptLsp,
            })
        }
    }

    /// Initialize the LSP client
    pub fn initialize(&mut self, root_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut client) = self.real_client {
            client.initialize(root_path)
        } else {
            // Mock server doesn't need explicit initialization
            Ok(())
        }
    }

    /// Open a file in the LSP client
    pub fn open_file(
        &mut self,
        file_path: &str,
        content: &str,
        language: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut client) = self.real_client {
            client.open_file(file_path, content, language)
        } else {
            // Mock server doesn't need explicit file opening
            Ok(())
        }
    }

    /// Collect diagnostics from the LSP client
    pub fn collect_diagnostics(&mut self) -> Result<Vec<MockDiagnostic>, Box<dyn std::error::Error>> {
        if let Some(ref mut client) = self.real_client {
            // Convert real LSP diagnostics to our mock format
            // In the real implementation, this would collect from LSP server
            // For now, return empty since we don't have the real LSP protocol implementation
            Ok(vec![])
        } else if let Some(ref mock_server) = self.mock_server {
            // Return mock diagnostics
            let diagnostics = mock_server.diagnostics.lock().unwrap();
            Ok(diagnostics.clone())
        } else {
            Ok(vec![])
        }
    }

    /// Shutdown the LSP client
    pub fn shutdown(self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = self.real_client {
            client.shutdown()
        } else {
            Ok(())
        }
    }

    /// Check if using real LSP server
    pub fn is_using_real_server(&self) -> bool {
        self.real_client.is_some()
    }
}

// Re-export types for convenience
pub use super::mock_lsp_server::{MockDiagnostic, Position as MockPosition, Range as MockRange};

/// Convert mock diagnostic to LSP Bridge diagnostic
pub fn convert_mock_diagnostic(
    mock_diag: &MockDiagnostic,
    file_path: &str,
    source: &str,
) -> Diagnostic {
    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: file_path.to_string(),
        range: Range {
            start: Position {
                line: mock_diag.range.start.line,
                character: mock_diag.range.start.character,
            },
            end: Position {
                line: mock_diag.range.end.line,
                character: mock_diag.range.end.character,
            },
        },
        severity: match mock_diag.severity {
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Information,
            4 => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        },
        message: mock_diag.message.clone(),
        code: mock_diag.code.clone(),
        source: source.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create expected diagnostics for Rust tests
pub fn create_expected_rust_diagnostics() -> Vec<ExpectedDiagnostic> {
    vec![
        ExpectedDiagnostic {
            message_contains: "cannot find value",
            variable_name: Some("undefined_var".to_string()),
            description: "undefined variable error",
        },
        ExpectedDiagnostic {
            message_contains: "mismatched types",
            variable_name: None,
            description: "type mismatch error",
        },
        ExpectedDiagnostic {
            message_contains: "cannot borrow",
            variable_name: None,
            description: "borrow checker error",
        },
    ]
}

/// Create expected diagnostics for TypeScript tests
pub fn create_expected_typescript_diagnostics() -> Vec<ExpectedDiagnostic> {
    vec![
        ExpectedDiagnostic {
            message_contains: "Cannot find name",
            variable_name: Some("undefined_var".to_string()),
            description: "undefined identifier error",
        },
        ExpectedDiagnostic {
            message_contains: "not assignable to type",
            variable_name: None,
            description: "type assignment error",
        },
    ]
}

/// Represents an expected diagnostic for testing
#[derive(Debug, Clone)]
pub struct ExpectedDiagnostic {
    pub message_contains: &'static str,
    pub variable_name: Option<String>,
    pub description: &'static str,
}

/// Verify that diagnostics match expected patterns
pub fn verify_diagnostics(
    diagnostics: &[MockDiagnostic], 
    expected: &[ExpectedDiagnostic]
) -> Result<(), String> {
    if diagnostics.is_empty() {
        return Err("No diagnostics found".to_string());
    }

    for expected_diag in expected {
        let found = diagnostics.iter().any(|d| {
            let message_matches = d.message.contains(expected_diag.message_contains);
            let variable_matches = expected_diag.variable_name.as_ref()
                .map(|var| d.message.contains(var))
                .unwrap_or(true);
            
            message_matches && variable_matches
        });

        if !found {
            return Err(format!(
                "Expected diagnostic not found: {} ({})", 
                expected_diag.description,
                expected_diag.message_contains
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_detection() {
        // These should always exist on Unix systems
        assert!(command_exists("ls") || command_exists("dir"));
    }

    #[test]
    fn test_expected_diagnostics_creation() {
        let rust_diags = create_expected_rust_diagnostics();
        assert_eq!(rust_diags.len(), 3);
        
        let ts_diags = create_expected_typescript_diagnostics();
        assert_eq!(ts_diags.len(), 2);
    }

    #[test]
    fn test_diagnostic_verification() {
        let mock_diags = vec![
            MockDiagnostic {
                uri: "test.rs".to_string(),
                range: crate::mock_lsp_server::Range {
                    start: crate::mock_lsp_server::Position { line: 0, character: 0 },
                    end: crate::mock_lsp_server::Position { line: 0, character: 10 },
                },
                severity: 1,
                message: "cannot find value `undefined_var` in this scope".to_string(),
                code: Some("E0425".to_string()),
            }
        ];

        let expected = vec![
            ExpectedDiagnostic {
                message_contains: "cannot find value",
                variable_name: Some("undefined_var".to_string()),
                description: "undefined variable",
            }
        ];

        assert!(verify_diagnostics(&mock_diags, &expected).is_ok());
    }
}