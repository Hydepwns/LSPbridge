//! Severity conversion utilities for different diagnostic formats

use crate::core::DiagnosticSeverity;

/// Converter for diagnostic severities across different formats
pub struct SeverityConverter;

impl SeverityConverter {
    /// Convert TypeScript severity
    /// TypeScript uses: 0=message, 1=error, 2=warning, 3=suggestion
    pub fn convert_typescript(category: u8) -> DiagnosticSeverity {
        match category {
            0 => DiagnosticSeverity::Information,
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }

    /// Convert Rust analyzer severity
    pub fn convert_rust(level: &str) -> DiagnosticSeverity {
        match level.to_lowercase().as_str() {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "note" => DiagnosticSeverity::Information,
            "help" => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }

    /// Convert ESLint severity
    /// ESLint uses: 1=warning, 2=error
    pub fn convert_eslint(severity: u8) -> DiagnosticSeverity {
        match severity {
            1 => DiagnosticSeverity::Warning,
            2 => DiagnosticSeverity::Error,
            _ => DiagnosticSeverity::Warning,
        }
    }

    /// Convert LSP standard severity
    /// LSP standard: 1=Error, 2=Warning, 3=Information, 4=Hint
    pub fn convert_lsp(severity: u8) -> DiagnosticSeverity {
        match severity {
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Information,
            4 => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }
}