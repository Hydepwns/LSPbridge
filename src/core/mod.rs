pub mod async_processor;
pub mod config;
pub mod constants;
pub mod context_ranking;
pub mod dependency_analyzer;
pub mod diagnostic_grouping;
pub mod diagnostic_prioritization;
pub mod error_recovery;
pub mod errors;
pub mod incremental_processor;
pub mod io_utils;
pub mod macros;
pub mod memory_manager;
pub mod metrics;
pub mod persistent_cache;
pub mod semantic_context;
pub mod traits;
pub mod types;
pub mod utils;
// pub mod enhanced_processor;
pub mod dynamic_config;
pub mod git_integration;
pub mod health_dashboard;
pub mod simple_enhanced_processor;

pub use context_ranking::{
    format_context_for_ai, BudgetOptimizedContext, ContextContent, ContextElement,
    ContextElementType, ContextRanker, PriorityConfig, RankedContext, TokenWeights,
};
pub use dependency_analyzer::{
    DependencyAnalyzer, DependencyGraph, ExportInfo, ExternalFunctionCall, FileDependencies,
    ImportDependency, TypeReference,
};
pub use diagnostic_grouping::{DiagnosticGroup, DiagnosticGrouper, GroupingSummary};
pub use diagnostic_prioritization::{
    DiagnosticPrioritizer, FixRecommendation, PrioritizationSummary, PrioritizedDiagnostic,
};
pub use error_recovery::{
    CircuitBreaker, ErrorEvent, ErrorRecoverySystem, ErrorSeverity, RecoveryAction,
    RecoveryStrategy,
};
pub use incremental_processor::{FileEntry, FileHash, IncrementalProcessor, ProcessingStats};
pub use memory_manager::{BoundedCache, EvictionPolicy, MemoryConfig, MemoryReport};
pub use metrics::{HealthStatus, MetricsCollector, PerformanceSummary, ProcessingMetrics};
pub use persistent_cache::{CacheConfig, CacheEntry as PersistentCacheEntry, PersistentCache};
pub use semantic_context::{
    CallHierarchy, ClassContext, ContextExtractor, DependencyInfo, DependencyType, FunctionCall,
    FunctionContext, ImportContext, SemanticContext, TypeDefinition, VariableContext,
};
pub use traits::*;
pub use types::*;
// pub use enhanced_processor::{EnhancedIncrementalProcessor, EnhancedProcessorConfig, ComprehensiveStats, OverallHealthStatus};
pub use async_processor::{
    AsyncDiagnosticProcessor, ProcessedDiagnostic, ProcessingStats as AsyncProcessingStats,
};
pub use dynamic_config::{
    ConfigChange, ConfigChangeNotifier, ConfigChangeReceiver, DynamicConfig, DynamicConfigManager,
};
pub use errors::{
    AnalysisError, CacheError, ConfigError, DatabaseError, ExportError, FileError,
    LSPBridgeError, ParseError, ProcessingError,
};
pub use git_integration::{GitFileInfo, GitFileStatus, GitIntegration, GitRepositoryInfo};
pub use health_dashboard::{
    AlertSeverity, AlertThresholds, ComponentHealth, ComponentMetrics, ComponentStatus,
    DashboardMetrics, EffortLevel, HealthAlert, HealthDashboard, HealthMonitor, ImpactLevel,
    MonitoringConfig, PerformanceRecommendation, SystemHealthStatus,
};
pub use simple_enhanced_processor::{
    PerformanceSummary as SimplePerformanceSummary, SimpleEnhancedConfig, SimpleEnhancedProcessor,
};
