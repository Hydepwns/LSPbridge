pub mod analyzers;
pub mod context;
pub mod fixes;

use crate::analyzers::base::AnalyzerBase;
use crate::analyzers::error_codes::TypeScriptErrorCode;
use crate::analyzers::language_analyzer::{
    ContextRequirements, DiagnosticAnalysis, FixSuggestion, LanguageAnalyzer,
};
use crate::core::{Diagnostic, SemanticContext};

use analyzers::{ImportAnalyzer, PropertyErrorAnalyzer, TypeSystemAnalyzer};
use context::TypeScriptContextAnalyzer;
use fixes::TypeScriptFixSuggestionGenerator;

pub struct TypeScriptAnalyzer {
    property_analyzer: PropertyErrorAnalyzer,
    type_system: TypeSystemAnalyzer,
    import_analyzer: ImportAnalyzer,
    context_analyzer: TypeScriptContextAnalyzer,
    fix_generator: TypeScriptFixSuggestionGenerator,
}

impl AnalyzerBase for TypeScriptAnalyzer {}

impl TypeScriptAnalyzer {
    pub fn new() -> Self {
        Self {
            property_analyzer: PropertyErrorAnalyzer::new(),
            type_system: TypeSystemAnalyzer::new(),
            import_analyzer: ImportAnalyzer::new(),
            context_analyzer: TypeScriptContextAnalyzer::new(),
            fix_generator: TypeScriptFixSuggestionGenerator::new(),
        }
    }
}

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn analyze_diagnostic(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        // Try to parse TypeScript error code
        if let Some(code_str) = &diagnostic.code {
            if let Some(ts_code) = TypeScriptErrorCode::from_str(code_str) {
                return match ts_code {
                    TypeScriptErrorCode::PropertyDoesNotExist
                    | TypeScriptErrorCode::PropertyDoesNotExistWithSuggestion => {
                        self.property_analyzer.analyze_property_error(diagnostic, context)
                    }
                    TypeScriptErrorCode::TypeNotAssignable
                    | TypeScriptErrorCode::ArgumentTypeNotAssignable => {
                        self.type_system.analyze_type_mismatch(diagnostic, context)
                    }
                    TypeScriptErrorCode::CannotFindName
                    | TypeScriptErrorCode::CannotFindNameWithSuggestion => {
                        self.import_analyzer.analyze_import_error(diagnostic, context)
                    }
                    TypeScriptErrorCode::GenericTypeRequiresArguments => {
                        self.type_system.analyze_generic_error(diagnostic, context)
                    }
                };
            }
        }

        // Fallback to message-based analysis if no code or unrecognized code
        if diagnostic.message.contains("Property")
            && diagnostic.message.contains("does not exist")
        {
            self.property_analyzer.analyze_property_error(diagnostic, context)
        } else if diagnostic.message.contains("Type")
            && diagnostic.message.contains("is not assignable")
        {
            self.type_system.analyze_type_mismatch(diagnostic, context)
        } else if diagnostic.message.contains("Cannot find") {
            self.import_analyzer.analyze_import_error(diagnostic, context)
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
        self.fix_generator.suggest_fixes(
            diagnostic,
            context,
            analysis.category,
            &analysis.insights,
            &analysis.related_symbols,
        )
    }

    fn extract_context_requirements(&self, diagnostic: &Diagnostic) -> ContextRequirements {
        self.context_analyzer.extract_context_requirements(diagnostic)
    }

    fn language(&self) -> &str {
        "typescript"
    }
}