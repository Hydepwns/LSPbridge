use crate::analyzers::base::DiagnosticPatterns;
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::{Diagnostic, SemanticContext};

pub struct MoveSemanticsAnalyzer;

impl Default for MoveSemanticsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveSemanticsAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_move_error(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let identifiers = DiagnosticPatterns::extract_quoted_identifiers(&diagnostic.message);

        let mut analysis = DiagnosticAnalysis {
            category: DiagnosticCategory::MoveError,
            likely_cause: "Value moved when it should be borrowed or copied".to_string(),
            confidence: 0.9,
            related_symbols: identifiers.clone(),
            is_cascading: true,
            fix_complexity: 2,
            insights: Vec::with_capacity(4), // Move errors typically have 2-4 insights
        };

        if diagnostic.message.contains("cannot move out of") {
            if diagnostic.message.contains("borrowed content")
                || diagnostic.message.contains("behind a shared reference")
            {
                analysis
                    .insights
                    .push("Cannot move from behind a reference".to_string());
                analysis
                    .insights
                    .push("Consider cloning the value".to_string());
                analysis
                    .insights
                    .push("Or use pattern matching with ref".to_string());
            } else if diagnostic.message.contains("index") {
                analysis
                    .insights
                    .push("Cannot move out of indexed content".to_string());
                analysis
                    .insights
                    .push("Use remove() to take ownership from Vec".to_string());
                analysis.insights.push("Or clone the value".to_string());
            }
        } else if diagnostic.message.contains("use of moved value") {
            analysis
                .insights
                .push("Value was moved in previous operation".to_string());

            // Check if type implements Copy
            if let Some(_ctx) = context {
                // Simple heuristic - primitive types are usually Copy
                if identifiers.iter().any(|id| {
                    ["i32", "u32", "i64", "u64", "f32", "f64", "bool", "char"]
                        .contains(&id.as_str())
                }) {
                    analysis
                        .insights
                        .push("Consider implementing Copy trait for this type".to_string());
                    analysis.fix_complexity = 1;
                }
            }

            analysis
                .insights
                .push("Clone the value before moving".to_string());
            analysis
                .insights
                .push("Or restructure to avoid multiple uses".to_string());
        }

        analysis
    }
}