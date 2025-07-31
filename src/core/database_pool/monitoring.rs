use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info};

use crate::core::database_pool::manager::ConnectionManager;
use crate::core::database_pool::types::PoolConfig;

/// Handles monitoring and maintenance of the connection pool
pub(crate) struct PoolMonitor {
    manager: &'static ConnectionManager,
    config: PoolConfig,
}

impl PoolMonitor {
    pub fn new(manager: &'static ConnectionManager, config: PoolConfig) -> Self {
        Self { manager, config }
    }

    /// Start the maintenance loop
    pub async fn start_maintenance_loop(&self) {
        let mut interval = interval(self.config.maintenance_interval);
        
        loop {
            interval.tick().await;
            self.manager.perform_maintenance().await;
        }
    }
}