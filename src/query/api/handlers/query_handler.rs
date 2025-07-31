use crate::core::{RateLimiter, RateLimitResult, extract_client_id};
use crate::query::{QueryExecutor, QueryResult};
use crate::query::api::types::{QueryRequest, QueryResponse, RateLimitStatus};
use crate::query::api::validation::QueryValidator;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main query handler that processes query requests with rate limiting
pub struct QueryHandler {
    executor: Arc<RwLock<QueryExecutor>>,
    rate_limiter: Arc<RateLimiter>,
    validator: QueryValidator,
}

impl QueryHandler {
    pub fn new(executor: Arc<RwLock<QueryExecutor>>, rate_limiter: Arc<RateLimiter>) -> Self {
        Self {
            executor,
            rate_limiter,
            validator: QueryValidator::new(),
        }
    }

    /// Handle a query request with full rate limiting and error handling
    pub async fn handle_request(&self, request: QueryRequest) -> QueryResponse {
        let start_time = std::time::Instant::now();

        // Extract client ID for rate limiting
        let client_id = if let Some(ref client_info) = request.client_info {
            extract_client_id(
                client_info.ip,
                client_info.user_agent.as_deref(),
                client_info.api_key.as_deref(),
            )
        } else {
            "anonymous".to_string()
        };

        // Check rate limit
        let rate_limit_result = match self.rate_limiter.check_request(&client_id).await {
            Ok(result) => result,
            Err(e) => {
                return QueryResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Rate limit check failed: {}", e)),
                    query_time_ms: start_time.elapsed().as_millis() as u64,
                    rate_limit_status: Some(RateLimitStatus {
                        limited: true,
                        retry_after_secs: None,
                        requests_remaining: None,
                    }),
                };
            }
        };

        // Handle rate limiting
        if !rate_limit_result.is_allowed() {
            let retry_after_secs = match &rate_limit_result {
                RateLimitResult::ClientLimitExceeded { retry_after } => {
                    retry_after.map(|d| d.as_secs())
                }
                _ => None,
            };

            return QueryResponse {
                success: false,
                result: None,
                error: rate_limit_result.error_message(),
                query_time_ms: start_time.elapsed().as_millis() as u64,
                rate_limit_status: Some(RateLimitStatus {
                    limited: true,
                    retry_after_secs,
                    requests_remaining: Some(0),
                }),
            };
        }

        // Validate and execute the query
        match self.validate_and_execute(&request.query).await {
            Ok(mut result) => {
                result.query_time_ms = start_time.elapsed().as_millis() as u64;

                QueryResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                    query_time_ms: start_time.elapsed().as_millis() as u64,
                    rate_limit_status: Some(RateLimitStatus {
                        limited: false,
                        retry_after_secs: None,
                        requests_remaining: None, // Would need to track this for precise counts
                    }),
                }
            }
            Err(e) => QueryResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                query_time_ms: start_time.elapsed().as_millis() as u64,
                rate_limit_status: Some(RateLimitStatus {
                    limited: false,
                    retry_after_secs: None,
                    requests_remaining: None,
                }),
            },
        }
    }

    /// Validate and execute a query
    async fn validate_and_execute(&self, query_str: &str) -> anyhow::Result<QueryResult> {
        // Validate query
        let query = self.validator.validate_query(query_str)?;
        
        // Execute query
        let mut executor = self.executor.write().await;
        executor.execute(&query).await
    }
}