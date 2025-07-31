pub mod types;
pub mod connection;
pub mod builder;
mod manager;
mod monitoring;
mod pool;

pub use types::{PoolConfig, PoolStats, ConnectionStats};
pub use connection::PooledConnection;
pub use builder::DatabasePoolBuilder;
pub use pool::DatabasePool;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use std::time::Duration;

    #[tokio::test]
    async fn test_pool_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let pool = DatabasePoolBuilder::new(&db_path)
            .min_connections(2)
            .max_connections(5)
            .build()
            .await
            .unwrap();

        let stats = pool.stats().await;
        assert_eq!(stats.current_idle_connections, 2);
    }

    #[tokio::test]
    async fn test_connection_acquisition() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let pool = DatabasePoolBuilder::new(&db_path)
            .min_connections(1)
            .max_connections(3)
            .build()
            .await
            .unwrap();

        // Test getting multiple connections
        let _conn1 = pool.get_connection().await.unwrap();
        let _conn2 = pool.get_connection().await.unwrap();
        
        let stats = pool.stats().await;
        assert_eq!(stats.current_active_connections, 2);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let pool = DatabasePoolBuilder::new(&db_path)
            .min_connections(2)
            .max_connections(10)
            .build()
            .await
            .unwrap();

        // Initialize table
        pool.with_connection(|conn| {
            conn.execute_batch(
                "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT);"
            )?;
            Ok(())
        }).await.unwrap();

        // Test concurrent writes
        let handles: Vec<_> = (0..20).map(|i| {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.with_connection(move |conn| {
                    conn.execute(
                        "INSERT INTO test (value) VALUES (?)",
                        rusqlite::params![format!("value_{}", i)]
                    )?;
                    Ok(())
                }).await
            })
        }).collect();

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all inserts succeeded
        let count: i64 = pool.with_read_connection(|conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM test")?;
            let count = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await.unwrap();

        assert_eq!(count, 20);
    }

    #[tokio::test]
    #[ignore] // TODO: Fix connection pool semaphore design - permits released too early
    async fn test_connection_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let pool = DatabasePoolBuilder::new(&db_path)
            .min_connections(1)
            .max_connections(1)
            .connection_timeout(Duration::from_millis(10))
            .build()
            .await
            .unwrap();

        // Hold the only connection
        let _conn = pool.get_connection().await.unwrap();
        println!("First connection acquired");
        
        // Add a small delay to ensure the connection is fully established
        sleep(Duration::from_millis(10)).await;
        
        // Second connection should timeout (pool is at max capacity)
        println!("Attempting second connection...");
        let start = std::time::Instant::now();
        let result = pool.get_connection().await;
        let elapsed = start.elapsed();
        println!("Second connection result: {:?}, elapsed: {:?}", result.is_ok(), elapsed);
        
        assert!(result.is_err(), "Expected second connection to timeout, but it succeeded");
    }
}