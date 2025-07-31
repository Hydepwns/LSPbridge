use crate::core::context_ranking::types::PriorityConfig;
use crate::core::semantic_context::{
    CallHierarchy, ClassContext, DependencyInfo, FunctionContext, ImportContext, TypeDefinition,
    VariableContext,
};
use crate::core::types::Diagnostic;

pub struct RelevanceScorer<'a> {
    config: &'a PriorityConfig,
}

impl<'a> RelevanceScorer<'a> {
    pub fn new(config: &'a PriorityConfig) -> Self {
        Self { config }
    }

    pub fn score_function(
        &self,
        func_ctx: &FunctionContext,
        diagnostic: &Diagnostic,
    ) -> (f32, String) {
        let mut priority = self.config.function_context_weight;
        let mut explanation_parts = Vec::new();

        // Boost priority if function name appears in diagnostic message
        if diagnostic.message.contains(&func_ctx.name) {
            priority *= 1.5;
            explanation_parts.push(format!(
                "Function '{}' is mentioned in the error message",
                func_ctx.name
            ));
        }

        // Boost priority if diagnostic is within function bounds
        let diagnostic_line = diagnostic.range.start.line;
        if diagnostic_line >= func_ctx.start_line && diagnostic_line <= func_ctx.end_line {
            priority *= 1.3;
            if explanation_parts.is_empty() {
                explanation_parts.push(format!(
                    "Function '{}' contains the error location",
                    func_ctx.name
                ));
            }
        }

        let explanation = if explanation_parts.is_empty() {
            format!("Function '{}' provides context", func_ctx.name)
        } else {
            explanation_parts.join("; ")
        };

        (priority.min(1.0), explanation)
    }

    pub fn score_class(&self, class_ctx: &ClassContext, diagnostic: &Diagnostic) -> (f32, String) {
        let mut priority = self.config.class_context_weight;
        let mut explanation_parts = Vec::new();

        // Boost priority if class name appears in diagnostic message
        if diagnostic.message.contains(&class_ctx.name) {
            priority *= 1.4;
            explanation_parts.push(format!(
                "{} '{}' is mentioned in the error message",
                class_ctx.kind, class_ctx.name
            ));
        }

        // Boost priority for type-related errors
        if diagnostic.message.contains("type")
            || diagnostic.message.contains("interface")
            || diagnostic.message.contains("struct")
        {
            priority *= 1.2;
            if explanation_parts.is_empty() {
                explanation_parts.push("Type-related error detected".to_string());
            }
        }

        let explanation = if explanation_parts.is_empty() {
            format!(
                "{} '{}' contains the error location",
                class_ctx.kind, class_ctx.name
            )
        } else {
            explanation_parts.join("; ")
        };

        (priority.min(1.0), explanation)
    }

    pub fn score_import(
        &self,
        import_ctx: &ImportContext,
        diagnostic: &Diagnostic,
    ) -> (f32, String) {
        let mut priority = self.config.import_relevance_weight;
        let imported_names = import_ctx.imported_names.join(", ");

        // Check if any imported names appear in the diagnostic message
        let mentioned_imports: Vec<&String> = import_ctx
            .imported_names
            .iter()
            .filter(|name| diagnostic.message.contains(*name))
            .collect();

        if !mentioned_imports.is_empty() {
            priority *= 1.3;
            let mentioned = mentioned_imports
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return (
                priority.min(1.0),
                format!("Import {} directly mentioned in error: {}", mentioned, imported_names),
            );
        }

        // Check if source module is mentioned
        if let Some(source) = &import_ctx.source {
            if diagnostic.message.contains(source) {
                priority *= 1.2;
                return (
                    priority.min(1.0),
                    format!(
                        "Import source '{}' mentioned in error: {}",
                        source, imported_names
                    ),
                );
            }
        }

        let explanation = if let Some(source) = &import_ctx.source {
            format!(
                "Import {} from '{}' may provide context for error resolution",
                imported_names, source
            )
        } else {
            format!(
                "Import {} may provide context for error resolution",
                imported_names
            )
        };

        (priority.min(1.0), explanation)
    }

    pub fn score_type(
        &self,
        type_def: &TypeDefinition,
        diagnostic: &Diagnostic,
    ) -> (f32, String) {
        let mut priority = self.config.type_definition_weight;

        // Very high priority if type name appears in diagnostic
        if diagnostic.message.contains(&type_def.name) {
            priority *= 1.8;
            return (
                priority.min(1.0),
                format!("Type '{}' is directly mentioned in the error", type_def.name),
            );
        }

        // High priority for type errors
        if diagnostic.message.contains("type") {
            priority *= 1.3;
            return (
                priority.min(1.0),
                format!(
                    "Type '{}' may be relevant to type error resolution",
                    type_def.name
                ),
            );
        }

        (
            priority.min(1.0),
            format!("Type '{}' may be relevant to error resolution", type_def.name),
        )
    }

    pub fn score_variable(
        &self,
        var_ctx: &VariableContext,
        diagnostic: &Diagnostic,
    ) -> (f32, String) {
        let mut priority = self.config.local_variable_weight;

        // Higher priority if variable name appears in diagnostic
        if diagnostic.message.contains(&var_ctx.name) {
            priority *= 1.4;
            return (
                priority.min(1.0),
                format!(
                    "Variable '{}' is mentioned in the error message",
                    var_ctx.name
                ),
            );
        }

        // Slightly higher priority for variables close to the diagnostic line
        let distance = (var_ctx.line as i32 - diagnostic.range.start.line as i32).abs();
        if distance <= 3 {
            priority *= 1.2;
            return (
                priority.min(1.0),
                format!(
                    "Variable '{}' is in immediate scope at error location",
                    var_ctx.name
                ),
            );
        }

        (
            priority.min(1.0),
            format!("Variable '{}' is in scope at error location", var_ctx.name),
        )
    }

    pub fn score_call_hierarchy(
        &self,
        call_hierarchy: &CallHierarchy,
        diagnostic: &Diagnostic,
    ) -> (f32, String) {
        let mut priority = self.config.call_hierarchy_weight;

        // Check if any called functions appear in diagnostic
        for call in &call_hierarchy.callees {
            if diagnostic.message.contains(&call.function_name) {
                priority *= 1.3;
                return (
                    priority.min(1.0),
                    format!(
                        "Called function '{}' is mentioned in error",
                        call.function_name
                    ),
                );
            }
        }

        // Higher priority if we have many outgoing calls (complex function)
        if call_hierarchy.callees.len() > 3 {
            priority *= 1.2;
            return (
                priority.min(1.0),
                "Complex function with multiple calls provides important context".to_string(),
            );
        }

        (
            priority.min(1.0),
            "Function call hierarchy provides execution context".to_string(),
        )
    }

    pub fn score_dependency(
        &self,
        dep_info: &DependencyInfo,
        diagnostic: &Diagnostic,
    ) -> (f32, String) {
        let mut priority = self.config.dependency_weight;

        // Higher priority for dependencies that export symbols mentioned in diagnostic
        for symbol in &dep_info.imported_symbols {
            if diagnostic.message.contains(symbol) {
                priority *= 1.4;
                return (
                    priority.min(1.0),
                    format!(
                        "Dependency exports symbol '{}' mentioned in error",
                        symbol
                    ),
                );
            }
        }

        (
            priority.min(1.0),
            format!(
                "Cross-file dependency on '{}' may be relevant",
                dep_info.file_path
            ),
        )
    }
}