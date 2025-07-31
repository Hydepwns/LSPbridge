use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::core::database_pool::{DatabasePool, PoolConfig};

/// Builder for creating database pools with specific configurations
pub struct DatabasePoolBuilder {
    config: PoolConfig,
}

impl DatabasePoolBuilder {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            config: PoolConfig {
                db_path: db_path.as_ref().to_path_buf(),
                ..Default::default()
            }
        }
    }

    pub fn min_connections(mut self, min: usize) -> Self {
        self.config.min_connections = min;
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }

    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout = timeout;
        self
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.config.idle_timeout = timeout;
        self
    }

    pub fn enable_wal(mut self, enable: bool) -> Self {
        self.config.enable_wal = enable;
        self
    }

    pub async fn build(self) -> Result<Arc<DatabasePool>> {
        DatabasePool::new(self.config).await
    }
}