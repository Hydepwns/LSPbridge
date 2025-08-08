//! Query execution engine for LSP Bridge diagnostics
//!
//! This module provides a complete query execution system for searching and
//! analyzing diagnostic data. It supports multiple data sources (diagnostics,
//! files, history, trends) with filtering, aggregation, and caching capabilities.
//!
//! # Architecture
//!
//! The executor is built around several core components:
//!
//! - **Types**: Data structures for query results and values
//! - **Engines**: Specialized execution engines for each data source  
//! - **Filters**: Pattern matching and filtering logic with security validation
//! - **Processing**: Aggregation, sorting, and grouping utilities
//! - **Cache**: Result caching with TTL and performance optimization
//!
//! # Example Usage
//!
//! ```rust
//! use lsp_bridge::query::executor::{QueryExecutor, QueryResult};
//! use lsp_bridge::query::parser::Query;
//! use lsp_bridge::core::DiagnosticResult;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut executor = QueryExecutor::new();
//! let diagnostics = DiagnosticResult::new();
//! 
//! executor.with_diagnostics(diagnostics);
//!
//! // Execute a query (assuming you have a parsed Query)
//! // let query = parser.parse("SELECT COUNT(*) FROM diagnostics WHERE severity = 'error'")?;
//! // let result = executor.execute(&query).await?;
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod engines;
pub mod filters;
pub mod processing;
pub mod types;

// Re-export main types for convenience
pub use types::{FileStatistics, QueryMetadata, QueryResult, Row, Value};
pub use cache::{CacheStats, QueryCache, QueryCost, CostCategory};
pub use filters::{FilterEngine, ValueFilter};
pub use engines::{DiagnosticsEngine, FilesEngine, HistoryEngine, TrendsEngine, EngineFactory, QueryEngine};
pub use processing::{AggregationProcessor, SortingProcessor, GroupingProcessor};

use crate::core::{DiagnosticResult};
use crate::history::HistoryStorage;
use super::parser::{FromClause, Query};
use anyhow::{anyhow, Result};
use std::time::Instant;

/// Main query executor that coordinates all components
///
/// The QueryExecutor serves as the central coordinator for query execution,
/// managing data sources, caching, and delegating to specialized engines.
///
/// # Features
///
/// - **Multi-source Support**: Execute queries against diagnostics, files, history, and trends
/// - **Intelligent Caching**: Automatic result caching with configurable TTL
/// - **Security Validation**: Pattern validation to prevent injection attacks
/// - **Performance Optimization**: Query cost estimation and optimization hints
/// - **Flexible Filtering**: Regex and pattern-based filtering with safety checks
///
/// # Thread Safety
///
/// QueryExecutor is not thread-safe by default. If you need to use it across
/// threads, wrap it in appropriate synchronization primitives.
pub struct QueryExecutor {
    diagnostic_cache: Option<DiagnosticResult>,
    history_storage: Option<HistoryStorage>,
    query_cache: QueryCache,
    diagnostics_engine: DiagnosticsEngine,
    files_engine: FilesEngine,
    history_engine: HistoryEngine,
    trends_engine: TrendsEngine,
}

impl QueryExecutor {
    /// Create a new query executor
    ///
    /// Initializes all execution engines and sets up default caching behavior.
    pub fn new() -> Self {
        Self {
            diagnostic_cache: None,
            history_storage: None,
            query_cache: QueryCache::new(),
            diagnostics_engine: DiagnosticsEngine::new(),
            files_engine: FilesEngine::new(),
            history_engine: HistoryEngine::new(),
            trends_engine: TrendsEngine::new(),
        }
    }

    /// Create an executor with custom cache settings
    ///
    /// # Arguments
    ///
    /// * `cache_ttl_secs` - How long to cache results (in seconds)
    /// * `max_cache_entries` - Maximum number of cached results
    pub fn with_cache_settings(cache_ttl_secs: u64, max_cache_entries: usize) -> Self {
        Self {
            diagnostic_cache: None,
            history_storage: None,
            query_cache: QueryCache::with_settings(cache_ttl_secs, max_cache_entries),
            diagnostics_engine: DiagnosticsEngine::new(),
            files_engine: FilesEngine::new(),
            history_engine: HistoryEngine::new(),
            trends_engine: TrendsEngine::new(),
        }
    }

    /// Set diagnostic data for queries
    ///
    /// This data will be used for diagnostics and files queries.
    pub fn with_diagnostics(&mut self, diagnostics: DiagnosticResult) -> &mut Self {
        self.diagnostic_cache = Some(diagnostics);
        self
    }

    /// Set history storage for historical queries
    pub fn with_history(&mut self, history: HistoryStorage) -> &mut Self {
        self.history_storage = Some(history);
        self
    }

    /// Execute a query and return results
    ///
    /// This is the main entry point for query execution. It handles caching,
    /// validation, and delegation to the appropriate engine.
    ///
    /// # Arguments
    ///
    /// * `query` - The parsed query to execute
    ///
    /// # Returns
    ///
    /// * `Ok(QueryResult)` - Query results with timing and metadata
    /// * `Err(anyhow::Error)` - Query execution error
    ///
    /// # Performance
    ///
    /// - Results are automatically cached based on query structure
    /// - Expensive queries are identified and can be optimized
    /// - Filter validation prevents regex DoS attacks
    pub async fn execute(&mut self, query: &Query) -> Result<QueryResult> {
        let start_time = Instant::now();

        // Validate query safety
        if let Err(warnings) = cache::QueryValidator::validate_query_safety(query) {
            println!("Query performance warnings: {:?}", warnings);
        }

        // Check cache first
        let cache_key = cache::QueryValidator::generate_cache_key(query);
        if let Some(cached_result) = self.query_cache.get(&cache_key) {
            println!("Query cache hit for key: {}", cache_key);
            return Ok(cached_result);
        }

        // Execute query based on data source
        let mut result = match &query.from {
            FromClause::Diagnostics => self.execute_diagnostics_query(query).await?,
            FromClause::Files => self.execute_files_query(query).await?,
            FromClause::History => self.execute_history_query(query).await?,
            FromClause::Trends => self.execute_trends_query(query).await?,
            FromClause::Symbols => self.execute_symbols_query(query).await?,
            FromClause::References => self.execute_references_query(query).await?,
            FromClause::Projects => self.execute_projects_query(query).await?,
        };

        // Apply post-processing
        result = self.apply_post_processing(result, query)?;

        // Set execution time
        result.query_time_ms = start_time.elapsed().as_millis() as u64;

        // Cache the result
        self.query_cache.insert(cache_key, result.clone());
        println!("Cached query result with {} rows", result.rows.len());

        Ok(result)
    }

    /// Execute a query against diagnostic data
    async fn execute_diagnostics_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;

        self.diagnostics_engine.execute(query, diagnostics).await
    }

    /// Execute a query against file statistics
    async fn execute_files_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;

        self.files_engine.execute(query, diagnostics).await
    }

    /// Execute a query against historical data
    async fn execute_history_query(&self, query: &Query) -> Result<QueryResult> {
        let history = self
            .history_storage
            .as_ref()
            .ok_or_else(|| anyhow!("History storage not available"))?;

        self.history_engine.execute(query, history).await
    }

    /// Execute a query against trend data
    async fn execute_trends_query(&self, query: &Query) -> Result<QueryResult> {
        let history = self
            .history_storage
            .as_ref()
            .ok_or_else(|| anyhow!("History storage not available"))?;

        self.trends_engine.execute(query, history).await
    }

    /// Execute a query against symbol data
    async fn execute_symbols_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;
        
        let engine = engines::SymbolsEngine::new();
        engine.execute(query, diagnostics).await
    }

    /// Execute a query against reference data
    async fn execute_references_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;
        
        let engine = engines::ReferencesEngine::new();
        engine.execute(query, diagnostics).await
    }

    /// Execute a query against project data
    async fn execute_projects_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;
        
        let engine = engines::ProjectsEngine::new();
        engine.execute(query, diagnostics).await
    }

    /// Apply post-processing operations (sorting, limiting)
    fn apply_post_processing(&self, mut result: QueryResult, query: &Query) -> Result<QueryResult> {
        // Apply sorting if specified
        if let Some(order_by) = &query.order_by {
            processing::SortingProcessor::apply_sorting(&mut result.rows, &result.columns, order_by)?;
        }

        // Apply limit if specified
        let total_count = result.rows.len();
        if let Some(limit) = query.limit {
            result.rows.truncate(limit as usize);
        }
        result.total_count = total_count;

        Ok(result)
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        self.query_cache.stats()
    }

    /// Clear query cache
    pub fn clear_cache(&mut self) {
        self.query_cache.clear();
    }

    /// Configure cache settings
    pub fn configure_cache(&mut self, ttl_secs: u64, max_entries: usize) {
        self.query_cache.set_ttl(ttl_secs);
        self.query_cache.set_max_entries(max_entries);
    }

    /// Estimate query execution cost
    pub fn estimate_query_cost(&self, query: &Query) -> QueryCost {
        cache::QueryValidator::estimate_query_cost(query)
    }

    /// Check if executor has diagnostic data loaded
    pub fn has_diagnostics(&self) -> bool {
        self.diagnostic_cache.is_some()
    }

    /// Check if executor has history storage configured
    pub fn has_history(&self) -> bool {
        self.history_storage.is_some()
    }

    /// Get diagnostic data summary
    pub fn diagnostic_summary(&self) -> Option<DiagnosticSummary> {
        self.diagnostic_cache.as_ref().map(|diagnostics| {
            let mut total_diagnostics = 0;
            let file_count = diagnostics.diagnostics.len();
            
            for file_diagnostics in diagnostics.diagnostics.values() {
                total_diagnostics += file_diagnostics.len();
            }

            DiagnosticSummary {
                file_count,
                total_diagnostics,
            }
        })
    }
}

/// Summary of loaded diagnostic data
#[derive(Debug, Clone)]
pub struct DiagnosticSummary {
    pub file_count: usize,
    pub total_diagnostics: usize,
}

impl Default for QueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for executing a query
///
/// Creates a temporary executor and executes the query.
/// Useful for one-off queries without maintaining state.
///
/// # Arguments
///
/// * `query` - The query to execute
/// * `diagnostics` - Diagnostic data for the query
///
/// # Returns
///
/// Query results or execution error
pub async fn execute_query(query: &Query, diagnostics: DiagnosticResult) -> Result<QueryResult> {
    let mut executor = QueryExecutor::new();
    executor.with_diagnostics(diagnostics);
    executor.execute(query).await
}

/// Convenience function for executing a query with history
pub async fn execute_query_with_history(
    query: &Query,
    diagnostics: DiagnosticResult,
    history: HistoryStorage,
) -> Result<QueryResult> {
    let mut executor = QueryExecutor::new();
    executor.with_diagnostics(diagnostics).with_history(history);
    executor.execute(query).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Diagnostic, DiagnosticSeverity, Position, Range};
    use crate::query::parser::{SelectClause, QueryFilter};
    use std::path::PathBuf;

    fn create_test_diagnostic(severity: DiagnosticSeverity, message: &str) -> Diagnostic {
        Diagnostic {
            id: "1".to_string(),
            file: "test.rs".to_string(),
            range: Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 10 },
            },
            severity,
            message: message.to_string(),
            source: "rust".to_string(),
            code: None,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    #[tokio::test]
    async fn test_executor_diagnostics_query() {
        let mut executor = QueryExecutor::new();

        // Create test diagnostics
        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Error, "Type error"),
                create_test_diagnostic(DiagnosticSeverity::Warning, "Unused variable"),
            ],
        );

        executor.with_diagnostics(diagnostics);

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = executor.execute(&query).await.unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.rows[0].values[0], Value::Integer(2));
        assert_eq!(result.metadata.data_source, "diagnostics");
    }

    #[tokio::test]
    async fn test_executor_files_query() {
        let mut executor = QueryExecutor::new();

        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test1.rs"),
            vec![create_test_diagnostic(DiagnosticSeverity::Error, "Error 1")],
        );
        diagnostics.diagnostics.insert(
            PathBuf::from("test2.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Warning, "Warning 1"),
                create_test_diagnostic(DiagnosticSeverity::Warning, "Warning 2"),
            ],
        );

        executor.with_diagnostics(diagnostics);

        let query = Query {
            select: SelectClause::All,
            from: FromClause::Files,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = executor.execute(&query).await.unwrap();
        assert_eq!(result.total_count, 2);
        assert_eq!(result.metadata.data_source, "files");
        assert_eq!(result.columns, vec!["file", "errors", "warnings", "total"]);
    }

    #[tokio::test]
    async fn test_executor_caching() {
        let mut executor = QueryExecutor::new();

        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![create_test_diagnostic(DiagnosticSeverity::Error, "Error")],
        );

        executor.with_diagnostics(diagnostics);

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        // First execution should not be cached
        let result1 = executor.execute(&query).await.unwrap();
        assert!(!result1.metadata.cache_hit);

        // Second execution should be cached
        let result2 = executor.execute(&query).await.unwrap();
        assert!(result2.metadata.cache_hit);
    }

    #[test]
    fn test_executor_configuration() {
        let mut executor = QueryExecutor::new();
        
        assert!(!executor.has_diagnostics());
        assert!(!executor.has_history());

        let diagnostics = DiagnosticResult::new();
        executor.with_diagnostics(diagnostics);
        
        assert!(executor.has_diagnostics());
        assert!(executor.diagnostic_summary().is_some());

        let stats = executor.cache_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.ttl_seconds, 300); // Default TTL

        executor.configure_cache(600, 500);
        let new_stats = executor.cache_stats();
        assert_eq!(new_stats.ttl_seconds, 600);
        assert_eq!(new_stats.max_entries, 500);
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![create_test_diagnostic(DiagnosticSeverity::Error, "Error")],
        );

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = execute_query(&query, diagnostics).await.unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.rows[0].values[0], Value::Integer(1));
    }
}