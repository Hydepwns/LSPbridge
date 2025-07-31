use rusqlite::OpenFlags;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for database connection pooling
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Database file path
    pub db_path: PathBuf,
    /// Minimum number of connections to maintain
    pub min_connections: usize,
    /// Maximum number of connections allowed
    pub max_connections: usize,
    /// Maximum time to wait for a connection
    pub connection_timeout: Duration,
    /// Maximum time a connection can be idle before being closed
    pub idle_timeout: Duration,
    /// How often to run connection maintenance
    pub maintenance_interval: Duration,
    /// Enable WAL mode for better concurrency
    pub enable_wal: bool,
    /// Connection flags for opening database
    pub open_flags: OpenFlags,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("database.db"),
            min_connections: 2,
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            maintenance_interval: Duration::from_secs(60), // 1 minute
            enable_wal: true,
            open_flags: OpenFlags::SQLITE_OPEN_READ_WRITE 
                | OpenFlags::SQLITE_OPEN_CREATE 
                | OpenFlags::SQLITE_OPEN_URI 
                | OpenFlags::SQLITE_OPEN_NO_MUTEX, // Disable SQLite's mutex since we handle concurrency
        }
    }
}

impl PoolConfig {
    /// Create a high-performance configuration for production workloads
    pub fn high_performance(db_path: PathBuf) -> Self {
        Self {
            db_path,
            min_connections: 5,
            max_connections: 50,
            connection_timeout: Duration::from_secs(1),
            idle_timeout: Duration::from_secs(120),
            maintenance_interval: Duration::from_secs(30),
            enable_wal: true,
            open_flags: OpenFlags::SQLITE_OPEN_READ_WRITE 
                | OpenFlags::SQLITE_OPEN_CREATE 
                | OpenFlags::SQLITE_OPEN_URI 
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        }
    }

    /// Create a memory-efficient configuration for resource-constrained environments
    pub fn memory_efficient(db_path: PathBuf) -> Self {
        Self {
            db_path,
            min_connections: 1,
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            maintenance_interval: Duration::from_secs(120),
            enable_wal: true,
            open_flags: OpenFlags::SQLITE_OPEN_READ_WRITE 
                | OpenFlags::SQLITE_OPEN_CREATE 
                | OpenFlags::SQLITE_OPEN_URI 
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        }
    }
}

/// Statistics about a database connection
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub created_at: Instant,
    pub last_used: Instant,
    pub age: Duration,
    pub idle_time: Duration,
}

/// Statistics about the connection pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub total_connections_created: u64,
    pub total_connections_destroyed: u64,
    pub current_active_connections: usize,
    pub current_idle_connections: usize,
    pub total_requests: u64,
    pub total_wait_time: Duration,
    pub max_wait_time: Duration,
    pub connection_errors: u64,
}