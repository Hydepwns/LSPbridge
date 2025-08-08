use tokio::time::interval;

use crate::core::database_pool::manager::ConnectionManager;
use crate::core::database_pool::types::PoolConfig;

/// Handles monitoring and maintenance of the connection pool
#[allow(dead_code)]
pub(crate) struct PoolMonitor {
    manager: &'static ConnectionManager,
    config: PoolConfig,
}

#[allow(dead_code)]
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