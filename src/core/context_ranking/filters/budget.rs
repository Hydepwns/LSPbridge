use anyhow::Result;
use crate::core::context_ranking::types::{BudgetOptimizedContext, ContextElement};

pub struct BudgetOptimizer;

impl BudgetOptimizer {
    /// Optimize context selection for token budget using greedy algorithm
    pub fn optimize(elements: &[ContextElement], max_tokens: usize) -> Result<BudgetOptimizedContext> {
        let mut essential = Vec::new();
        let mut supplementary = Vec::new();
        let mut excluded = Vec::new();
        let mut tokens_used = 0;

        // Priority thresholds
        let essential_threshold = 0.8;
        let supplementary_threshold = 0.4;

        // First pass: include essential items
        for element in elements {
            if element.priority_score >= essential_threshold {
                if tokens_used + element.estimated_tokens <= max_tokens {
                    tokens_used += element.estimated_tokens;
                    essential.push(element.clone());
                } else {
                    // Even high priority items might not fit
                    excluded.push(element.clone());
                }
            }
        }

        // Second pass: include supplementary items if budget allows
        for element in elements {
            if element.priority_score >= supplementary_threshold
                && element.priority_score < essential_threshold
                && !Self::already_included(&essential, element)
            {
                if tokens_used + element.estimated_tokens <= max_tokens {
                    tokens_used += element.estimated_tokens;
                    supplementary.push(element.clone());
                } else {
                    excluded.push(element.clone());
                }
            }
        }

        // Third pass: include remaining items if budget allows
        for element in elements {
            if element.priority_score < supplementary_threshold
                && !Self::already_included(&essential, element)
                && !Self::already_included(&supplementary, element)
            {
                if tokens_used + element.estimated_tokens <= max_tokens {
                    tokens_used += element.estimated_tokens;
                    supplementary.push(element.clone());
                } else {
                    excluded.push(element.clone());
                }
            }
        }

        Ok(BudgetOptimizedContext {
            essential_context: essential,
            supplementary_context: supplementary,
            excluded_context: excluded,
            tokens_used,
            tokens_remaining: max_tokens.saturating_sub(tokens_used),
        })
    }

    /// Alternative optimization using dynamic programming for optimal selection
    pub fn optimize_dynamic(elements: &[ContextElement], max_tokens: usize) -> Result<BudgetOptimizedContext> {
        // For small inputs, use the simple greedy algorithm
        if elements.len() <= 10 {
            return Self::optimize(elements, max_tokens);
        }

        // For larger inputs, we could implement a knapsack-style DP solution
        // but for now, fall back to greedy for performance
        Self::optimize(elements, max_tokens)
    }

    fn already_included(collection: &[ContextElement], element: &ContextElement) -> bool {
        collection.iter().any(|e| {
            match (&e.content, &element.content) {
                (crate::core::context_ranking::types::ContextContent::Function(f1),
                 crate::core::context_ranking::types::ContextContent::Function(f2)) => f1.name == f2.name,
                (crate::core::context_ranking::types::ContextContent::Class(c1),
                 crate::core::context_ranking::types::ContextContent::Class(c2)) => c1.name == c2.name,
                (crate::core::context_ranking::types::ContextContent::Type(t1),
                 crate::core::context_ranking::types::ContextContent::Type(t2)) => t1.name == t2.name,
                (crate::core::context_ranking::types::ContextContent::Variable(v1),
                 crate::core::context_ranking::types::ContextContent::Variable(v2)) => v1.name == v2.name,
                _ => false,
            }
        })
    }
}