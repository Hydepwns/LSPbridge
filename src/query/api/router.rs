use crate::query::{Query, QueryExecutor, QueryResult};
use crate::query::api::types::QueryPlan;
use crate::query::api::validation::QueryValidator;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API routing and execution coordination
pub struct QueryRouter {
    executor: Arc<RwLock<QueryExecutor>>,
    validator: QueryValidator,
}

impl QueryRouter {
    pub fn new(executor: Arc<RwLock<QueryExecutor>>) -> Self {
        Self {
            executor,
            validator: QueryValidator::new(),
        }
    }

    /// Execute a query string directly and return the raw result
    pub async fn execute(&self, query_str: &str) -> Result<QueryResult> {
        let query = self.validator.validate_query(query_str)?;
        self.execute_query(query).await
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
        callback: impl Fn(Vec<crate::query::executor::Row>) + Send + 'static,
    ) -> Result<()> {
        // Parse and validate query
        let query = self.validator.validate_query(query_str)?;

        // For now, execute normally and call callback with all results
        // In a full implementation, this would stream results as they're processed
        let result = self.execute_query(query).await?;
        callback(result.rows);

        Ok(())
    }

    /// Get query execution plan (for debugging/optimization)
    pub fn explain(&self, query_str: &str) -> Result<QueryPlan> {
        let query = self.validator.validate_query(query_str)?;

        Ok(QueryPlan {
            query: format!("{:?}", query),
            estimated_rows: None,
            indexes_used: vec![],
            optimization_hints: self.validator.get_optimization_hints(&query),
        })
    }
}