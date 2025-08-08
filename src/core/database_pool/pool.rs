use anyhow::{Context, Result};
use rusqlite::Connection;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::core::database_pool::{
    connection::PooledConnection,
    manager::ConnectionManager,
    types::{PoolConfig, PoolStats},
};

/// A thread-safe database connection pool for SQLite
pub struct DatabasePool {
    pub(crate) config: PoolConfig,
    manager: ConnectionManager,
    /// Semaphore to limit concurrent connections
    semaphore: Semaphore,
}

impl DatabasePool {
    /// Create a new database connection pool
    pub async fn new(config: PoolConfig) -> Result<Arc<Self>> {
        // Ensure database directory exists
        if let Some(parent) = config.db_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create database directory: {parent:?}"))?;
        }

        let manager = ConnectionManager::new(config.clone());
        
        let pool = Arc::new(Self {
            semaphore: Semaphore::new(config.max_connections),
            config: config.clone(),
            manager,
        });

        // Initialize minimum connections
        pool.manager.initialize_connections().await?;

        // Start maintenance task
        let maintenance_pool = pool.clone();
        tokio::spawn(async move {
            maintenance_pool.maintenance_loop().await;
        });

        info!(
            "Database pool initialized: {} min, {} max connections for {:?}",
            config.min_connections,
            config.max_connections,
            config.db_path
        );

        Ok(pool)
    }

    /// Get a connection from the pool
    pub async fn get_connection(self: &Arc<Self>) -> Result<PooledConnection> {
        let start_time = Instant::now();
        
        // Wait for available slot
        let _permit = timeout(self.config.connection_timeout, self.semaphore.acquire())
            .await
            .context("Timeout waiting for connection slot")?
            .context("Semaphore closed")?;

        // Try to get existing connection
        let connection = self.manager.get_connection().await;

        let conn = if let Some(mut internal_conn) = connection {
            internal_conn.touch();
            internal_conn.connection
        } else {
            // Create new connection
            self.manager.create_connection().await?
        };

        // Update statistics
        let wait_time = start_time.elapsed();
        self.manager.record_acquisition(wait_time).await;

        debug!("Connection acquired in {:?}", wait_time);
        Ok(PooledConnection::new(conn, Arc::clone(self)))
    }

    /// Execute a closure with a pooled connection
    pub async fn with_connection<F, R>(self: &Arc<Self>, f: F) -> Result<R>
    where
        F: FnOnce(&mut Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let mut conn = self.get_connection().await?;
        let result = tokio::task::spawn_blocking(move || {
            conn.execute(f)
        }).await
        .context("Task panicked")?;
        
        result
    }

    /// Execute a read-only query with a pooled connection
    pub async fn with_read_connection<F, R>(self: &Arc<Self>, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let mut conn = self.get_connection().await?;
        let result = tokio::task::spawn_blocking(move || {
            let connection = conn.connection();
            f(connection)
        }).await
        .context("Task panicked")?;
        
        result
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        self.manager.get_stats().await
    }

    /// Close all connections and shut down the pool
    pub async fn shutdown(&self) {
        self.manager.shutdown().await;
    }

    /// Return a connection to the pool (called by PooledConnection)
    pub(crate) async fn return_connection(&self, connection: Connection, idle_time: Duration) {
        self.manager.return_connection(connection, idle_time).await;
    }

    /// Run maintenance loop
    async fn maintenance_loop(&self) {
        let mut interval = tokio::time::interval(self.config.maintenance_interval);
        
        loop {
            interval.tick().await;
            self.manager.perform_maintenance().await;
        }
    }
}