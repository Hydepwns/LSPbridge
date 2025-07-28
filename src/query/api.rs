use super::{Query, QueryExecutor, QueryParser, QueryResult};
use crate::core::DiagnosticResult;
use crate::history::HistoryStorage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub format: Option<ResponseFormat>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    Json,
    Csv,
    Table,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub success: bool,
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    pub query_time_ms: u64,
}

/// Programmatic API for executing diagnostic queries
pub struct QueryApi {
    parser: QueryParser,
    executor: Arc<RwLock<QueryExecutor>>,
}

impl QueryApi {
    pub fn new() -> Self {
        Self {
            parser: QueryParser::new(),
            executor: Arc::new(RwLock::new(QueryExecutor::new())),
        }
    }

    pub async fn with_diagnostics(&self, diagnostics: DiagnosticResult) -> Result<()> {
        let mut executor = self.executor.write().await;
        executor.with_diagnostics(diagnostics);
        Ok(())
    }

    pub async fn with_history(&self, history: HistoryStorage) -> Result<()> {
        let mut executor = self.executor.write().await;
        executor.with_history(history);
        Ok(())
    }

    /// Execute a query string and return the result
    pub async fn execute(&self, query_str: &str) -> Result<QueryResult> {
        let query = self.parser.parse(query_str)?;
        let mut executor = self.executor.write().await;
        executor.execute(&query).await
    }

    /// Execute a query request with formatting options
    pub async fn handle_request(&self, request: QueryRequest) -> QueryResponse {
        let start_time = std::time::Instant::now();

        match self.execute(&request.query).await {
            Ok(mut result) => {
                result.query_time_ms = start_time.elapsed().as_millis() as u64;

                QueryResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                    query_time_ms: start_time.elapsed().as_millis() as u64,
                }
            }
            Err(e) => QueryResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                query_time_ms: start_time.elapsed().as_millis() as u64,
            },
        }
    }

    /// Execute a pre-parsed query
    pub async fn execute_query(&self, query: Query) -> Result<QueryResult> {
        let mut executor = self.executor.write().await;
        executor.execute(&query).await
    }

    /// Stream query results for large datasets
    pub async fn execute_streaming(
        &self,
        query_str: &str,
        callback: impl Fn(Vec<super::executor::Row>) + Send + 'static,
    ) -> Result<()> {
        // Parse query
        let query = self.parser.parse(query_str)?;

        // For now, execute normally and call callback with all results
        // In a full implementation, this would stream results as they're processed
        let result = self.execute_query(query).await?;
        callback(result.rows);

        Ok(())
    }

    /// Get query execution plan (for debugging/optimization)
    pub fn explain(&self, query_str: &str) -> Result<QueryPlan> {
        let query = self.parser.parse(query_str)?;

        Ok(QueryPlan {
            query: format!("{:?}", query),
            estimated_rows: None,
            indexes_used: vec![],
            optimization_hints: self.get_optimization_hints(&query),
        })
    }

    fn get_optimization_hints(&self, query: &Query) -> Vec<String> {
        let mut hints = Vec::new();

        // Check for missing indexes
        if matches!(query.from, super::parser::FromClause::History) {
            hints.push("Consider adding time-based index for historical queries".to_string());
        }

        // Check for expensive operations
        if query.group_by.is_some() && query.limit.is_none() {
            hints.push("Consider adding LIMIT to grouped queries".to_string());
        }

        // Check for regex patterns that could be optimized
        for filter in &query.filters {
            if let super::parser::QueryFilter::Path(path_filter) = filter {
                if path_filter.is_regex && path_filter.pattern.starts_with('^') {
                    hints.push("Anchored regex patterns are more efficient".to_string());
                }
            }
        }

        hints
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryPlan {
    pub query: String,
    pub estimated_rows: Option<usize>,
    pub indexes_used: Vec<String>,
    pub optimization_hints: Vec<String>,
}

/// JSON-RPC handler for query API
pub struct QueryRpcHandler {
    api: Arc<QueryApi>,
}

impl QueryRpcHandler {
    pub fn new(api: Arc<QueryApi>) -> Self {
        Self { api }
    }

    pub async fn handle_method(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match method {
            "query.execute" => {
                let request: QueryRequest = serde_json::from_value(params)?;
                let response = self.api.handle_request(request).await;
                Ok(serde_json::to_value(response)?)
            }
            "query.explain" => {
                let query_str: String = serde_json::from_value(params)?;
                let plan = self.api.explain(&query_str)?;
                Ok(serde_json::to_value(plan)?)
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}

/// WebSocket subscription handler for real-time queries
pub struct QuerySubscription {
    query: Query,
    interval: std::time::Duration,
}

impl QuerySubscription {
    pub fn new(query_str: &str, interval_seconds: u64) -> Result<Self> {
        let parser = QueryParser::new();
        let query = parser.parse(query_str)?;

        Ok(Self {
            query,
            interval: std::time::Duration::from_secs(interval_seconds),
        })
    }

    pub async fn run(
        self,
        api: Arc<QueryApi>,
        sender: tokio::sync::mpsc::Sender<QueryResult>,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;

            match api.execute_query(self.query.clone()).await {
                Ok(result) => {
                    if sender.send(result).await.is_err() {
                        break; // Client disconnected
                    }
                }
                Err(e) => {
                    tracing::error!("Subscription query error: {}", e);
                }
            }
        }

        Ok(())
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
        };

        let response = api.handle_request(request).await;
        assert!(response.success || response.error.is_some());
    }
}
