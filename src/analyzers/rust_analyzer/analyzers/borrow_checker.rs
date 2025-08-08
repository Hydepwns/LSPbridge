use crate::analyzers::base::AnalyzerBase;
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::constants::error_patterns;
use crate::core::{Diagnostic, SemanticContext};

pub struct BorrowCheckerAnalyzer;

impl AnalyzerBase for BorrowCheckerAnalyzer {}

impl Default for BorrowCheckerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl BorrowCheckerAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_borrow_error(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let identifiers = self.extract_identifiers(&diagnostic.message);

        let mut analysis = self.create_analysis(
            DiagnosticCategory::BorrowChecker,
            0.9,
            3,
            "Borrow checker violation".to_string(),
            identifiers.clone(),
        );
        self.mark_cascading(&mut analysis); // Borrow errors often cascade

        // Analyze specific borrow patterns
        if diagnostic
            .message
            .contains(error_patterns::CANNOT_BORROW_MUTABLE)
            && diagnostic.message.contains("as mutable more than once")
        {
            analysis.likely_cause = "Multiple mutable borrows of the same value".to_string();
            self.add_insight(
                &mut analysis,
                "Consider using RefCell for interior mutability",
            );
            self.add_insight(
                &mut analysis,
                "Or restructure code to avoid overlapping mutable borrows",
            );
            analysis.fix_complexity = 4;
        } else if diagnostic
            .message
            .contains(error_patterns::CANNOT_BORROW_MUTABLE)
            && diagnostic
                .message
                .contains("as mutable because it is also borrowed as immutable")
        {
            analysis.likely_cause = "Mutable borrow while immutable borrow exists".to_string();
            self.add_insight(
                &mut analysis,
                "Ensure immutable borrows go out of scope before mutable borrow",
            );
            self.add_insight(&mut analysis, "Consider cloning if the data is small");
        } else if diagnostic
            .message
            .contains(error_patterns::DOES_NOT_LIVE_LONG_ENOUGH)
        {
            analysis.category = DiagnosticCategory::LifetimeError;
            analysis.likely_cause = "Value dropped while still borrowed".to_string();
            self.add_insight(&mut analysis, "Extend the lifetime of the value");
            self.add_insight(&mut analysis, "Or reduce the lifetime of the borrow");
        }

        // Check if it's in a loop
        if let Some(ctx) = context {
            if let Some(func) = &ctx.function_context {
                if func.body.contains("for") || func.body.contains("while") {
                    self.add_insight(
                        &mut analysis,
                        "Borrow errors in loops often require collecting results first",
                    );
                }
            }
        }

        analysis
    }
}