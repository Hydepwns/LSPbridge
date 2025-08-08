use super::super::analyzers::TypeInferenceHelper;
use crate::analyzers::language_analyzer::{DiagnosticCategory, FixSuggestion};
use crate::core::{Diagnostic, SemanticContext};

pub struct TypeScriptFixSuggestionGenerator {
    type_inference: TypeInferenceHelper,
}

impl Default for TypeScriptFixSuggestionGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptFixSuggestionGenerator {
    pub fn new() -> Self {
        Self {
            type_inference: TypeInferenceHelper::new(),
        }
    }

    pub fn suggest_fixes(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
        analysis_category: DiagnosticCategory,
        analysis_insights: &[String],
        related_symbols: &[String],
    ) -> Vec<FixSuggestion> {
        let mut suggestions = Vec::new();

        match analysis_category {
            DiagnosticCategory::MissingProperty => {
                self.suggest_property_fixes(diagnostic, context, analysis_insights, related_symbols, &mut suggestions);
            }

            DiagnosticCategory::TypeMismatch => {
                self.suggest_type_mismatch_fixes(diagnostic, &mut suggestions);
            }

            DiagnosticCategory::MissingImport => {
                self.suggest_import_fixes(related_symbols, &mut suggestions);
            }
            
            DiagnosticCategory::GenericTypeError => {
                self.suggest_generic_fixes(diagnostic, analysis_insights, &mut suggestions);
            }

            _ => {}
        }

        suggestions
    }

    fn suggest_property_fixes(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
        insights: &[String],
        related_symbols: &[String],
        suggestions: &mut Vec<FixSuggestion>,
    ) {
        if let Some(insight) = insights.first() {
            if insight.starts_with("Did you mean") {
                suggestions.push(FixSuggestion {
                    description: insight.clone(),
                    code_snippet: None, // Would need actual code to generate
                    confidence: 0.8,
                    is_automatic: true,
                    prerequisites: vec![],
                });
            }
        }

        // Suggest adding the property with proper type inference
        if let Some(prop) = related_symbols.first() {
            // Try to infer the type from the diagnostic message
            let inferred_type = self.type_inference.infer_property_type(diagnostic, prop, context);
            
            suggestions.push(FixSuggestion {
                description: format!("Add property '{prop}' to the type"),
                code_snippet: Some(format!("{prop}: {inferred_type};")),
                confidence: if inferred_type != "unknown" { 0.8 } else { 0.6 },
                is_automatic: false,
                prerequisites: vec!["Access to type definition".to_string()],
            });
        }
    }

    fn suggest_type_mismatch_fixes(&self, diagnostic: &Diagnostic, suggestions: &mut Vec<FixSuggestion>) {
        if diagnostic.message.contains("string") && diagnostic.message.contains("number") {
            suggestions.push(FixSuggestion {
                description: "Convert string to number".to_string(),
                code_snippet: Some("Number(value)".to_string()),
                confidence: 0.7,
                is_automatic: true,
                prerequisites: vec![],
            });

            suggestions.push(FixSuggestion {
                description: "Convert number to string".to_string(),
                code_snippet: Some("value.toString()".to_string()),
                confidence: 0.7,
                is_automatic: true,
                prerequisites: vec![],
            });
        }
    }

    fn suggest_import_fixes(&self, related_symbols: &[String], suggestions: &mut Vec<FixSuggestion>) {
        if let Some(symbol) = related_symbols.first() {
            // Common imports
            let common_imports = vec![
                ("useState", "import { useState } from 'react';"),
                ("useEffect", "import { useEffect } from 'react';"),
                ("FC", "import { FC } from 'react';"),
                ("ReactNode", "import { ReactNode } from 'react';"),
            ];

            for (name, import) in common_imports {
                if symbol == name {
                    suggestions.push(FixSuggestion {
                        description: format!("Add {name} import"),
                        code_snippet: Some(import.to_string()),
                        confidence: 0.9,
                        is_automatic: true,
                        prerequisites: vec![],
                    });
                }
            }
        }
    }

    fn suggest_generic_fixes(
        &self, 
        diagnostic: &Diagnostic, 
        insights: &[String], 
        suggestions: &mut Vec<FixSuggestion>
    ) {
        // Extract generic type information from insights
        if let Some(insight) = insights.iter().find(|i| i.contains("type arguments")) {
            // Try to extract the number of required arguments
            if let Some(num_str) = insight.split_whitespace().find(|s| s.parse::<usize>().is_ok()) {
                if let Ok(num) = num_str.parse::<usize>() {
                    // Generate appropriate type parameters based on context
                    let type_params = match num {
                        1 => "<T>",
                        2 => "<T, U>",
                        3 => "<T, U, V>",
                        _ => "<T, ...>",
                    };
                    
                    suggestions.push(FixSuggestion {
                        description: format!("Add {num} type argument(s)"),
                        code_snippet: Some(type_params.to_string()),
                        confidence: 0.7,
                        is_automatic: false,
                        prerequisites: vec!["Knowledge of expected types".to_string()],
                    });
                    
                    // Suggest common patterns
                    if diagnostic.message.contains("Array") {
                        suggestions.push(FixSuggestion {
                            description: "Specify array element type".to_string(),
                            code_snippet: Some("<string>".to_string()),
                            confidence: 0.6,
                            is_automatic: false,
                            prerequisites: vec![],
                        });
                    } else if diagnostic.message.contains("Promise") {
                        suggestions.push(FixSuggestion {
                            description: "Specify promise resolution type".to_string(),
                            code_snippet: Some("<void>".to_string()),
                            confidence: 0.6,
                            is_automatic: false,
                            prerequisites: vec![],
                        });
                    }
                }
            }
        }
    }
}