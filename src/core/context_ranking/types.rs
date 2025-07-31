use crate::simple_builder;
use super::super::semantic_context::{
    CallHierarchy, ClassContext, DependencyInfo, FunctionContext, ImportContext, SemanticContext,
    TypeDefinition, VariableContext,
};

// Apply builder pattern to ContextRanker
simple_builder! {
    /// Manages context ranking and token budget allocation
    #[derive(Debug, Clone)]
    pub struct ContextRanker {
        /// Maximum tokens allowed for context (configurable)
        pub max_tokens: usize = 2000,
        /// Token cost estimation weights
        pub token_weights: TokenWeights = TokenWeights::new(),
        /// Context priority configuration
        pub priority_config: PriorityConfig = PriorityConfig::new(),
    }
}

// Apply builder pattern to PriorityConfig
simple_builder! {
    /// Configuration for context element priorities
    #[derive(Debug, Clone)]
    pub struct PriorityConfig {
        /// How much to prioritize function context containing the error
        pub function_context_weight: f32 = 1.0,
        /// How much to prioritize class/struct context
        pub class_context_weight: f32 = 0.8,
        /// How much to prioritize imports relevant to error types
        pub import_relevance_weight: f32 = 0.6,
        /// How much to prioritize type definitions mentioned in error
        pub type_definition_weight: f32 = 0.7,
        /// How much to prioritize local variables in scope
        pub local_variable_weight: f32 = 0.5,
        /// How much to prioritize call hierarchy information
        pub call_hierarchy_weight: f32 = 0.4,
        /// How much to prioritize cross-file dependencies
        pub dependency_weight: f32 = 0.3,
    }
}

// Apply builder pattern to TokenWeights
simple_builder! {
    /// Token cost estimation for different context elements
    #[derive(Debug, Clone)]
    pub struct TokenWeights {
        /// Estimated tokens per line of code
        pub tokens_per_line: f32 = 4.0,
        /// Base cost for including function context
        pub function_base_cost: usize = 50,
        /// Base cost for including class context
        pub class_base_cost: usize = 30,
        /// Cost per import statement
        pub import_cost: usize = 10,
        /// Cost per type definition
        pub type_definition_cost: usize = 25,
        /// Cost per local variable
        pub variable_cost: usize = 5,
        /// Cost per function call in hierarchy
        pub call_cost: usize = 15,
        /// Cost per dependency reference
        pub dependency_cost: usize = 20,
    }
}

/// Ranked context with priority scores and estimated token costs
#[derive(Debug, Clone)]
pub struct RankedContext {
    /// Original semantic context
    pub context: SemanticContext,
    /// Priority-ranked context elements
    pub ranked_elements: Vec<ContextElement>,
    /// Total estimated token cost
    pub estimated_tokens: usize,
    /// Context elements that fit within token budget
    pub budget_context: BudgetOptimizedContext,
}

/// Individual context element with priority and cost
#[derive(Debug, Clone)]
pub struct ContextElement {
    pub element_type: ContextElementType,
    pub priority_score: f32,
    pub estimated_tokens: usize,
    pub relevance_explanation: String,
    pub content: ContextContent,
}

#[derive(Debug, Clone)]
pub enum ContextElementType {
    FunctionContext,
    ClassContext,
    Import,
    TypeDefinition,
    LocalVariable,
    CallHierarchy,
    Dependency,
}

#[derive(Debug, Clone)]
pub enum ContextContent {
    Function(FunctionContext),
    Class(ClassContext),
    Import(ImportContext),
    Type(TypeDefinition),
    Variable(VariableContext),
    Calls(CallHierarchy),
    Dependency(DependencyInfo),
}

/// Context optimized for token budget constraints
#[derive(Debug, Clone)]
pub struct BudgetOptimizedContext {
    /// High priority elements that should always be included
    pub essential_context: Vec<ContextElement>,
    /// Medium priority elements included if budget allows
    pub supplementary_context: Vec<ContextElement>,
    /// Low priority elements that were excluded due to budget
    pub excluded_context: Vec<ContextElement>,
    /// Total tokens used by included context
    pub tokens_used: usize,
    /// Remaining token budget
    pub tokens_remaining: usize,
}