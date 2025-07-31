use crate::analyzers::base::DiagnosticPatterns;
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::{Diagnostic, SemanticContext};

pub struct TypeSystemAnalyzer;

impl TypeSystemAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_type_mismatch(
        &self,
        diagnostic: &Diagnostic,
        _context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let types = DiagnosticPatterns::extract_types(&diagnostic.message);

        let mut analysis = DiagnosticAnalysis {
            category: DiagnosticCategory::TypeMismatch,
            likely_cause: "Type incompatibility between expected and actual types".to_string(),
            confidence: 0.85,
            related_symbols: types.clone(),
            is_cascading: false,
            fix_complexity: 2,
            insights: Vec::new(),
        };

        // Analyze specific type mismatches
        if types.len() >= 2 {
            let expected = &types[0];
            let actual = &types[1];

            // String vs Number
            if (expected == "number" && actual == "string")
                || (expected == "string" && actual == "number")
            {
                analysis
                    .insights
                    .push("Consider using type conversion (Number() or String())".to_string());
                analysis.fix_complexity = 1;
            }
            // Array type mismatches
            else if expected.contains("[]") || actual.contains("[]") {
                analysis
                    .insights
                    .push("Array type mismatch - check element types".to_string());
            }
            // Promise-related
            else if expected.contains("Promise") || actual.contains("Promise") {
                analysis
                    .insights
                    .push("Async/await mismatch - ensure proper await usage".to_string());
                analysis.category = DiagnosticCategory::AsyncError;
            }
            // Union type issues
            else if expected.contains("|") || actual.contains("|") {
                analysis
                    .insights
                    .push("Union type mismatch - consider type narrowing".to_string());
            }
        }

        // Check for null/undefined issues
        if diagnostic.message.contains("null") || diagnostic.message.contains("undefined") {
            analysis.insights.push(
                "Null/undefined handling needed - use optional chaining or type guards".to_string(),
            );
        }

        analysis
    }

    pub fn analyze_generic_error(
        &self,
        diagnostic: &Diagnostic,
        _context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let types = DiagnosticPatterns::extract_types(&diagnostic.message);

        let mut analysis = DiagnosticAnalysis {
            category: DiagnosticCategory::TypeMismatch,
            likely_cause: "Generic type constraint violation or inference failure".to_string(),
            confidence: 0.75,
            related_symbols: types,
            is_cascading: false,
            fix_complexity: 3,
            insights: Vec::new(),
        };

        if diagnostic.message.contains("constraint") {
            analysis
                .insights
                .push("Generic type doesn't satisfy constraint".to_string());
            analysis
                .insights
                .push("Check extends clause or provide explicit type".to_string());
        } else if diagnostic.message.contains("infer") {
            analysis
                .insights
                .push("TypeScript couldn't infer generic type".to_string());
            analysis
                .insights
                .push("Consider providing explicit type arguments".to_string());
        } else if diagnostic.message.contains("keyof") {
            analysis
                .insights
                .push("Key doesn't exist on the given type".to_string());
            analysis
                .insights
                .push("Use keyof operator or check available keys".to_string());
        }

        analysis
    }
}