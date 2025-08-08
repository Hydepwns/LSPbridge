use anyhow::{Context, Result};
use rusqlite::Connection;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use std::collections::VecDeque;
use tracing::{debug, info};

use crate::core::database_pool::types::{PoolConfig, PoolStats};

/// Internal connection with metadata
pub(crate) struct InternalConnection {
    pub connection: Connection,
    #[allow(dead_code)]
    pub created_at: Instant,
    pub last_used: Instant,
}

impl InternalConnection {
    pub fn new(connection: Connection) -> Self {
        let now = Instant::now();
        Self {
            connection,
            created_at: now,
            last_used: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    pub fn idle_time(&self) -> Duration {
        self.last_used.elapsed()
    }

    pub fn is_expired(&self, idle_timeout: Duration) -> bool {
        self.idle_time() > idle_timeout
    }
}

/// Manages the lifecycle of database connections
pub(crate) struct ConnectionManager {
    config: PoolConfig,
    pub(crate) connections: Mutex<VecDeque<InternalConnection>>,
    stats: Mutex<PoolStats>,
}

impl ConnectionManager {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            connections: Mutex::new(VecDeque::new()),
            stats: Mutex::new(PoolStats::default()),
        }
    }

    /// Initialize minimum connections
    pub async fn initialize_connections(&self) -> Result<()> {
        for _ in 0..self.config.min_connections {
            let conn = self.create_connection().await?;
            let internal_conn = InternalConnection::new(conn);
            
            let mut connections = self.connections.lock().await;
            connections.push_back(internal_conn);
        }
        
        let mut stats = self.stats.lock().await;
        stats.total_connections_created += self.config.min_connections as u64;
        
        Ok(())
    }

    /// Create a new database connection
    pub async fn create_connection(&self) -> Result<Connection> {
        let conn = tokio::task::spawn_blocking({
            let db_path = self.config.db_path.clone();
            let flags = self.config.open_flags;
            let enable_wal = self.config.enable_wal;
            
            move || -> Result<Connection> {
                let conn = Connection::open_with_flags(&db_path, flags)
                    .with_context(|| format!("Failed to open database: {db_path:?}"))?;

                // Optimize connection settings
                conn.execute_batch(&format!(
                    r#"
                    PRAGMA synchronous = NORMAL;
                    PRAGMA cache_size = -64000;
                    PRAGMA temp_store = memory;
                    PRAGMA mmap_size = 268435456;
                    {}
                    "#,
                    if enable_wal { "PRAGMA journal_mode = WAL;" } else { "" }
                ))?;

                Ok(conn)
            }
        }).await
        .context("Task panicked")??;

        let mut stats = self.stats.lock().await;
        stats.total_connections_created += 1;
        
        debug!("Created new database connection");
        Ok(conn)
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Option<InternalConnection> {
        let mut connections = self.connections.lock().await;
        connections.pop_front()
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, connection: Connection, idle_time: Duration) {
        // Don't return connection if it's been idle too long
        if idle_time > self.config.idle_timeout {
            let mut stats = self.stats.lock().await;
            stats.total_connections_destroyed += 1;
            stats.current_active_connections = stats.current_active_connections.saturating_sub(1);
            debug!("Connection destroyed due to idle timeout");
            return;
        }

        // Return to pool
        let internal_conn = InternalConnection::new(connection);
        let mut connections = self.connections.lock().await;
        connections.push_back(internal_conn);
        
        let mut stats = self.stats.lock().await;
        stats.current_active_connections = stats.current_active_connections.saturating_sub(1);
        
        debug!("Connection returned to pool");
    }

    /// Update statistics for connection acquisition
    pub async fn record_acquisition(&self, wait_time: Duration) {
        let mut stats = self.stats.lock().await;
        stats.total_requests += 1;
        stats.total_wait_time += wait_time;
        if wait_time > stats.max_wait_time {
            stats.max_wait_time = wait_time;
        }
        stats.current_active_connections += 1;
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> PoolStats {
        let stats = self.stats.lock().await;
        let connections = self.connections.lock().await;
        
        PoolStats {
            current_idle_connections: connections.len(),
            ..stats.clone()
        }
    }

    /// Perform maintenance on the connection pool
    pub async fn perform_maintenance(&self) {
        let mut connections = self.connections.lock().await;
        let mut stats = self.stats.lock().await;
        
        // Remove expired connections
        let initial_count = connections.len();
        connections.retain(|conn| !conn.is_expired(self.config.idle_timeout));
        let removed_count = initial_count - connections.len();
        
        if removed_count > 0 {
            stats.total_connections_destroyed += removed_count as u64;
            debug!("Maintenance: removed {} expired connections", removed_count);
        }

        // Ensure minimum connections
        let current_total = connections.len() + stats.current_active_connections;
        if current_total < self.config.min_connections {
            let needed = self.config.min_connections - current_total;
            drop(connections); // Release lock before creating connections
            drop(stats);
            
            for _ in 0..needed {
                if let Ok(conn) = self.create_connection().await {
                    let internal_conn = InternalConnection::new(conn);
                    let mut connections = self.connections.lock().await;
                    connections.push_back(internal_conn);
                }
            }
        }
    }

    /// Close all connections
    pub async fn shutdown(&self) {
        info!("Shutting down connection manager");
        let mut connections = self.connections.lock().await;
        connections.clear();
        
        let mut stats = self.stats.lock().await;
        stats.current_active_connections = 0;
        stats.current_idle_connections = 0;
    }
}