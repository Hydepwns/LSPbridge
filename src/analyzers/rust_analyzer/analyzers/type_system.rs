use crate::analyzers::base::DiagnosticPatterns;
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::{Diagnostic, SemanticContext};
use regex::Regex;

pub struct TypeSystemAnalyzer;

impl TypeSystemAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_type_error(
        &self,
        diagnostic: &Diagnostic,
        _context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let types = DiagnosticPatterns::extract_types(&diagnostic.message);

        let mut analysis = DiagnosticAnalysis {
            category: DiagnosticCategory::TypeMismatch,
            likely_cause: "Type mismatch between expected and found types".to_string(),
            confidence: 0.85,
            related_symbols: types.clone(),
            is_cascading: false,
            fix_complexity: 2,
            insights: Vec::with_capacity(3), // Type errors typically have 1-3 insights
        };

        if diagnostic.message.contains("expected") && diagnostic.message.contains("found") {
            // Extract expected and found types
            if let (Some(expected_cap), Some(found_cap)) = (
                Regex::new(r"expected (?:type )?`([^`]+)`")
                    .unwrap()
                    .captures(&diagnostic.message),
                Regex::new(r"found (?:type )?`([^`]+)`")
                    .unwrap()
                    .captures(&diagnostic.message),
            ) {
                if let (Some(expected), Some(found)) = (expected_cap.get(1), found_cap.get(1)) {
                    let expected_type = expected.as_str();
                    let found_type = found.as_str();

                    // Reference vs value
                    if expected_type.starts_with('&') && !found_type.starts_with('&') {
                        analysis
                            .insights
                            .push("Expected a reference, found a value - add &".to_string());
                        analysis.fix_complexity = 1;
                    } else if !expected_type.starts_with('&') && found_type.starts_with('&') {
                        analysis.insights.push(
                            "Expected a value, found a reference - dereference with *".to_string(),
                        );
                        analysis.fix_complexity = 1;
                    }
                    // Result/Option handling
                    else if expected_type.contains("Result") && !found_type.contains("Result") {
                        analysis.insights.push("Wrap value in Ok()".to_string());
                    } else if expected_type.contains("Option") && !found_type.contains("Option") {
                        analysis.insights.push("Wrap value in Some()".to_string());
                    } else if found_type.contains("Result") && !expected_type.contains("Result") {
                        analysis
                            .insights
                            .push("Handle Result with ? or unwrap()".to_string());
                        analysis.category = DiagnosticCategory::AsyncError; // Might be async related
                    }
                    // String types
                    else if expected_type == "&str" && found_type == "String" {
                        analysis
                            .insights
                            .push("Convert String to &str with .as_str() or &".to_string());
                        analysis.fix_complexity = 1;
                    } else if expected_type == "String" && found_type == "&str" {
                        analysis
                            .insights
                            .push("Convert &str to String with .to_string()".to_string());
                        analysis.fix_complexity = 1;
                    }
                }
            }
        }

        analysis
    }

    pub fn analyze_trait_error(
        &self,
        diagnostic: &Diagnostic,
        _context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let identifiers = DiagnosticPatterns::extract_quoted_identifiers(&diagnostic.message);

        let mut analysis = DiagnosticAnalysis {
            category: DiagnosticCategory::TypeMismatch,
            likely_cause: "Required trait not implemented".to_string(),
            confidence: 0.8,
            related_symbols: identifiers,
            is_cascading: false,
            fix_complexity: 3,
            insights: Vec::with_capacity(3), // Trait errors typically have 2-3 insights
        };

        // Common trait errors
        if diagnostic.message.contains("doesn't implement") {
            if diagnostic.message.contains("Display") {
                analysis
                    .insights
                    .push("Implement Display trait for custom formatting".to_string());
                analysis
                    .insights
                    .push("Or use Debug with {:?} in format string".to_string());
            } else if diagnostic.message.contains("Clone") {
                analysis
                    .insights
                    .push("Derive or implement Clone trait".to_string());
                analysis
                    .insights
                    .push("#[derive(Clone)] for simple cases".to_string());
                analysis.fix_complexity = 1;
            } else if diagnostic.message.contains("Copy") {
                analysis
                    .insights
                    .push("Derive Copy trait for simple types".to_string());
                analysis
                    .insights
                    .push("#[derive(Copy, Clone)]".to_string());
                analysis.fix_complexity = 1;
            } else if diagnostic.message.contains("Send") || diagnostic.message.contains("Sync") {
                analysis.category = DiagnosticCategory::AsyncError;
                analysis
                    .insights
                    .push("Type must be thread-safe for async/threading".to_string());
                analysis
                    .insights
                    .push("Consider using Arc/Mutex for shared state".to_string());
            }
        }

        analysis
    }
}