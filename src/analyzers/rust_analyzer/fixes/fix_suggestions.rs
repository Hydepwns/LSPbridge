use crate::analyzers::language_analyzer::{DiagnosticCategory, FixSuggestion};
use crate::core::{Diagnostic, SemanticContext};

pub struct FixSuggestionGenerator;

impl Default for FixSuggestionGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl FixSuggestionGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn suggest_fixes(
        &self,
        diagnostic: &Diagnostic,
        _context: Option<&SemanticContext>,
        analysis_category: DiagnosticCategory,
    ) -> Vec<FixSuggestion> {
        let mut suggestions = Vec::with_capacity(3); // Most diagnostics have 1-3 fix suggestions

        match analysis_category {
            DiagnosticCategory::BorrowChecker => {
                self.suggest_borrow_checker_fixes(diagnostic, &mut suggestions);
            }

            DiagnosticCategory::MoveError => {
                self.suggest_move_error_fixes(diagnostic, &mut suggestions);
            }

            DiagnosticCategory::TypeMismatch => {
                self.suggest_type_mismatch_fixes(diagnostic, &mut suggestions);
            }

            DiagnosticCategory::LifetimeError => {
                self.suggest_lifetime_fixes(diagnostic, &mut suggestions);
            }

            _ => {}
        }

        suggestions
    }

    fn suggest_borrow_checker_fixes(
        &self,
        diagnostic: &Diagnostic,
        suggestions: &mut Vec<FixSuggestion>,
    ) {
        if diagnostic.message.contains("mutable more than once") {
            suggestions.push(FixSuggestion {
                description: "Use RefCell for interior mutability".to_string(),
                code_snippet: Some(
                    "use std::cell::RefCell;\nlet x = RefCell::new(value);".to_string(),
                ),
                confidence: 0.7,
                is_automatic: false,
                prerequisites: vec!["Single-threaded context".to_string()],
            });

            suggestions.push(FixSuggestion {
                description: "Clone the data if it's small".to_string(),
                code_snippet: Some("let cloned = original.clone();".to_string()),
                confidence: 0.6,
                is_automatic: false,
                prerequisites: vec!["Type implements Clone".to_string()],
            });
        }
    }

    fn suggest_move_error_fixes(
        &self,
        diagnostic: &Diagnostic,
        suggestions: &mut Vec<FixSuggestion>,
    ) {
        if diagnostic.message.contains("use of moved value") {
            suggestions.push(FixSuggestion {
                description: "Clone before moving".to_string(),
                code_snippet: Some(".clone()".to_string()),
                confidence: 0.8,
                is_automatic: true,
                prerequisites: vec!["Type implements Clone".to_string()],
            });
        }

        if diagnostic.message.contains("cannot move out of index") {
            suggestions.push(FixSuggestion {
                description: "Use Vec::remove to take ownership".to_string(),
                code_snippet: Some("vec.remove(index)".to_string()),
                confidence: 0.7,
                is_automatic: false,
                prerequisites: vec!["Mutable access to Vec".to_string()],
            });
        }
    }

    fn suggest_type_mismatch_fixes(
        &self,
        diagnostic: &Diagnostic,
        suggestions: &mut Vec<FixSuggestion>,
    ) {
        // String conversions
        if diagnostic.message.contains("expected `&str`")
            && diagnostic.message.contains("found `String`")
        {
            suggestions.push(FixSuggestion {
                description: "Convert String to &str".to_string(),
                code_snippet: Some("&value".to_string()),
                confidence: 0.9,
                is_automatic: true,
                prerequisites: vec![],
            });
        } else if diagnostic.message.contains("expected `String`")
            && diagnostic.message.contains("found `&str`")
        {
            suggestions.push(FixSuggestion {
                description: "Convert &str to String".to_string(),
                code_snippet: Some(".to_string()".to_string()),
                confidence: 0.9,
                is_automatic: true,
                prerequisites: vec![],
            });
        }

        // Result/Option handling
        if diagnostic.message.contains("expected enum `Result`") {
            suggestions.push(FixSuggestion {
                description: "Wrap in Ok()".to_string(),
                code_snippet: Some("Ok(value)".to_string()),
                confidence: 0.8,
                is_automatic: true,
                prerequisites: vec![],
            });
        }
    }

    fn suggest_lifetime_fixes(
        &self,
        _diagnostic: &Diagnostic,
        suggestions: &mut Vec<FixSuggestion>,
    ) {
        suggestions.push(FixSuggestion {
            description: "Add explicit lifetime annotations".to_string(),
            code_snippet: Some("fn example<'a>(x: &'a str) -> &'a str { x }".to_string()),
            confidence: 0.5,
            is_automatic: false,
            prerequisites: vec!["Understanding of lifetime rules".to_string()],
        });
    }
}