use anyhow::Result;
use lsp_bridge::core::{DatabasePool, DatabasePoolBuilder, PoolConfig};
use rusqlite::params;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔗 LSP Bridge Database Connection Pooling Example");
    println!("==================================================");

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a temporary database for demonstration
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("example.db");

    // Example 1: Basic connection pool setup
    println!("\n📊 Creating connection pool...");
    let pool = DatabasePoolBuilder::new(&db_path)
        .min_connections(3)
        .max_connections(10)
        .connection_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .enable_wal(true)
        .build()
        .await?;

    println!("✅ Connection pool created successfully!");

    // Example 2: Initialize database schema
    println!("\n🏗️ Initializing database schema...");
    pool.with_connection(|conn| {
        conn.execute_batch(
            r#"
            CREATE TABLE diagnostic_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                severity TEXT NOT NULL,
                message TEXT NOT NULL,
                source TEXT NOT NULL
            );
            
            CREATE INDEX idx_events_timestamp ON diagnostic_events(timestamp);
            CREATE INDEX idx_events_file_path ON diagnostic_events(file_path);
            "#
        )?;
        println!("✅ Database schema initialized!");
        Ok(())
    }).await?;

    // Example 3: Demonstrate concurrent write performance
    println!("\n⚡ Testing concurrent write performance...");
    let start_time = Instant::now();
    
    // Simulate multiple concurrent diagnostic events
    let handles: Vec<_> = (0..100).map(|i| {
        let pool = pool.clone();
        tokio::spawn(async move {
            pool.with_connection(move |conn| {
                let file_path = format!("src/file_{}.rs", i % 10);
                let severity = if i % 4 == 0 { "error" } else { "warning" };
                let message = format!("Diagnostic message #{}", i);
                
                conn.execute(
                    "INSERT INTO diagnostic_events (timestamp, file_path, severity, message, source) 
                     VALUES (?, ?, ?, ?, ?)",
                    params![
                        chrono::Utc::now().timestamp(),
                        file_path,
                        severity,
                        message,
                        "rust-analyzer"
                    ]
                )?;
                Ok(())
            }).await
        })
    }).collect();

    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await??;
    }

    let write_duration = start_time.elapsed();
    println!("✅ 100 concurrent writes completed in {:?}", write_duration);

    // Example 4: Demonstrate read performance with connection pooling
    println!("\n📖 Testing concurrent read performance...");
    let start_time = Instant::now();

    let read_handles: Vec<_> = (0..50).map(|_| {
        let pool = pool.clone();
        tokio::spawn(async move {
            pool.with_read_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT COUNT(*) as count, severity 
                     FROM diagnostic_events 
                     GROUP BY severity"
                )?;
                
                let results: Vec<(i64, String)> = stmt.query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?.collect::<rusqlite::Result<Vec<_>>>()?;
                
                Ok(results)
            }).await
        })
    }).collect();

    // Collect all read results
    let mut all_results = Vec::new();
    for handle in read_handles {
        let results = handle.await??;
        all_results.push(results);
    }

    let read_duration = start_time.elapsed();
    println!("✅ 50 concurrent reads completed in {:?}", read_duration);
    
    if let Some(first_result) = all_results.first() {
        println!("📈 Diagnostic counts by severity:");
        for (count, severity) in first_result {
            println!("   {} {}: {}", 
                if severity == "error" { "🔴" } else { "⚠️" }, 
                severity, 
                count
            );
        }
    }

    // Example 5: Pool statistics and monitoring
    println!("\n📊 Connection pool statistics:");
    let stats = pool.stats().await;
    println!("   • Total connections created: {}", stats.total_connections_created);
    println!("   • Current active connections: {}", stats.current_active_connections);
    println!("   • Current idle connections: {}", stats.current_idle_connections);
    println!("   • Total requests processed: {}", stats.total_requests);
    println!("   • Average wait time: {:?}", stats.total_wait_time.checked_div(stats.total_requests.max(1) as u32).unwrap_or_default());
    println!("   • Maximum wait time: {:?}", stats.max_wait_time);

    // Example 6: Demonstrate different pool configurations
    println!("\n🎛️ Testing different pool configurations...");
    
    // High-performance configuration
    let high_perf_db = temp_dir.path().join("high_perf.db");
    let high_perf_pool = DatabasePoolBuilder::new(&high_perf_db)
        .min_connections(5)
        .max_connections(50)
        .connection_timeout(Duration::from_secs(1))
        .idle_timeout(Duration::from_secs(120))
        .build()
        .await?;
    
    println!("✅ High-performance pool created (5-50 connections)");

    // Memory-efficient configuration
    let memory_eff_db = temp_dir.path().join("memory_eff.db");
    let memory_eff_pool = DatabasePoolBuilder::new(&memory_eff_db)
        .min_connections(1)
        .max_connections(3)
        .connection_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .build()
        .await?;
    
    println!("✅ Memory-efficient pool created (1-3 connections)");

    // Example 7: Error handling and connection recovery
    println!("\n🛡️ Testing error handling...");
    
    // Simulate a scenario where we try to exceed connection limits
    let limited_pool = DatabasePoolBuilder::new(temp_dir.path().join("limited.db"))
        .min_connections(1)
        .max_connections(1)
        .connection_timeout(Duration::from_millis(100))
        .build()
        .await?;

    // Hold the only connection
    let _held_conn = limited_pool.get_connection().await?;
    
    // Try to get another connection (should timeout)
    match limited_pool.get_connection().await {
        Ok(_) => println!("⚠️ Unexpected success - should have timed out"),
        Err(e) => println!("✅ Properly handled connection timeout: {}", e),
    }

    // Example 8: Integration with HistoryStorage-style operations
    println!("\n🏛️ Demonstrating integration with diagnostic history...");
    
    // Simulate the pattern used in HistoryStorage
    demonstrate_history_pattern(&pool).await?;

    println!("\n🎉 Database connection pooling examples completed!");
    println!("Key benefits demonstrated:");
    println!("   • Concurrent database access without blocking");
    println!("   • Automatic connection lifecycle management");
    println!("   • Configurable performance vs memory tradeoffs");
    println!("   • Built-in error handling and timeouts");
    println!("   • Comprehensive monitoring and statistics");
    println!("   • WAL mode for better concurrent read/write performance");

    Ok(())
}

/// Demonstrates the pattern used in HistoryStorage with connection pooling
async fn demonstrate_history_pattern(pool: &Arc<DatabasePool>) -> Result<()> {
    // Create a diagnostics history table similar to HistoryStorage
    pool.with_connection(|conn| {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS diagnostic_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                diagnostics_json TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_snapshots_file_path ON diagnostic_snapshots(file_path);
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON diagnostic_snapshots(timestamp);
            "#
        )?;
        Ok(())
    }).await?;

    println!("📝 Created diagnostic snapshots table");

    // Simulate recording multiple diagnostic snapshots concurrently
    let snapshot_handles: Vec<_> = (0..20).map(|i| {
        let pool = pool.clone();
        tokio::spawn(async move {
            let file_path = format!("src/component_{}.rs", i % 5);
            let file_hash = format!("hash_{:x}", i * 123);
            let diagnostics = serde_json::json!([
                {
                    "severity": if i % 3 == 0 { "error" } else { "warning" },
                    "message": format!("Issue #{} in file", i),
                    "range": {"start": {"line": i % 100, "character": 0}, "end": {"line": i % 100, "character": 10}}
                }
            ]);

            pool.with_connection(move |conn| {
                let id: i64 = conn.query_row(
                    "INSERT INTO diagnostic_snapshots (timestamp, file_path, file_hash, diagnostics_json) 
                     VALUES (?, ?, ?, ?) RETURNING id",
                    params![
                        chrono::Utc::now().timestamp(),
                        file_path,
                        file_hash,
                        diagnostics.to_string()
                    ],
                    |row| row.get(0)
                )?;
                Ok(id)
            }).await
        })
    }).collect();

    // Wait for all snapshots to be recorded
    let mut snapshot_ids = Vec::new();
    for handle in snapshot_handles {
        let id = handle.await??;
        snapshot_ids.push(id);
    }

    println!("✅ Recorded {} diagnostic snapshots concurrently", snapshot_ids.len());

    // Demonstrate concurrent reads of snapshots
    let read_handles: Vec<_> = (0..10).map(|_| {
        let pool = pool.clone();
        tokio::spawn(async move {
            pool.with_read_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT file_path, COUNT(*) as snapshot_count 
                     FROM diagnostic_snapshots 
                     GROUP BY file_path 
                     ORDER BY snapshot_count DESC"
                )?;
                
                let results: Vec<(String, i64)> = stmt.query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?.collect::<rusqlite::Result<Vec<_>>>()?;
                
                Ok(results)
            }).await
        })
    }).collect();

    // Collect results from concurrent reads
    for handle in read_handles {
        let _results = handle.await??;
    }

    println!("✅ Performed concurrent snapshot analysis");

    Ok(())
}