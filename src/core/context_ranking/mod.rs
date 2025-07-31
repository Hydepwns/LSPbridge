//! # Context Ranking and Token Budget Management
//!
//! This module provides intelligent ranking and optimization of semantic context
//! based on relevance to diagnostics and token budget constraints for AI models.
//!
//! ## Key Components
//!
//! - **ContextRanker**: Main service for ranking and optimizing context
//! - **PriorityConfig**: Configurable weights for different context elements
//! - **TokenWeights**: Token cost estimation for budget management
//! - **RankedContext**: Output structure with prioritized context elements
//!
//! ## Ranking Algorithm
//!
//! The ranking system uses a multi-factor scoring approach:
//!
//! 1. **Relevance Scoring**: How directly related the context is to the diagnostic
//! 2. **Token Cost Estimation**: Estimated token consumption for each element
//! 3. **Priority Weighting**: Configurable importance weights by context type
//! 4. **Budget Optimization**: Greedy selection within token limits
//!
//! ## Usage Examples
//!
//! ```rust
//! use lsp_bridge::core::context_ranking::{ContextRanker, PriorityConfig, TokenWeights};
//!
//! // Create a context ranker with custom configuration
//! let ranker = ContextRanker::builder()
//!     .max_tokens(1500)
//!     .priority_config(
//!         PriorityConfig::builder()
//!             .function_context_weight(1.2)
//!             .class_context_weight(0.9)
//!             .build()
//!     )
//!     .token_weights(
//!         TokenWeights::builder()
//!             .tokens_per_line(4.5)
//!             .function_base_cost(60)
//!             .build()
//!     )
//!     .build();
//!
//! // Rank context for a diagnostic
//! let ranked = ranker.rank_context(semantic_context, &diagnostic)?;
//!
//! // Access optimized results
//! println!("Selected {} elements using {} tokens",
//!          ranked.elements.len(), ranked.tokens_used);
//!
//! // Get AI-formatted context
//! let formatted = format_context_for_ai(&ranked.optimized_context);
//! ```

pub mod algorithms;
pub mod filters;
pub mod formatter;
pub mod scorer;
pub mod token_estimator;
pub mod types;

pub use types::*;
pub use formatter::format_context_for_ai;

// Re-export key types for convenience
pub use types::{
    ContextRanker, PriorityConfig, TokenWeights, RankedContext, ContextElement,
    ContextElementType, ContextContent, BudgetOptimizedContext,
};