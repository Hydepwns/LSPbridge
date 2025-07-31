use crate::query::{Query, QueryParser, QueryResult};
use crate::query::api::QueryApi;
use anyhow::Result;
use std::sync::Arc;

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