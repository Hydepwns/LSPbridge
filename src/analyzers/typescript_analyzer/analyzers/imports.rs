use crate::analyzers::base::DiagnosticPatterns;
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::{Diagnostic, SemanticContext};

pub struct ImportAnalyzer;

impl Default for ImportAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_import_error(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let identifiers = DiagnosticPatterns::extract_quoted_identifiers(&diagnostic.message);

        let mut analysis = DiagnosticAnalysis {
            category: DiagnosticCategory::MissingImport,
            likely_cause: "Module or symbol not found in imports".to_string(),
            confidence: 0.9,
            related_symbols: identifiers.clone(),
            is_cascading: true, // Import errors often cascade
            fix_complexity: 1,
            insights: Vec::new(),
        };

        // Check if it's a missing type import
        if diagnostic.message.contains("Cannot find name") {
            if let Some(symbol) = identifiers.first() {
                // Common React types
                if ["JSX", "FC", "ReactNode", "ReactElement"].contains(&symbol.as_str()) {
                    analysis.insights.push(
                        "Missing React type import - add: import { FC } from 'react'".to_string(),
                    );
                }
                // Node types
                else if symbol.starts_with("Node") || symbol == "Buffer" {
                    analysis
                        .insights
                        .push("Missing Node.js types - install @types/node".to_string());
                }
                // Check if it's already imported but not in scope
                else if let Some(ctx) = context {
                    let imported = ctx
                        .imports
                        .iter()
                        .any(|imp| imp.imported_names.contains(symbol));
                    if imported {
                        analysis.category = DiagnosticCategory::Unknown;
                        analysis.likely_cause = "Symbol is imported but not in scope".to_string();
                    }
                }
            }
        }

        // Module resolution errors
        if diagnostic.message.contains("Cannot find module") {
            analysis
                .insights
                .push("Check if the module is installed (npm install)".to_string());
            analysis
                .insights
                .push("Verify the import path is correct".to_string());

            if diagnostic.message.contains("'.css'") || diagnostic.message.contains("'.scss'") {
                analysis
                    .insights
                    .push("CSS imports may need a type declaration file".to_string());
            }
        }

        analysis
    }
}