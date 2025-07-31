use crate::query::api::{QueryApi, types::{QueryRequest, QueryPlan}};
use anyhow::Result;
use std::sync::Arc;

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