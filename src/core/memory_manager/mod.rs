//! Memory management module for LSPbridge
//!
//! This module provides a comprehensive memory management system with bounded caches,
//! configurable eviction policies, memory monitoring, and optimization capabilities.
//!
//! ## Architecture
//!
//! The memory manager is organized into focused modules:
//! - `types`: Core data structures and configuration types
//! - `eviction`: Various eviction strategies (LRU, LFU, size/age-weighted, adaptive)
//! - `cache`: The main bounded cache implementation
//! - `monitor`: Memory monitoring, alerting, and optimization
//!
//! ## Features
//!
//! - **Bounded Memory Usage**: Enforce hard limits on memory consumption
//! - **Multiple Eviction Policies**: LRU, LFU, size-weighted, age-weighted, and adaptive
//! - **Real-time Monitoring**: Track memory usage, cache performance, and health
//! - **Automatic Optimization**: Detect and respond to performance degradation
//! - **Configurable Thresholds**: Set water marks for proactive memory management
//!
//! ## Usage Example
//!
//! ```rust
//! use lsp_bridge::core::memory_manager::{BoundedCacheBuilder, EvictionPolicy};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a cache with 256MB limit and LRU eviction
//! let cache = BoundedCacheBuilder::<String, Vec<u8>>::new()
//!     .max_memory_mb(256)
//!     .max_entries(10000)
//!     .eviction_policy(EvictionPolicy::LRU)
//!     .build();
//!
//! // Store data
//! cache.put("key".to_string(), vec![0; 1024], 1024).await?;
//!
//! // Retrieve data
//! if let Some(data) = cache.get(&"key".to_string()).await {
//!     println!("Found data: {} bytes", data.len());
//! }
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod eviction;
pub mod monitor;
pub mod types;

// Re-export commonly used types
pub use cache::{BoundedCache, BoundedCacheBuilder};
pub use eviction::{EvictionManager, EvictionStrategy};
pub use monitor::{AlertHandler, MemoryEvent, MemoryMonitor, MemoryOptimizer, MemoryThreshold, MonitorConfig};
pub use types::{
    CacheEntry, CacheStatistics, EvictionPolicy, MemoryConfig, MemoryHealthStatus, MemoryReport,
};

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

/// Global memory manager for coordinating multiple caches
pub struct MemoryManager {
    /// Global memory configuration
    global_config: MemoryConfig,
    
    /// Memory monitor
    monitor: Arc<MemoryMonitor>,
    
    /// Alert handler
    alert_handler: Arc<AlertHandler>,
    
    /// Memory optimizer
    optimizer: Arc<MemoryOptimizer>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(global_config: MemoryConfig) -> Self {
        let monitor_config = MonitorConfig {
            check_interval: global_config.monitoring_interval,
            ..Default::default()
        };

        Self {
            global_config,
            monitor: Arc::new(MemoryMonitor::new(monitor_config)),
            alert_handler: Arc::new(AlertHandler::new()),
            optimizer: Arc::new(MemoryOptimizer::new()),
        }
    }

    /// Create a new bounded cache managed by this memory manager
    pub fn create_cache<K, V>(&self, name: &str, config: MemoryConfig) -> Arc<BoundedCache<K, V>>
    where
        K: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        info!("Creating managed cache '{}' with {}MB limit", name, config.max_memory_mb);
        Arc::new(BoundedCache::new(config))
    }

    /// Start global memory monitoring
    pub async fn start_monitoring<F, Fut>(&self, get_global_report: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = MemoryReport> + Send,
    {
        self.monitor.start(get_global_report).await?;
        info!("Global memory monitoring started");
        Ok(())
    }

    /// Stop memory monitoring
    pub async fn stop_monitoring(&self) -> Result<()> {
        self.monitor.stop().await
    }

    /// Get the memory monitor
    pub fn monitor(&self) -> &Arc<MemoryMonitor> {
        &self.monitor
    }

    /// Get the alert handler
    pub fn alert_handler(&self) -> &Arc<AlertHandler> {
        &self.alert_handler
    }

    /// Get the memory optimizer
    pub fn optimizer(&self) -> &Arc<MemoryOptimizer> {
        &self.optimizer
    }

    /// Get global configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.global_config
    }
}

/// Convenience functions for creating caches with common configurations

/// Create a small cache (16MB, 1000 entries)
pub fn create_small_cache<K, V>() -> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    BoundedCacheBuilder::new()
        .max_memory_mb(16)
        .max_entries(1000)
        .build()
}

/// Create a medium cache (64MB, 10000 entries)
pub fn create_medium_cache<K, V>() -> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    BoundedCacheBuilder::new()
        .max_memory_mb(64)
        .max_entries(10000)
        .build()
}

/// Create a large cache (256MB, 50000 entries)
pub fn create_large_cache<K, V>() -> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    BoundedCacheBuilder::new()
        .max_memory_mb(256)
        .max_entries(50000)
        .build()
}

/// Create a cache optimized for small entries
pub fn create_small_entry_cache<K, V>() -> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    BoundedCacheBuilder::new()
        .max_memory_mb(128)
        .max_entries(100000)
        .eviction_policy(EvictionPolicy::LFU)
        .build()
}

/// Create a cache optimized for large entries
pub fn create_large_entry_cache<K, V>() -> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    BoundedCacheBuilder::new()
        .max_memory_mb(512)
        .max_entries(1000)
        .eviction_policy(EvictionPolicy::SizeWeighted)
        .build()
}

/// Utilities for memory management
pub mod utils {
    use super::*;

    /// Format bytes as human-readable string
    pub fn format_bytes(bytes: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    /// Calculate memory overhead for a collection
    pub fn estimate_collection_overhead<T>(capacity: usize) -> usize {
        // Rough estimation of collection overhead
        let element_size = std::mem::size_of::<T>();
        let pointer_size = std::mem::size_of::<usize>();
        
        // Base struct size + capacity * element size + some metadata
        pointer_size * 3 + capacity * element_size + 64
    }

    /// Check if system has sufficient memory for allocation
    pub fn check_memory_available(required_mb: usize) -> bool {
        // This is a simplified check - in production you'd use system APIs
        required_mb < 1024 // Assume we have less than 1GB available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        assert_eq!(manager.config().max_memory_mb, 256);
    }

    #[tokio::test]
    async fn test_convenience_cache_creation() {
        let small_cache: BoundedCache<String, String> = create_small_cache();
        assert_eq!(small_cache.config().max_memory_mb, 16);
        assert_eq!(small_cache.config().max_entries, 1000);

        let large_cache: BoundedCache<String, Vec<u8>> = create_large_cache();
        assert_eq!(large_cache.config().max_memory_mb, 256);
        assert_eq!(large_cache.config().max_entries, 50000);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(utils::format_bytes(100), "100 B");
        assert_eq!(utils::format_bytes(1024), "1.00 KB");
        assert_eq!(utils::format_bytes(1536), "1.50 KB");
        assert_eq!(utils::format_bytes(1048576), "1.00 MB");
        assert_eq!(utils::format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_estimate_overhead() {
        let overhead = utils::estimate_collection_overhead::<u64>(100);
        assert!(overhead > 800); // At least 8 bytes * 100 elements
        assert!(overhead < 2000); // But not too much overhead
    }
}