use super::types::{Diagnostic, DiagnosticSeverity};
use std::collections::{HashMap, HashSet};

/// Groups related diagnostics together
#[derive(Debug, Clone)]
pub struct DiagnosticGroup {
    /// The primary/root diagnostic
    pub primary: Diagnostic,
    /// Related diagnostics that are likely caused by the primary issue
    pub related: Vec<Diagnostic>,
    /// Confidence score for the grouping (0.0-1.0)
    pub confidence: f32,
}

/// Service for deduplicating and grouping related diagnostics
pub struct DiagnosticGrouper {
    /// Patterns for identifying related errors
    patterns: Vec<GroupingPattern>,
}

#[derive(Debug)]
struct GroupingPattern {
    /// Name of the pattern
    name: String,
    /// Function to check if two diagnostics are related
    matcher: fn(&Diagnostic, &Diagnostic) -> bool,
    /// Confidence score if matched
    confidence: f32,
}

impl DiagnosticGrouper {
    pub fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
        }
    }

    /// Create default grouping patterns
    fn default_patterns() -> Vec<GroupingPattern> {
        vec![
            GroupingPattern {
                name: "same_symbol".to_string(),
                matcher: |a, b| {
                    // Group errors about the same symbol/identifier
                    if a.file != b.file {
                        return false;
                    }

                    // Extract symbol names from messages
                    let a_symbols = Self::extract_symbols(&a.message);
                    let b_symbols = Self::extract_symbols(&b.message);

                    // Check for common symbols
                    !a_symbols.is_disjoint(&b_symbols)
                },
                confidence: 0.8,
            },
            GroupingPattern {
                name: "cascading_type_errors".to_string(),
                matcher: |a, b| {
                    // Type errors often cascade
                    a.file == b.file
                        && a.severity == DiagnosticSeverity::Error
                        && b.severity == DiagnosticSeverity::Error
                        && (a.message.contains("type") || a.message.contains("Type"))
                        && (b.message.contains("type") || b.message.contains("Type"))
                },
                confidence: 0.7,
            },
            GroupingPattern {
                name: "import_errors".to_string(),
                matcher: |a, b| {
                    // Import/module errors are often related
                    a.file == b.file
                        && ((a.message.contains("import") && b.message.contains("import"))
                            || (a.message.contains("module") && b.message.contains("module"))
                            || (a.message.contains("Cannot find")
                                && b.message.contains("Cannot find")
                                && !DiagnosticGrouper::extract_symbols(&a.message)
                                    .is_disjoint(&DiagnosticGrouper::extract_symbols(&b.message))))
                },
                confidence: 0.9,
            },
            GroupingPattern {
                name: "undefined_variable_errors".to_string(),
                matcher: |a, b| {
                    // Undefined/undeclared variable errors for the same symbol should be grouped
                    a.file == b.file
                        && ((a.message.contains("undefined")
                            || a.message.contains("undeclared")
                            || a.message.contains("Cannot find value"))
                            && (b.message.contains("undefined")
                                || b.message.contains("undeclared")
                                || b.message.contains("Cannot find value"))
                            && !DiagnosticGrouper::extract_symbols(&a.message)
                                .is_disjoint(&DiagnosticGrouper::extract_symbols(&b.message)))
                },
                confidence: 0.95,
            },
            GroupingPattern {
                name: "same_line_range".to_string(),
                matcher: |a, b| {
                    // Errors on the same line are often related
                    a.file == b.file
                        && a.range.start.line == b.range.start.line
                        && (a.range.start.character as i32 - b.range.start.character as i32).abs()
                            < 10
                },
                confidence: 0.6,
            },
            GroupingPattern {
                name: "initialization_errors".to_string(),
                matcher: |a, b| {
                    // Uninitialized variable errors cascade
                    a.file == b.file
                        && ((a.message.contains("initialized") && b.message.contains("assigned"))
                            || (a.message.contains("assigned") && b.message.contains("used"))
                            || (a.message.contains("initializer") && b.message.contains("before")))
                },
                confidence: 0.85,
            },
            GroupingPattern {
                name: "borrow_checker_cascade".to_string(),
                matcher: |a, b| {
                    // Rust borrow checker errors often cascade
                    a.file == b.file
                        && a.source.contains("rust")
                        && ((a.message.contains("borrow") && b.message.contains("borrow"))
                            || (a.message.contains("moved") && b.message.contains("moved"))
                            || (a.message.contains("lifetime") && b.message.contains("lifetime")))
                },
                confidence: 0.75,
            },
        ]
    }

    /// Extract potential symbol names from a diagnostic message
    fn extract_symbols(message: &str) -> HashSet<String> {
        let mut symbols = HashSet::with_capacity(4); // Most messages have 1-4 symbols

        // Extract quoted identifiers
        let re = regex::Regex::new(r"'([a-zA-Z_][a-zA-Z0-9_]*)'").unwrap();
        for cap in re.captures_iter(message) {
            if let Some(symbol) = cap.get(1) {
                symbols.insert(symbol.as_str().to_string());
            }
        }

        // Extract backtick identifiers
        let re = regex::Regex::new(r"`([a-zA-Z_][a-zA-Z0-9_]*)`").unwrap();
        for cap in re.captures_iter(message) {
            if let Some(symbol) = cap.get(1) {
                symbols.insert(symbol.as_str().to_string());
            }
        }

        symbols
    }

    /// Group related diagnostics together
    pub fn group_diagnostics(&self, diagnostics: Vec<Diagnostic>) -> Vec<DiagnosticGroup> {
        if diagnostics.is_empty() {
            return Vec::new();
        }

        let mut groups: Vec<DiagnosticGroup> = Vec::with_capacity(diagnostics.len());
        let mut processed: HashSet<String> = HashSet::with_capacity(diagnostics.len());

        // Sort diagnostics by file and line for better grouping
        let mut sorted_diagnostics = diagnostics;
        sorted_diagnostics.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.range.start.line.cmp(&b.range.start.line))
                .then(a.severity.cmp(&b.severity))
        });

        for (i, diagnostic) in sorted_diagnostics.iter().enumerate() {
            if processed.contains(&diagnostic.id) {
                continue;
            }

            let mut group = DiagnosticGroup {
                primary: diagnostic.clone(),
                related: Vec::with_capacity(5), // Typical cascade is 2-5 related errors
                confidence: 1.0,
            };

            // Find related diagnostics
            for (j, other) in sorted_diagnostics.iter().enumerate() {
                if i == j || processed.contains(&other.id) {
                    continue;
                }

                // Check all patterns
                for pattern in &self.patterns {
                    if (pattern.matcher)(diagnostic, other) {
                        group.related.push(other.clone());
                        processed.insert(other.id.clone());
                        group.confidence = group.confidence.min(pattern.confidence);
                        break; // Only match once per diagnostic
                    }
                }
            }

            processed.insert(diagnostic.id.clone());
            groups.push(group);
        }

        // Sort groups by importance (errors first, then by line number)
        groups.sort_by(|a, b| {
            a.primary
                .severity
                .cmp(&b.primary.severity)
                .then(a.primary.file.cmp(&b.primary.file))
                .then(a.primary.range.start.line.cmp(&b.primary.range.start.line))
        });

        groups
    }

    /// Deduplicate diagnostics by removing exact duplicates
    pub fn deduplicate_diagnostics(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        let mut seen = HashMap::with_capacity(diagnostics.len());
        let mut deduplicated = Vec::with_capacity(diagnostics.len());

        for diagnostic in diagnostics {
            // Create a key based on file, range, and message
            let key = format!(
                "{}:{}:{}-{}:{}",
                diagnostic.file,
                diagnostic.range.start.line,
                diagnostic.range.start.character,
                diagnostic.severity as u8,
                // Take first 100 chars of message for comparison
                &diagnostic.message.chars().take(100).collect::<String>()
            );

            if !seen.contains_key(&key) {
                seen.insert(key, true);
                deduplicated.push(diagnostic);
            }
        }

        deduplicated
    }

    /// Get a summary of grouped diagnostics
    pub fn summarize_groups(&self, groups: &[DiagnosticGroup]) -> GroupingSummary {
        let total_diagnostics = groups.iter().map(|g| 1 + g.related.len()).sum();

        let primary_errors = groups
            .iter()
            .filter(|g| g.primary.severity == DiagnosticSeverity::Error)
            .count();

        let cascading_errors = groups
            .iter()
            .filter(|g| !g.related.is_empty())
            .map(|g| g.related.len())
            .sum();

        GroupingSummary {
            total_groups: groups.len(),
            total_diagnostics,
            primary_errors,
            cascading_errors,
            average_group_size: if groups.is_empty() {
                0.0
            } else {
                total_diagnostics as f32 / groups.len() as f32
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupingSummary {
    pub total_groups: usize,
    pub total_diagnostics: usize,
    pub primary_errors: usize,
    pub cascading_errors: usize,
    pub average_group_size: f32,
}

impl Default for DiagnosticGrouper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Position, Range};
    use uuid::Uuid;

    fn create_test_diagnostic(
        file: &str,
        line: u32,
        message: &str,
        severity: DiagnosticSeverity,
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

    #[test]
    fn test_deduplicate_diagnostics() {
        let grouper = DiagnosticGrouper::new();

        let diagnostics = vec![
            create_test_diagnostic(
                "test.rs",
                10,
                "Cannot find value 'x'",
                DiagnosticSeverity::Error,
            ),
            create_test_diagnostic(
                "test.rs",
                10,
                "Cannot find value 'x'",
                DiagnosticSeverity::Error,
            ),
            create_test_diagnostic("test.rs", 20, "Type mismatch", DiagnosticSeverity::Error),
        ];

        let deduplicated = grouper.deduplicate_diagnostics(diagnostics);
        assert_eq!(deduplicated.len(), 2);
    }

    #[test]
    fn test_group_related_type_errors() {
        let grouper = DiagnosticGrouper::new();

        let diagnostics = vec![
            create_test_diagnostic(
                "test.ts",
                10,
                "Type 'string' is not assignable to type 'number'",
                DiagnosticSeverity::Error,
            ),
            create_test_diagnostic(
                "test.ts",
                11,
                "Argument of type 'string' is not assignable to parameter of type 'number'",
                DiagnosticSeverity::Error,
            ),
            create_test_diagnostic(
                "test.ts",
                20,
                "Cannot find name 'unknownVar'",
                DiagnosticSeverity::Error,
            ),
        ];

        let groups = grouper.group_diagnostics(diagnostics);

        // Should have 2 groups: one for type errors, one for unknown variable
        assert_eq!(groups.len(), 2);

        // First group should have related type error
        let type_error_group = groups
            .iter()
            .find(|g| g.primary.message.contains("Type 'string'"))
            .unwrap();
        assert_eq!(type_error_group.related.len(), 1);
    }

    #[test]
    fn test_group_same_symbol_errors() {
        let grouper = DiagnosticGrouper::new();

        let diagnostics = vec![
            create_test_diagnostic(
                "test.rs",
                10,
                "Cannot find value 'undefined_var' in this scope",
                DiagnosticSeverity::Error,
            ),
            create_test_diagnostic(
                "test.rs",
                15,
                "Use of undeclared variable 'undefined_var'",
                DiagnosticSeverity::Error,
            ),
            create_test_diagnostic(
                "test.rs",
                20,
                "Cannot find value 'other_var' in this scope",
                DiagnosticSeverity::Error,
            ),
        ];

        let groups = grouper.group_diagnostics(diagnostics);

        // Should group the two 'undefined_var' errors together
        let undefined_var_group = groups
            .iter()
            .find(|g| g.primary.message.contains("undefined_var"))
            .unwrap();
        assert_eq!(undefined_var_group.related.len(), 1);
    }

    #[test]
    fn test_summarize_groups() {
        let grouper = DiagnosticGrouper::new();

        let groups = vec![
            DiagnosticGroup {
                primary: create_test_diagnostic(
                    "test.rs",
                    10,
                    "Error 1",
                    DiagnosticSeverity::Error,
                ),
                related: vec![
                    create_test_diagnostic("test.rs", 11, "Related 1", DiagnosticSeverity::Error),
                    create_test_diagnostic("test.rs", 12, "Related 2", DiagnosticSeverity::Error),
                ],
                confidence: 0.8,
            },
            DiagnosticGroup {
                primary: create_test_diagnostic(
                    "test.rs",
                    20,
                    "Error 2",
                    DiagnosticSeverity::Warning,
                ),
                related: vec![],
                confidence: 1.0,
            },
        ];

        let summary = grouper.summarize_groups(&groups);
        assert_eq!(summary.total_groups, 2);
        assert_eq!(summary.total_diagnostics, 4); // 2 primary + 2 related
        assert_eq!(summary.primary_errors, 1);
        assert_eq!(summary.cascading_errors, 2);
        assert_eq!(summary.average_group_size, 2.0);
    }
}
