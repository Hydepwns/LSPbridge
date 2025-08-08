pub mod api;
pub mod executor;
pub mod parser;
pub mod repl;

pub use api::{QueryApi, QueryRequest, QueryResponse};
pub use executor::{QueryExecutor, QueryResult};
pub use parser::{Query, QueryAggregation, QueryFilter, QueryParser};
pub use repl::InteractiveRepl;

use anyhow::Result;
use std::path::PathBuf;

/// Simplified query engine for tests and basic usage
pub struct QueryEngine {
    api: QueryApi,
}

impl QueryEngine {
    /// Create a new QueryEngine with optional database path
    pub async fn new(_db_path: Option<PathBuf>) -> Result<Self> {
        // Database path configuration reserved for future use
        let api = QueryApi::new();
        
        Ok(Self { api })
    }
    
    /// Execute a query and return results
    pub async fn query(&self, query: &str) -> Result<QueryResult> {
        use crate::query::QueryRequest;
        
        let request = QueryRequest {
            query: query.to_string(),
            format: None,
            timeout_ms: None,
            client_info: None,
        };
        
        let response = self.api.handle_request(request).await;
        
        if response.success {
            response.result.ok_or_else(|| anyhow::anyhow!("Query succeeded but no result returned"))
        } else {
            Err(anyhow::anyhow!("Query failed: {}", response.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    /// Get all diagnostics with optional filtering
    pub async fn get_all_diagnostics(&self) -> Result<Vec<crate::core::Diagnostic>> {
        // Use a wildcard query to get all diagnostics
        let _result = self.query("*").await?;
        
        // Convert QueryResult rows to Diagnostic objects
        // Conversion from QueryResult to Diagnostics not yet implemented
        Ok(vec![])
    }
    
    /// Store a diagnostic in the query engine database
    pub async fn store_diagnostic(&mut self, _diagnostic: &crate::core::Diagnostic) -> Result<()> {
        // Diagnostic storage not yet implemented
        Ok(())
    }
    
    /// Query diagnostics with options
    pub async fn query_diagnostics(&self, _options: &QueryOptions) -> Result<Vec<crate::core::Diagnostic>> {
        // Query with options not yet implemented
        Ok(vec![])
    }
}

/// Query options for diagnostic queries
#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub severity: Option<crate::core::DiagnosticSeverity>,
    pub source: Option<String>,
    pub file_pattern: Option<String>,
    pub limit: Option<usize>,
}
