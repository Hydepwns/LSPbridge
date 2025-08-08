use anyhow::Result;
use crate::core::semantic_context::SemanticContext;
use crate::core::types::Diagnostic;

use super::algorithms::{RelevanceScorer, ProximityScorer};
use super::filters::BudgetOptimizer;
use super::token_estimator::TokenEstimator;
use super::types::{
    ContextContent, ContextElement, ContextElementType, ContextRanker,
    RankedContext,
};

impl ContextRanker {
    /// Rank and optimize context for the given diagnostic and token budget
    pub fn rank_context(
        &self,
        context: SemanticContext,
        diagnostic: &Diagnostic,
    ) -> Result<RankedContext> {
        // Initialize scoring components
        let relevance_scorer = RelevanceScorer::new(&self.priority_config);
        let token_estimator = TokenEstimator::new(&self.token_weights);
        
        // Collect and score all context elements
        let mut elements = Vec::new();

        // Score function context
        if let Some(func_ctx) = &context.function_context {
            let (priority, explanation) = relevance_scorer.score_function(func_ctx, diagnostic);
            let tokens = token_estimator.estimate_function(func_ctx);
            
            elements.push(ContextElement {
                element_type: ContextElementType::FunctionContext,
                priority_score: priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Function(func_ctx.clone()),
            });
        }

        // Score class context
        if let Some(class_ctx) = &context.class_context {
            let (priority, explanation) = relevance_scorer.score_class(class_ctx, diagnostic);
            let tokens = token_estimator.estimate_class(class_ctx);
            
            elements.push(ContextElement {
                element_type: ContextElementType::ClassContext,
                priority_score: priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Class(class_ctx.clone()),
            });
        }

        // Score imports
        for import in &context.imports {
            let (priority, explanation) = relevance_scorer.score_import(import, diagnostic);
            let tokens = token_estimator.estimate_import(import);
            
            elements.push(ContextElement {
                element_type: ContextElementType::Import,
                priority_score: priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Import(import.clone()),
            });
        }

        // Score type definitions
        for type_def in &context.type_definitions {
            let (priority, explanation) = relevance_scorer.score_type(type_def, diagnostic);
            let tokens = token_estimator.estimate_type(type_def);
            
            elements.push(ContextElement {
                element_type: ContextElementType::TypeDefinition,
                priority_score: priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Type(type_def.clone()),
            });
        }

        // Score local variables
        for var in &context.local_variables {
            let (priority, explanation) = relevance_scorer.score_variable(var, diagnostic);
            
            // Apply proximity scoring as well
            let proximity_boost = ProximityScorer::score_variable_proximity(var, diagnostic);
            let final_priority = priority * proximity_boost;
            
            let tokens = token_estimator.estimate_variable(var);
            
            elements.push(ContextElement {
                element_type: ContextElementType::LocalVariable,
                priority_score: final_priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Variable(var.clone()),
            });
        }

        // Score call hierarchy
        if !context.call_hierarchy.callees.is_empty() || !context.call_hierarchy.callers.is_empty() {
            let (priority, explanation) = relevance_scorer.score_call_hierarchy(&context.call_hierarchy, diagnostic);
            let tokens = token_estimator.estimate_call_hierarchy(&context.call_hierarchy);
            
            elements.push(ContextElement {
                element_type: ContextElementType::CallHierarchy,
                priority_score: priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Calls(context.call_hierarchy.clone()),
            });
        }

        // Score dependencies
        for dep in &context.dependencies {
            let (priority, explanation) = relevance_scorer.score_dependency(dep, diagnostic);
            let tokens = token_estimator.estimate_dependency(dep);
            
            elements.push(ContextElement {
                element_type: ContextElementType::Dependency,
                priority_score: priority,
                estimated_tokens: tokens,
                relevance_explanation: explanation,
                content: ContextContent::Dependency(dep.clone()),
            });
        }

        // Sort by priority score (highest first)
        elements.sort_by(|a, b| {
            b.priority_score
                .partial_cmp(&a.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Calculate total estimated tokens
        let estimated_tokens = elements.iter().map(|e| e.estimated_tokens).sum();

        // Optimize for budget
        let budget_context = BudgetOptimizer::optimize(&elements, self.max_tokens)?;

        Ok(RankedContext {
            context,
            ranked_elements: elements,
            estimated_tokens,
            budget_context,
        })
    }
}