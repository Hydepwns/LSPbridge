use super::types::TokenWeights;
use crate::core::semantic_context::{
    CallHierarchy, ClassContext, DependencyInfo, FunctionContext, ImportContext, TypeDefinition,
    VariableContext,
};

pub struct TokenEstimator<'a> {
    weights: &'a TokenWeights,
}

impl<'a> TokenEstimator<'a> {
    pub fn new(weights: &'a TokenWeights) -> Self {
        Self { weights }
    }

    /// Estimate tokens for function context
    pub fn estimate_function(&self, func_ctx: &FunctionContext) -> usize {
        let body_lines = func_ctx.end_line.saturating_sub(func_ctx.start_line) + 1;
        self.weights.function_base_cost
            + (body_lines as f32 * self.weights.tokens_per_line) as usize
    }

    /// Estimate tokens for class context
    pub fn estimate_class(&self, class_ctx: &ClassContext) -> usize {
        let body_lines = class_ctx.end_line.saturating_sub(class_ctx.start_line) + 1;
        self.weights.class_base_cost
            + (body_lines as f32 * self.weights.tokens_per_line) as usize
    }

    /// Estimate tokens for import statement
    pub fn estimate_import(&self, _import_ctx: &ImportContext) -> usize {
        self.weights.import_cost
    }

    /// Estimate tokens for type definition
    pub fn estimate_type(&self, type_def: &TypeDefinition) -> usize {
        // Base cost plus additional cost based on definition length
        let definition_lines = type_def.definition.lines().count();
        self.weights.type_definition_cost
            + (definition_lines as f32 * self.weights.tokens_per_line * 0.5) as usize
    }

    /// Estimate tokens for variable context
    pub fn estimate_variable(&self, var_ctx: &VariableContext) -> usize {
        // Base cost plus small additional cost if it has type annotation or initialization
        let mut cost = self.weights.variable_cost;
        
        if var_ctx.type_annotation.is_some() {
            cost += 3; // Small cost for type annotation
        }
        
        if let Some(init) = &var_ctx.value {
            // Rough estimate based on initialization string length
            cost += (init.len() / 10).min(10);
        }
        
        cost
    }

    /// Estimate tokens for call hierarchy
    pub fn estimate_call_hierarchy(&self, call_hierarchy: &CallHierarchy) -> usize {
        let total_calls = call_hierarchy.callees.len() + call_hierarchy.callers.len();
        total_calls * self.weights.call_cost
    }

    /// Estimate tokens for dependency
    pub fn estimate_dependency(&self, dep_info: &DependencyInfo) -> usize {
        // Base cost plus additional cost for imported symbols
        self.weights.dependency_cost + (dep_info.imported_symbols.len() * 2)
    }

    /// Estimate total tokens for a code snippet
    pub fn estimate_code_snippet(&self, code: &str) -> usize {
        let lines = code.lines().count();
        (lines as f32 * self.weights.tokens_per_line) as usize
    }
}