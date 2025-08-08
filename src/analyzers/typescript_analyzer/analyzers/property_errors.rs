use super::type_inference::TypeInferenceHelper;
use crate::analyzers::base::{AnalyzerBase, ComplexityScorer};
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::{Diagnostic, SemanticContext};
use regex::Regex;

pub struct PropertyErrorAnalyzer {
    #[allow(dead_code)]
    type_inference: TypeInferenceHelper,
}

impl AnalyzerBase for PropertyErrorAnalyzer {}

impl Default for PropertyErrorAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyErrorAnalyzer {
    pub fn new() -> Self {
        Self {
            type_inference: TypeInferenceHelper::new(),
        }
    }

    pub fn analyze_property_error(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let identifiers = self.extract_identifiers(&diagnostic.message);

        let mut analysis = self.create_analysis(
            DiagnosticCategory::MissingProperty,
            0.9,
            2,
            "Accessing a property that doesn't exist on the type".to_string(),
            identifiers.clone(),
        );

        // Check if it might be a typo
        if let Some(ctx) = context {
            if let Some(class_ctx) = &ctx.class_context {
                // Look for similar property names
                let property_pattern = Regex::new(r"\b(\w+):\s*\w+").unwrap();
                let mut available_properties = Vec::new();

                for cap in property_pattern.captures_iter(&class_ctx.definition) {
                    if let Some(prop) = cap.get(1) {
                        available_properties.push(prop.as_str().to_string());
                    }
                }

                if let Some(missing_prop) = identifiers.first() {
                    if let Some(similar) =
                        ComplexityScorer::find_similar_name(missing_prop, &available_properties)
                    {
                        self.add_insight(&mut analysis, &format!("Did you mean '{similar}'?"));
                        analysis.fix_complexity = 1;
                    }
                }
            }
        }

        // Check if it's an optional chaining issue
        if diagnostic.message.contains("possibly 'undefined'") {
            analysis
                .insights
                .push("Consider using optional chaining (?.) or null checks".to_string());
            analysis.category = DiagnosticCategory::TypeMismatch;
        }

        analysis
    }
}