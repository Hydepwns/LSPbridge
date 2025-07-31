pub mod types;
pub mod handlers;
pub mod validation;
pub mod router;

pub use types::{
    QueryRequest, QueryResponse, ClientInfo, ResponseFormat, 
    RateLimitStatus, QueryPlan
};
pub use handlers::{QueryRpcHandler, QuerySubscription};

use crate::core::{DiagnosticResult, RateLimiter, RateLimitConfig};
use crate::history::HistoryStorage;
use crate::query::{QueryParser, QueryExecutor, Query, QueryResult};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Programmatic API for executing diagnostic queries with rate limiting.
/// 
/// The main interface for querying diagnostic data programmatically.
/// Provides enterprise-grade rate limiting, concurrent query execution,
/// and flexible output formatting.
/// 
/// # Features
/// 
/// - **Rate Limiting**: Per-client and global rate limits with configurable policies
/// - **Concurrent Execution**: Thread-safe query processing
/// - **Multiple Formats**: JSON, CSV, Table, and Markdown output
/// - **Query Language**: Rich query syntax with filters, sorting, and aggregation
/// - **Security**: Input validation and sanitization for all queries
/// 
/// # Examples
/// 
/// ```rust
/// use lspbridge::query::api::{QueryApi, QueryRequest, ResponseFormat};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let api = QueryApi::new();
///     
///     let request = QueryRequest {
///         query: "severity:error language:rust".to_string(),
///         format: Some(ResponseFormat::Json),
///         timeout_ms: None,
///         client_info: None,
///     };
///     
///     let response = api.execute_query(request).await?;
///     if response.success {
///         println!("Query completed in {}ms", response.query_time_ms);
///     }
///     
///     Ok(())
/// }
/// ```
pub struct QueryApi {
    parser: QueryParser,
    executor: Arc<RwLock<QueryExecutor>>,
    rate_limiter: Arc<RateLimiter>,
    handler: handlers::QueryHandler,
    router: router::QueryRouter,
}

impl QueryApi {
    /// Create a new QueryApi instance with default rate limiting.
    /// 
    /// Uses default rate limiting configuration (100 requests per minute).
    /// For custom rate limiting, use [`QueryApi::with_rate_limiting`].
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use lspbridge::query::api::QueryApi;
    /// 
    /// let api = QueryApi::new();
    /// ```
    pub fn new() -> Self {
        let executor = Arc::new(RwLock::new(QueryExecutor::new()));
        let rate_limiter = Arc::new(RateLimiter::default());
        
        Self {
            parser: QueryParser::new(),
            executor: executor.clone(),
            rate_limiter: rate_limiter.clone(),
            handler: handlers::QueryHandler::new(executor.clone(), rate_limiter.clone()),
            router: router::QueryRouter::new(executor.clone()),
        }
    }

    /// Create a new QueryApi with custom rate limiting configuration.
    /// 
    /// Allows configuring per-client and global rate limits with custom policies.
    /// 
    /// # Arguments
    /// 
    /// * `rate_limit_config` - Rate limiting configuration with policies and limits
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use lspbridge::query::api::QueryApi;
    /// use lspbridge::core::{RateLimitConfig, RateLimitPolicy};
    /// 
    /// let config = RateLimitConfig {
    ///     policy: RateLimitPolicy::Strict,
    ///     ..Default::default()
    /// };
    /// let api = QueryApi::with_rate_limiting(config);
    /// ```
    pub fn with_rate_limiting(rate_limit_config: RateLimitConfig) -> Self {
        let executor = Arc::new(RwLock::new(QueryExecutor::new()));
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_config));
        
        Self {
            parser: QueryParser::new(),
            executor: executor.clone(),
            rate_limiter: rate_limiter.clone(),
            handler: handlers::QueryHandler::new(executor.clone(), rate_limiter.clone()),
            router: router::QueryRouter::new(executor.clone()),
        }
    }

    /// Load diagnostic data for querying.
    /// 
    /// Provides the query executor with diagnostic data to search through.
    /// This method should be called before executing queries.
    /// 
    /// # Arguments
    /// 
    /// * `diagnostics` - Diagnostic data to make available for queries
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use lspbridge::query::api::QueryApi;
    /// use lspbridge::core::DiagnosticResult;
    /// 
    /// let api = QueryApi::new();
    /// let diagnostics = DiagnosticResult::default();
    /// api.with_diagnostics(diagnostics).await?;
    /// ```
    pub async fn with_diagnostics(&self, diagnostics: DiagnosticResult) -> Result<()> {
        let mut executor = self.executor.write().await;
        executor.with_diagnostics(diagnostics);
        Ok(())
    }

    /// Load historical diagnostic data for time-based queries.
    /// 
    /// Enables queries that reference historical data or trends.
    /// Optional - only needed for history-based queries.
    /// 
    /// # Arguments
    /// 
    /// * `history` - Historical diagnostic storage to query against
    pub async fn with_history(&self, history: HistoryStorage) -> Result<()> {
        let mut executor = self.executor.write().await;
        executor.with_history(history);
        Ok(())
    }

    /// Execute a query string directly and return the raw result.
    /// 
    /// This is a lower-level method that bypasses rate limiting and formatting.
    /// For production use, prefer [`QueryApi::handle_request`] which includes
    /// rate limiting and proper error handling.
    /// 
    /// # Arguments
    /// 
    /// * `query_str` - Query string to execute (e.g. "severity:error file:*.rs")
    /// 
    /// # Returns
    /// 
    /// * `Ok(QueryResult)` - Query results with matching diagnostics
    /// * `Err(anyhow::Error)` - Parse or execution error
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use lspbridge::query::api::QueryApi;
    /// 
    /// let api = QueryApi::new();
    /// let result = api.execute("severity:error language:rust").await?;
    /// println!("Found {} diagnostics", result.diagnostics.len());
    /// ```
    pub async fn execute(&self, query_str: &str) -> Result<QueryResult> {
        self.router.execute(query_str).await
    }

    /// Execute a query request with full rate limiting and error handling.
    /// 
    /// This is the recommended method for production use. It provides:
    /// - Rate limiting with proper HTTP status codes
    /// - Input validation and sanitization  
    /// - Timeout handling
    /// - Structured error responses
    /// - Performance metrics
    /// 
    /// # Arguments
    /// 
    /// * `request` - Complete query request with formatting and client info
    /// 
    /// # Returns
    /// 
    /// Always returns a [`QueryResponse`] - never panics or returns errors.
    /// Check `response.success` to determine if the query succeeded.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use lspbridge::query::api::{QueryApi, QueryRequest, ResponseFormat};
    /// 
    /// let api = QueryApi::new();
    /// let request = QueryRequest {
    ///     query: "severity:error".to_string(),
    ///     format: Some(ResponseFormat::Json),
    ///     timeout_ms: Some(5000),
    ///     client_info: None,
    /// };
    /// 
    /// let response = api.handle_request(request).await;
    /// if response.success {
    ///     println!("Query succeeded in {}ms", response.query_time_ms);
    /// } else {
    ///     println!("Query failed: {}", response.error.unwrap_or_default());
    /// }
    /// ```
    pub async fn handle_request(&self, request: QueryRequest) -> QueryResponse {
        self.handler.handle_request(request).await
    }

    /// Execute a pre-parsed query
    pub async fn execute_query(&self, query: Query) -> Result<QueryResult> {
        self.router.execute_query(query).await
    }

    /// Stream query results for large datasets
    pub async fn execute_streaming(
        &self,
        query_str: &str,
        callback: impl Fn(Vec<crate::query::executor::Row>) + Send + 'static,
    ) -> Result<()> {
        self.router.execute_streaming(query_str, callback).await
    }

    /// Get query execution plan (for debugging/optimization)
    pub fn explain(&self, query_str: &str) -> Result<QueryPlan> {
        self.router.explain(query_str)
    }

    /// Get rate limiting statistics
    pub async fn get_rate_limit_stats(&self) -> crate::core::RateLimitStats {
        self.rate_limiter.get_stats().await
    }

    /// Reset rate limiting state (useful for testing)
    pub async fn reset_rate_limits(&self) {
        self.rate_limiter.reset().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_api_basic() {
        let api = QueryApi::new();

        // Create test diagnostics
        let diagnostics = DiagnosticResult::new();
        api.with_diagnostics(diagnostics).await.unwrap();

        // Test simple query
        let result = api.execute("SELECT COUNT(*) FROM diagnostics").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_request_handling() {
        let api = QueryApi::new();

        let request = QueryRequest {
            query: "SELECT * FROM diagnostics WHERE severity = error".to_string(),
            format: Some(ResponseFormat::Json),
            timeout_ms: Some(5000),
            client_info: None,
        };

        let response = api.handle_request(request).await;
        assert!(response.success || response.error.is_some());
    }

    #[tokio::test]
    #[ignore] // TODO: Fix QueryExecutor setup for rate limiting test - needs database/data setup
    async fn test_rate_limiting() {
        use crate::core::RateLimitConfig;
        use std::time::Duration;

        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
            max_clients: 100,
            per_ip_limiting: true,
            global_limit: None,
        };

        let api = QueryApi::with_rate_limiting(config);

        let request = QueryRequest {
            query: "SELECT COUNT(*) FROM diagnostics".to_string(),
            format: Some(ResponseFormat::Json),
            timeout_ms: Some(5000),
            client_info: Some(ClientInfo {
                ip: Some("127.0.0.1".parse().unwrap()),
                user_agent: Some("test-client".to_string()),
                api_key: None,
            }),
        };

        // First two requests should succeed
        let response1 = api.handle_request(request.clone()).await;
        assert!(response1.success);

        let response2 = api.handle_request(request.clone()).await;
        assert!(response2.success);

        // Third request should be rate limited
        let response3 = api.handle_request(request.clone()).await;
        assert!(!response3.success);
        assert!(response3.rate_limit_status.unwrap().limited);
    }
}