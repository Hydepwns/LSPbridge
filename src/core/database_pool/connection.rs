use anyhow::Result;
use rusqlite::Connection;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::OwnedSemaphorePermit;

use crate::core::database_pool::{DatabasePool, ConnectionStats};

/// A pooled database connection with automatic return-to-pool
pub struct PooledConnection {
    connection: Option<Connection>,
    pool: Arc<DatabasePool>,
    created_at: Instant,
    used_at: Instant,
    // Hold the semaphore permit until connection is returned
    _permit: OwnedSemaphorePermit,
}

impl PooledConnection {
    pub(crate) fn new(connection: Connection, pool: Arc<DatabasePool>, permit: OwnedSemaphorePermit) -> Self {
        let now = Instant::now();
        Self {
            connection: Some(connection),
            pool,
            created_at: now,
            used_at: now,
            _permit: permit,
        }
    }

    /// Get a reference to the underlying connection
    pub fn connection(&mut self) -> &mut Connection {
        self.used_at = Instant::now();
        self.connection.as_mut().expect("Connection should be available")
    }

    /// Execute a closure with the connection, automatically handling errors
    pub fn execute<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Connection) -> Result<R>,
    {
        self.used_at = Instant::now();
        let conn = self.connection.as_mut().expect("Connection should be available");
        f(conn)
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        ConnectionStats {
            created_at: self.created_at,
            last_used: self.used_at,
            age: self.created_at.elapsed(),
            idle_time: self.used_at.elapsed(),
        }
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            let pool = self.pool.clone();
            let idle_time = self.used_at.elapsed();
            
            // Return connection to pool in a background task
            tokio::spawn(async move {
                pool.return_connection(conn, idle_time).await;
            });
        }
    }
}