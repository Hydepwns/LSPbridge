pub mod analyzers;
pub mod context;
pub mod fixes;

use crate::analyzers::base::AnalyzerBase;
use crate::analyzers::error_codes::RustErrorCode;
use crate::analyzers::language_analyzer::{
    ContextRequirements, DiagnosticAnalysis, LanguageAnalyzer, FixSuggestion,
};
use crate::core::{Diagnostic, SemanticContext};

use analyzers::{
    BorrowCheckerAnalyzer, LifetimeAnalyzer, MoveSemanticsAnalyzer, TypeSystemAnalyzer,
};
use context::ContextAnalyzer;
use fixes::FixSuggestionGenerator;

pub struct RustAnalyzer {
    borrow_checker: BorrowCheckerAnalyzer,
    lifetime_analyzer: LifetimeAnalyzer,
    move_semantics: MoveSemanticsAnalyzer,
    type_system: TypeSystemAnalyzer,
    context_analyzer: ContextAnalyzer,
    fix_generator: FixSuggestionGenerator,
}

impl AnalyzerBase for RustAnalyzer {}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            borrow_checker: BorrowCheckerAnalyzer::new(),
            lifetime_analyzer: LifetimeAnalyzer::new(),
            move_semantics: MoveSemanticsAnalyzer::new(),
            type_system: TypeSystemAnalyzer::new(),
            context_analyzer: ContextAnalyzer::new(),
            fix_generator: FixSuggestionGenerator::new(),
        }
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn analyze_diagnostic(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        // Try to parse Rust error code
        if let Some(code_str) = &diagnostic.code {
            if let Some(rust_code) = RustErrorCode::from_str(code_str) {
                return if rust_code.is_borrow_error() {
                    self.borrow_checker.analyze_borrow_error(diagnostic, context)
                } else if rust_code.is_lifetime_error() {
                    self.lifetime_analyzer.analyze_lifetime_error(diagnostic, context)
                } else if rust_code.is_move_error() {
                    self.move_semantics.analyze_move_error(diagnostic, context)
                } else if rust_code == RustErrorCode::MismatchedTypes {
                    self.type_system.analyze_type_error(diagnostic, context)
                } else if rust_code == RustErrorCode::TraitBoundNotSatisfied {
                    self.type_system.analyze_trait_error(diagnostic, context)
                } else {
                    // Unknown Rust error code, fall through to message-based analysis
                    DiagnosticAnalysis::default()
                };
            }
        }

        // Fallback to message-based analysis
        if diagnostic.message.contains("borrow") {
            self.borrow_checker.analyze_borrow_error(diagnostic, context)
        } else if diagnostic.message.contains("lifetime")
            || diagnostic.message.contains("does not live long enough")
        {
            self.lifetime_analyzer.analyze_lifetime_error(diagnostic, context)
        } else if diagnostic.message.contains("move") || diagnostic.message.contains("moved") {
            self.move_semantics.analyze_move_error(diagnostic, context)
        } else if diagnostic.message.contains("expected") && diagnostic.message.contains("found") {
            self.type_system.analyze_type_error(diagnostic, context)
        } else if diagnostic.message.contains("trait")
            && diagnostic.message.contains("not implemented")
        {
            self.type_system.analyze_trait_error(diagnostic, context)
        } else {
            DiagnosticAnalysis::default()
        }
    }

    fn suggest_fix(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> Vec<FixSuggestion> {
        let analysis = self.analyze_diagnostic(diagnostic, context);
        self.fix_generator.suggest_fixes(diagnostic, context, analysis.category)
    }

    fn extract_context_requirements(&self, diagnostic: &Diagnostic) -> ContextRequirements {
        self.context_analyzer.extract_context_requirements(diagnostic)
    }

    fn language(&self) -> &str {
        "rust"
    }
}