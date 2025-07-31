//! Filtering engine for query execution
//!
//! This module provides filtering capabilities for different data types and sources.
//! It includes pattern matching, regex validation, and security measures to prevent
//! injection attacks and denial of service.

use crate::query::parser::{
    QueryFilter, ComparisonFilter, 
};
use crate::query::parser::ast::{
    CategoryFilter, Comparison, MessageFilter, PathFilter, SeverityFilter,
};
use super::types::{FileStatistics, Value};
use crate::core::{Diagnostic, DiagnosticSeverity};
use anyhow::{anyhow, Result};
use regex::Regex;
use std::path::PathBuf;

/// Main filtering engine that applies query filters to different data types
pub struct FilterEngine;

impl FilterEngine {
    /// Create a new filter engine
    pub fn new() -> Self {
        Self
    }

    /// Apply filters to diagnostic data
    pub fn apply_diagnostic_filters(
        &self,
        diagnostics: &[(PathBuf, Diagnostic)],
        filters: &[QueryFilter],
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        let mut result = diagnostics.to_vec();

        for filter in filters {
            result = match filter {
                QueryFilter::Path(path_filter) => self.filter_diagnostics_by_path(result, path_filter)?,
                QueryFilter::Severity(severity_filter) => {
                    self.filter_diagnostics_by_severity(result, severity_filter)?
                }
                QueryFilter::Category(category_filter) => {
                    self.filter_diagnostics_by_category(result, category_filter)?
                }
                QueryFilter::Message(message_filter) => {
                    self.filter_diagnostics_by_message(result, message_filter)?
                }
                _ => result, // Time range and other filters handled elsewhere
            };
        }

        Ok(result)
    }

    /// Apply filters to file statistics data
    pub fn apply_file_filters(
        &self,
        files: Vec<(PathBuf, FileStatistics)>,
        filters: &[QueryFilter],
    ) -> Result<Vec<(PathBuf, FileStatistics)>> {
        let mut result = files;

        for filter in filters {
            result = match filter {
                QueryFilter::Path(path_filter) => {
                    self.filter_files_by_path(result, path_filter)?
                }
                QueryFilter::FileCount(comparison_filter) => {
                    self.filter_files_by_count(result, comparison_filter)?
                }
                _ => result, // Other filters not applicable to files
            };
        }

        Ok(result)
    }

    /// Filter diagnostics by file path
    fn filter_diagnostics_by_path(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &PathFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        if filter.is_regex {
            let re = Self::validate_and_compile_regex(&filter.pattern)?;
            Ok(diagnostics
                .into_iter()
                .filter(|(path, _)| re.is_match(path.to_str().unwrap_or("")))
                .collect())
        } else {
            Self::validate_pattern_length(&filter.pattern)?;
            Ok(diagnostics
                .into_iter()
                .filter(|(path, _)| path.to_str().unwrap_or("").contains(&filter.pattern))
                .collect())
        }
    }

    /// Filter diagnostics by severity level
    fn filter_diagnostics_by_severity(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &SeverityFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        Ok(diagnostics
            .into_iter()
            .filter(|(_, diagnostic)| {
                Self::compare_severity(diagnostic.severity, filter.severity, filter.comparison.clone())
            })
            .collect())
    }

    /// Filter diagnostics by category/code
    fn filter_diagnostics_by_category(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &CategoryFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        Ok(diagnostics
            .into_iter()
            .filter(|(_, diagnostic)| {
                if let Some(code) = &diagnostic.code {
                    filter.categories.iter().any(|c| code.contains(c))
                } else {
                    false
                }
            })
            .collect())
    }

    /// Filter diagnostics by message content
    fn filter_diagnostics_by_message(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &MessageFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        if filter.is_regex {
            let re = Self::validate_and_compile_regex(&filter.pattern)?;
            Ok(diagnostics
                .into_iter()
                .filter(|(_, diagnostic)| re.is_match(&diagnostic.message))
                .collect())
        } else {
            Self::validate_pattern_length(&filter.pattern)?;
            Ok(diagnostics
                .into_iter()
                .filter(|(_, diagnostic)| diagnostic.message.contains(&filter.pattern))
                .collect())
        }
    }

    /// Filter files by path pattern
    fn filter_files_by_path(
        &self,
        files: Vec<(PathBuf, FileStatistics)>,
        filter: &PathFilter,
    ) -> Result<Vec<(PathBuf, FileStatistics)>> {
        if filter.is_regex {
            let re = Self::validate_and_compile_regex(&filter.pattern)?;
            Ok(files
                .into_iter()
                .filter(|(path, _)| re.is_match(path.to_str().unwrap_or("")))
                .collect())
        } else {
            Self::validate_pattern_length(&filter.pattern)?;
            Ok(files
                .into_iter()
                .filter(|(path, _)| {
                    path.to_str().unwrap_or("").contains(&filter.pattern)
                })
                .collect())
        }
    }

    /// Filter files by diagnostic count
    fn filter_files_by_count(
        &self,
        files: Vec<(PathBuf, FileStatistics)>,
        filter: &ComparisonFilter,
    ) -> Result<Vec<(PathBuf, FileStatistics)>> {
        let target_count = filter.value as usize;
        
        Ok(files
            .into_iter()
            .filter(|(_, stats)| {
                let actual_count = match filter.field.as_str() {
                    "error_count" => stats.error_count,
                    "warning_count" => stats.warning_count,
                    "total_count" | "file_count" => stats.total_count,
                    _ => stats.total_count,
                };
                
                Self::compare_numbers(actual_count, target_count, filter.comparison.clone())
            })
            .collect())
    }

    /// Compare severity levels based on comparison operator
    fn compare_severity(
        actual: DiagnosticSeverity,
        target: DiagnosticSeverity,
        comparison: Comparison,
    ) -> bool {
        match comparison {
            Comparison::Equal => actual == target,
            Comparison::NotEqual => actual != target,
            Comparison::GreaterThan => (actual as u8) > (target as u8),
            Comparison::LessThan => (actual as u8) < (target as u8),
            Comparison::GreaterThanOrEqual => (actual as u8) >= (target as u8),
            Comparison::LessThanOrEqual => (actual as u8) <= (target as u8),
        }
    }

    /// Compare numeric values based on comparison operator
    fn compare_numbers(actual: usize, target: usize, comparison: Comparison) -> bool {
        match comparison {
            Comparison::Equal => actual == target,
            Comparison::NotEqual => actual != target,
            Comparison::GreaterThan => actual > target,
            Comparison::LessThan => actual < target,
            Comparison::GreaterThanOrEqual => actual >= target,
            Comparison::LessThanOrEqual => actual <= target,
        }
    }

    /// Validate and compile a regex pattern with security checks
    ///
    /// This prevents regex injection attacks and ensures patterns are safe to compile.
    /// It checks for potentially dangerous patterns and validates syntax.
    fn validate_and_compile_regex(pattern: &str) -> Result<Regex> {
        // Basic safety checks
        if pattern.is_empty() {
            return Err(anyhow!("Empty regex pattern"));
        }
        
        if pattern.len() > 1024 {
            return Err(anyhow!("Regex pattern too long (max 1024 characters)"));
        }

        // Check for potentially dangerous patterns that could cause ReDoS or security issues
        let dangerous_patterns = [
            "(?R)",          // Recursive patterns
            "(?0)",          // Recursive patterns
            "\\g<",          // Named backreferences
            "\\g'",          // Named backreferences  
            "(?&",           // Subroutine calls
            "(?P>",          // Named subroutine calls
            "\\K",           // Keep match start
            "(*SKIP)",       // Skip patterns
            "(*FAIL)",       // Fail patterns
            "(*ACCEPT)",     // Accept patterns
            "(*COMMIT)",     // Commit patterns
            "(*PRUNE)",      // Prune patterns
        ];

        for dangerous in &dangerous_patterns {
            if pattern.contains(dangerous) {
                return Err(anyhow!("Potentially unsafe regex pattern: contains {}", dangerous));
            }
        }

        // Check for excessive repetition that could cause ReDoS
        if pattern.contains("*+") || pattern.contains("+*") || pattern.contains("++") {
            return Err(anyhow!("Potentially unsafe regex pattern: excessive repetition"));
        }

        // Validate character classes
        if pattern.contains("[[:") && !pattern.contains(":]]") {
            return Err(anyhow!("Invalid POSIX character class in regex"));
        }

        // Try to compile the regex
        match Regex::new(pattern) {
            Ok(regex) => {
                // Additional runtime safety check - test with empty string
                let _ = regex.is_match("");
                Ok(regex)
            }
            Err(e) => Err(anyhow!("Invalid regex pattern: {}", e)),
        }
    }

    /// Validate pattern length for non-regex patterns
    fn validate_pattern_length(pattern: &str) -> Result<()> {
        if pattern.len() > 1024 {
            Err(anyhow!("Pattern too long (max 1024 characters)"))
        } else {
            Ok(())
        }
    }
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for value-based filtering
pub struct ValueFilter;

impl ValueFilter {
    /// Filter values based on comparison criteria
    pub fn compare_values(left: &Value, right: &Value, comparison: Comparison) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Self::compare_integers(*a, *b, comparison),
            (Value::Number(a), Value::Number(b)) => Self::compare_numbers(*a, *b, comparison),
            (Value::Integer(a), Value::Number(b)) => Self::compare_numbers(*a as f64, *b, comparison),
            (Value::Number(a), Value::Integer(b)) => Self::compare_numbers(*a, *b as f64, comparison),
            (Value::String(a), Value::String(b)) => Self::compare_strings(a, b, comparison),
            (Value::Path(a), Value::String(b)) => {
                Self::compare_strings(&a.to_string_lossy(), b, comparison)
            }
            (Value::String(a), Value::Path(b)) => {
                Self::compare_strings(a, &b.to_string_lossy(), comparison)
            }
            (Value::Severity(a), Value::Severity(b)) => {
                FilterEngine::compare_severity(*a, *b, comparison)
            }
            _ => false, // Incompatible types
        }
    }

    fn compare_integers(a: i64, b: i64, comparison: Comparison) -> bool {
        match comparison {
            Comparison::Equal => a == b,
            Comparison::NotEqual => a != b,
            Comparison::GreaterThan => a > b,
            Comparison::LessThan => a < b,
            Comparison::GreaterThanOrEqual => a >= b,
            Comparison::LessThanOrEqual => a <= b,
        }
    }

    fn compare_numbers(a: f64, b: f64, comparison: Comparison) -> bool {
        match comparison {
            Comparison::Equal => (a - b).abs() < f64::EPSILON,
            Comparison::NotEqual => (a - b).abs() >= f64::EPSILON,
            Comparison::GreaterThan => a > b,
            Comparison::LessThan => a < b,
            Comparison::GreaterThanOrEqual => a >= b,
            Comparison::LessThanOrEqual => a <= b,
        }
    }

    fn compare_strings(a: &str, b: &str, comparison: Comparison) -> bool {
        match comparison {
            Comparison::Equal => a == b,
            Comparison::NotEqual => a != b,
            Comparison::GreaterThan => a > b,
            Comparison::LessThan => a < b,
            Comparison::GreaterThanOrEqual => a >= b,
            Comparison::LessThanOrEqual => a <= b,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Position, Range};

    fn create_test_diagnostic(severity: DiagnosticSeverity, message: &str, code: Option<String>) -> Diagnostic {
        Diagnostic {
            id: "1".to_string(),
            file: "test.rs".to_string(),
            range: Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 10 },
            },
            severity,
            message: message.to_string(),
            source: "rust".to_string(),
            code,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    #[test]
    fn test_severity_filtering() {
        let engine = FilterEngine::new();
        let diagnostics = vec![
            (PathBuf::from("test1.rs"), create_test_diagnostic(DiagnosticSeverity::Error, "Error msg", None)),
            (PathBuf::from("test2.rs"), create_test_diagnostic(DiagnosticSeverity::Warning, "Warning msg", None)),
            (PathBuf::from("test3.rs"), create_test_diagnostic(DiagnosticSeverity::Information, "Info msg", None)),
        ];

        let filter = SeverityFilter {
            severity: DiagnosticSeverity::Error,
            comparison: Comparison::Equal,
        };

        let result = engine.filter_diagnostics_by_severity(diagnostics, &filter).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn test_path_filtering() {
        let engine = FilterEngine::new();
        let diagnostics = vec![
            (PathBuf::from("src/main.rs"), create_test_diagnostic(DiagnosticSeverity::Error, "Error", None)),
            (PathBuf::from("tests/test.rs"), create_test_diagnostic(DiagnosticSeverity::Warning, "Warning", None)),
            (PathBuf::from("src/lib.rs"), create_test_diagnostic(DiagnosticSeverity::Information, "Info", None)),
        ];

        let filter = PathFilter {
            pattern: "src/".to_string(),
            is_regex: false,
        };

        let result = engine.filter_diagnostics_by_path(diagnostics, &filter).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].0.to_str().unwrap().contains("src/"));
        assert!(result[1].0.to_str().unwrap().contains("src/"));
    }

    #[test]
    fn test_message_filtering() {
        let engine = FilterEngine::new();
        let diagnostics = vec![
            (PathBuf::from("test1.rs"), create_test_diagnostic(DiagnosticSeverity::Error, "Type error in function", None)),
            (PathBuf::from("test2.rs"), create_test_diagnostic(DiagnosticSeverity::Warning, "Unused variable", None)),
            (PathBuf::from("test3.rs"), create_test_diagnostic(DiagnosticSeverity::Error, "Parse error", None)),
        ];

        let filter = MessageFilter {
            pattern: "error".to_string(),
            is_regex: false,
        };

        let result = engine.filter_diagnostics_by_message(diagnostics, &filter).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].1.message.contains("error"));
        assert!(result[1].1.message.contains("error"));
    }

    #[test]
    fn test_regex_validation() {
        // Valid regex should work
        assert!(FilterEngine::validate_and_compile_regex(r"src/.*\.rs$").is_ok());

        // Invalid regex should fail
        assert!(FilterEngine::validate_and_compile_regex(r"[unclosed").is_err());

        // Dangerous patterns should be rejected
        assert!(FilterEngine::validate_and_compile_regex(r"(?R)").is_err());
        assert!(FilterEngine::validate_and_compile_regex(r"(*SKIP)").is_err());

        // Too long patterns should be rejected
        let long_pattern = "a".repeat(2000);
        assert!(FilterEngine::validate_and_compile_regex(&long_pattern).is_err());

        // Excessive repetition should be rejected
        assert!(FilterEngine::validate_and_compile_regex(r".*+").is_err());
    }

    #[test]
    fn test_value_comparison() {
        assert!(ValueFilter::compare_values(
            &Value::Integer(10),
            &Value::Integer(5),
            Comparison::GreaterThan
        ));

        assert!(ValueFilter::compare_values(
            &Value::String("apple".to_string()),
            &Value::String("banana".to_string()),
            Comparison::LessThan
        ));

        assert!(ValueFilter::compare_values(
            &Value::Number(3.14),
            &Value::Integer(3),
            Comparison::GreaterThan
        ));
    }
}