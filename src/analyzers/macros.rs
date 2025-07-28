/// Macros for reducing boilerplate in analyzer implementations
///
/// These macros help create consistent analyzer implementations
/// while reducing code duplication across language analyzers.

/// Creates a new analyzer struct with base implementation
///
/// Usage:
/// ```rust
/// create_analyzer!(RustAnalyzer);
/// ```
///
/// Generates:
/// - Struct definition
/// - AnalyzerBase trait implementation  
/// - Default constructor
#[macro_export]
macro_rules! create_analyzer {
    ($name:ident) => {
        pub struct $name;

        impl $crate::analyzers::base::AnalyzerBase for $name {}

        impl $name {
            pub fn new() -> Self {
                Self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Creates a standard diagnostic analysis helper method
///
/// Usage:
/// ```rust
/// create_analysis_method!(
///     analyze_syntax_error,
///     DiagnosticCategory::SyntaxError,
///     0.95,
///     1,
///     "Syntax error"
/// );
/// ```
#[macro_export]
macro_rules! create_analysis_method {
    ($method_name:ident, $category:expr, $confidence:expr, $complexity:expr, $description:expr) => {
        fn $method_name(
            &self,
            diagnostic: &Diagnostic,
            _context: Option<&SemanticContext>,
        ) -> DiagnosticAnalysis {
            let identifiers = self.extract_identifiers(&diagnostic.message);
            self.create_analysis(
                $category,
                $confidence,
                $complexity,
                $description,
                identifiers,
            )
        }
    };
}

/// Creates a common pattern matcher for diagnostic messages
///
/// Usage:
/// ```rust
/// match_diagnostic_pattern!(
///     diagnostic,
///     analysis,
///     [
///         ("cannot find", "Symbol not found", vec!["Check imports", "Verify symbol name"]),
///         ("type mismatch", "Type error", vec!["Check type annotations", "Verify assignments"])
///     ]
/// );
/// ```
#[macro_export]
macro_rules! match_diagnostic_pattern {
    ($diagnostic:expr, $analysis:expr, [$( ($pattern:expr, $cause:expr, $insights:expr) ),* $(,)?]) => {
        $(
            if $diagnostic.message.contains($pattern) {
                $analysis.likely_cause = $cause;
                for insight in $insights {
                    $analysis.insights.push(insight);
                }
            }
        )*
    };
}

/// Creates a severity-based router for diagnostic handling
///
/// Usage:
/// ```rust
/// route_by_severity!(
///     diagnostic,
///     Error => self.analyze_error(diagnostic, context),
///     Warning => self.analyze_warning(diagnostic, context),
///     _ => self.analyze_default(diagnostic, context)
/// );
/// ```
#[macro_export]
macro_rules! route_by_severity {
    ($diagnostic:expr, $($severity:pat => $handler:expr),* $(,)?) => {
        match $diagnostic.severity {
            $(
                $severity => $handler,
            )*
        }
    };
}

/// Creates standard test helpers for analyzers
///
/// Usage:
/// ```rust
/// create_analyzer_tests!(RustAnalyzer, "rust");
/// ```
#[macro_export]
macro_rules! create_analyzer_tests {
    ($analyzer:ident, $language:expr) => {
        #[cfg(test)]
        mod tests {
            use super::*;
            use $crate::core::types::{Diagnostic, DiagnosticSeverity, Position, Range};

            fn create_test_diagnostic(message: &str, severity: DiagnosticSeverity) -> Diagnostic {
                Diagnostic::new(
                    format!("test.{}", $language),
                    Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 10,
                        },
                    },
                    severity,
                    message.to_string(),
                    concat!($language, "-analyzer").to_string(),
                )
            }

            #[test]
            fn test_analyzer_creation() {
                let analyzer = $analyzer::new();
                // Basic creation test
                let _ = analyzer; // Ensure it compiles
            }

            #[test]
            fn test_default_implementation() {
                let analyzer = $analyzer::default();
                let analyzer2 = $analyzer::new();
                // Both should create valid instances
                let _ = (analyzer, analyzer2);
            }
        }
    };
}
