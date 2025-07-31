use crate::query::QueryResult;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Request structure for executing diagnostic queries.
/// 
/// Contains the query string and optional formatting/timeout parameters.
/// Also includes client information for rate limiting purposes.
/// 
/// # Examples
/// 
/// ```rust
/// use lspbridge::query::api::{QueryRequest, ResponseFormat};
/// 
/// let request = QueryRequest {
///     query: "severity:error file:*.rs".to_string(),
///     format: Some(ResponseFormat::Json),
///     timeout_ms: Some(5000),
///     client_info: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// The diagnostic query string to execute
    pub query: String,
    /// Optional response format (defaults to JSON)
    pub format: Option<ResponseFormat>,
    /// Optional timeout in milliseconds (defaults to 30000ms)
    pub timeout_ms: Option<u64>,
    /// Client information for rate limiting
    pub client_info: Option<ClientInfo>,
}

/// Client information for rate limiting and request tracking.
/// 
/// Used to identify clients for rate limiting purposes and provide
/// contextual information for query processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client IP address for rate limiting
    pub ip: Option<IpAddr>,
    /// User agent string for client identification
    pub user_agent: Option<String>,
    /// API key for authenticated requests
    pub api_key: Option<String>,
}

/// Available response formats for query results.
/// 
/// Different formats are suitable for different use cases:
/// - [`Json`] - Machine-readable structured data
/// - [`Csv`] - Spreadsheet-compatible tabular data
/// - [`Table`] - Human-readable console table format
/// - [`Markdown`] - Documentation-friendly markup format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    /// JSON format for programmatic processing
    Json,
    /// CSV format for spreadsheet applications
    Csv,
    /// ASCII table format for console display
    Table,
    /// Markdown format for documentation
    Markdown,
}

/// Response structure containing query results and metadata.
/// 
/// Provides the query result along with execution metadata including
/// timing information and rate limiting status.
/// 
/// # Examples
/// 
/// ```rust
/// // Successful response
/// if response.success {
///     if let Some(result) = response.result {
///         println!("Found {} diagnostics", result.diagnostics.len());
///     }
/// } else if let Some(error) = response.error {
///     eprintln!("Query failed: {}", error);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Whether the query executed successfully
    pub success: bool,
    /// Query result data (present if success=true)
    pub result: Option<QueryResult>,
    /// Error message (present if success=false)
    pub error: Option<String>,
    /// Query execution time in milliseconds
    pub query_time_ms: u64,
    /// Rate limiting information for this request
    pub rate_limit_status: Option<RateLimitStatus>,
}

/// Rate limiting status information included in query responses.
/// 
/// Provides clients with information about current rate limiting
/// state and guidance for future requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    /// Whether this request was rate limited
    pub limited: bool,
    /// Seconds to wait before retrying (if limited=true)
    pub retry_after_secs: Option<u64>,
    /// Number of requests remaining in current window
    pub requests_remaining: Option<u32>,
}

/// Query execution plan for debugging/optimization
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryPlan {
    pub query: String,
    pub estimated_rows: Option<usize>,
    pub indexes_used: Vec<String>,
    pub optimization_hints: Vec<String>,
}