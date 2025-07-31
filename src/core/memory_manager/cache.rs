//! Bounded cache implementation with memory management
//!
//! This module provides the main cache implementation with configurable
//! memory limits, eviction policies, and performance monitoring.

use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::eviction::EvictionManager;
use super::types::{CacheEntry, CacheStatistics, MemoryConfig, MemoryReport};

/// A bounded cache with memory management and eviction policies
pub struct BoundedCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Cache entries
    entries: RwLock<HashMap<K, CacheEntry<V>>>,
    
    /// Access order tracking for LRU
    access_order: RwLock<VecDeque<K>>,
    
    /// Configuration
    config: MemoryConfig,
    
    /// Current size in bytes
    current_size: AtomicUsize,
    
    /// Current number of entries
    current_entries: AtomicUsize,
    
    /// Cache statistics
    statistics: RwLock<CacheStatistics>,
    
    /// Eviction manager
    eviction_manager: EvictionManager,
}

impl<K, V> BoundedCache<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    /// Create a new bounded cache with the given configuration
    pub fn new(config: MemoryConfig) -> Self {
        // Validate configuration
        if let Err(e) = config.validate() {
            panic!("Invalid memory configuration: {}", e);
        }

        let eviction_manager = EvictionManager::new(config.clone());

        Self {
            entries: RwLock::new(HashMap::with_capacity(config.max_entries / 4)),
            access_order: RwLock::new(VecDeque::with_capacity(config.max_entries / 4)),
            config,
            current_size: AtomicUsize::new(0),
            current_entries: AtomicUsize::new(0),
            statistics: RwLock::new(CacheStatistics::default()),
            eviction_manager,
        }
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        if let Some(entry) = entries.get_mut(key) {
            entry.update_access();

            // Update LRU order
            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }
            access_order.push_back(key.clone());

            // Record hit
            self.record_hit().await;

            Some(entry.data.clone())
        } else {
            // Record miss
            self.record_miss().await;
            None
        }
    }

    /// Get a value from the cache without updating access statistics
    pub async fn peek(&self, key: &K) -> Option<V> {
        let entries = self.entries.read().await;
        entries.get(key).map(|entry| entry.data.clone())
    }

    /// Put a value into the cache
    pub async fn put(&self, key: K, value: V, size_bytes: usize) -> Result<()> {
        // Check if we need to evict before inserting
        self.eviction_manager
            .evict_if_needed(
                &self.entries,
                &self.access_order,
                &self.current_size,
                &self.current_entries,
                &self.statistics,
                size_bytes,
            )
            .await?;

        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        // Remove existing entry if present
        if let Some(old_entry) = entries.remove(&key) {
            self.current_size.fetch_sub(old_entry.size_bytes, Ordering::SeqCst);
            self.current_entries.fetch_sub(1, Ordering::SeqCst);

            if let Some(pos) = access_order.iter().position(|k| k == &key) {
                access_order.remove(pos);
            }
        }

        // Insert new entry
        let entry = CacheEntry::new(value, size_bytes);
        entries.insert(key.clone(), entry);
        access_order.push_back(key);

        self.current_size.fetch_add(size_bytes, Ordering::SeqCst);
        self.current_entries.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// Put a value into the cache if it doesn't already exist
    pub async fn put_if_absent(&self, key: K, value: V, size_bytes: usize) -> Result<bool> {
        let entries = self.entries.read().await;
        if entries.contains_key(&key) {
            return Ok(false);
        }
        drop(entries);

        self.put(key, value, size_bytes).await?;
        Ok(true)
    }

    /// Update a value in the cache
    pub async fn update<F>(&self, key: &K, updater: F) -> Result<Option<V>>
    where
        F: FnOnce(&V) -> V,
    {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.get_mut(key) {
            let new_value = updater(&entry.data);
            entry.data = new_value.clone();
            entry.update_access();
            
            // Update access order
            let mut access_order = self.access_order.write().await;
            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }
            access_order.push_back(key.clone());
            
            Ok(Some(new_value))
        } else {
            Ok(None)
        }
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        if let Some(entry) = entries.remove(key) {
            self.current_size.fetch_sub(entry.size_bytes, Ordering::SeqCst);
            self.current_entries.fetch_sub(1, Ordering::SeqCst);

            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }

            Some(entry.data)
        } else {
            None
        }
    }

    /// Remove all entries matching a predicate
    pub async fn remove_if<F>(&self, predicate: F) -> Vec<(K, V)>
    where
        F: Fn(&K, &V) -> bool,
    {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;
        let mut removed = Vec::new();

        let keys_to_remove: Vec<K> = entries
            .iter()
            .filter_map(|(k, entry)| {
                if predicate(k, &entry.data) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            if let Some(entry) = entries.remove(&key) {
                self.current_size.fetch_sub(entry.size_bytes, Ordering::SeqCst);
                self.current_entries.fetch_sub(1, Ordering::SeqCst);

                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }

                removed.push((key, entry.data));
            }
        }

        removed
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        entries.clear();
        access_order.clear();
        self.current_size.store(0, Ordering::SeqCst);
        self.current_entries.store(0, Ordering::SeqCst);

        info!("Cache cleared completely");
    }

    /// Get the current size in bytes
    pub fn size_bytes(&self) -> usize {
        self.current_size.load(Ordering::SeqCst)
    }

    /// Get the current number of entries
    pub fn entry_count(&self) -> usize {
        self.current_entries.load(Ordering::SeqCst)
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entry_count() == 0
    }

    /// Get current memory pressure (0.0 - 1.0)
    pub fn memory_pressure(&self) -> f64 {
        let current_mb = self.size_bytes() as f64 / (1024.0 * 1024.0);
        current_mb / self.config.max_memory_mb as f64
    }

    /// Check if the cache contains a key
    pub async fn contains_key(&self, key: &K) -> bool {
        let entries = self.entries.read().await;
        entries.contains_key(key)
    }

    /// Get all keys in the cache
    pub async fn keys(&self) -> Vec<K> {
        let entries = self.entries.read().await;
        entries.keys().cloned().collect()
    }

    /// Get cache statistics
    pub async fn get_statistics(&self) -> CacheStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }

    /// Reset cache statistics
    pub async fn reset_statistics(&self) {
        let mut stats = self.statistics.write().await;
        stats.reset();
    }

    /// Get a memory report
    pub async fn get_memory_report(&self) -> MemoryReport {
        let stats = self.get_statistics().await;
        let size_bytes = self.size_bytes();
        let entry_count = self.entry_count();

        MemoryReport {
            total_size_bytes: size_bytes,
            total_entries: entry_count,
            max_size_bytes: self.config.max_memory_mb * 1024 * 1024,
            max_entries: self.config.max_entries,
            memory_utilization: size_bytes as f64 / (self.config.max_memory_mb * 1024 * 1024) as f64,
            entry_utilization: entry_count as f64 / self.config.max_entries as f64,
            hit_rate: stats.hit_rate(),
            eviction_rate: if stats.hits + stats.misses > 0 {
                stats.evictions as f64 / (stats.hits + stats.misses) as f64
            } else {
                0.0
            },
            eviction_policy: self.config.eviction_policy,
        }
    }

    /// Optimize the cache by removing stale entries
    pub async fn optimize(&self) -> Result<()> {
        let start = std::time::Instant::now();
        let memory_pressure = self.memory_pressure();
        let stats = self.get_statistics().await;

        if memory_pressure > 0.7 || stats.hit_rate() < 0.6 {
            warn!(
                "Cache performance degraded (pressure: {:.2}, hit_rate: {:.2}), running optimization",
                memory_pressure,
                stats.hit_rate()
            );

            let target_size = (self.config.max_memory_mb * 1024 * 1024) as f64 * 0.5;
            let target_count = self.config.max_entries as f64 * 0.5;

            let evicted = self.eviction_manager
                .evict_if_needed(
                    &self.entries,
                    &self.access_order,
                    &self.current_size,
                    &self.current_entries,
                    &self.statistics,
                    0, // No new item, just optimizing
                )
                .await?;

            info!(
                "Cache optimization completed: evicted {} entries in {:?}",
                evicted,
                start.elapsed()
            );
        }

        Ok(())
    }

    /// Record a cache hit
    async fn record_hit(&self) {
        let mut stats = self.statistics.write().await;
        stats.hits += 1;
    }

    /// Record a cache miss
    async fn record_miss(&self) {
        let mut stats = self.statistics.write().await;
        stats.misses += 1;
    }

    /// Get cache configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
}

/// Builder for creating a bounded cache with custom configuration
pub struct BoundedCacheBuilder<K, V> {
    config: MemoryConfig,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> BoundedCacheBuilder<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    /// Create a new cache builder
    pub fn new() -> Self {
        Self {
            config: MemoryConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set maximum memory in MB
    pub fn max_memory_mb(mut self, mb: usize) -> Self {
        self.config.max_memory_mb = mb;
        self
    }

    /// Set maximum number of entries
    pub fn max_entries(mut self, entries: usize) -> Self {
        self.config.max_entries = entries;
        self
    }

    /// Set eviction policy
    pub fn eviction_policy(mut self, policy: super::types::EvictionPolicy) -> Self {
        self.config.eviction_policy = policy;
        self
    }

    /// Set water marks
    pub fn water_marks(mut self, high: f64, low: f64) -> Self {
        self.config.high_water_mark = high;
        self.config.low_water_mark = low;
        self
    }

    /// Build the cache
    pub fn build(self) -> BoundedCache<K, V> {
        BoundedCache::new(self.config)
    }
}

impl<K, V> Default for BoundedCacheBuilder<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug + 'static,
    V: Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bounded_cache_basic_operations() {
        let cache = BoundedCacheBuilder::<String, String>::new()
            .max_memory_mb(1)
            .max_entries(10)
            .build();

        // Test put and get
        cache.put("key1".to_string(), "value1".to_string(), 100).await.unwrap();
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));
        assert_eq!(cache.get(&"nonexistent".to_string()).await, None);

        // Test statistics
        let stats = cache.get_statistics().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = BoundedCacheBuilder::<String, String>::new()
            .max_memory_mb(1)
            .max_entries(2)
            .eviction_policy(super::super::types::EvictionPolicy::LRU)
            .water_marks(1.0, 0.5)
            .build();

        // Fill cache
        cache.put("key1".to_string(), "value1".to_string(), 100).await.unwrap();
        cache.put("key2".to_string(), "value2".to_string(), 100).await.unwrap();

        // This should trigger eviction
        cache.put("key3".to_string(), "value3".to_string(), 100).await.unwrap();

        // Check that we have exactly 2 entries
        assert_eq!(cache.entry_count(), 2);
    }

    #[tokio::test]
    async fn test_cache_update() {
        let cache = BoundedCacheBuilder::<String, i32>::new()
            .max_entries(10)
            .build();

        cache.put("counter".to_string(), 0, 8).await.unwrap();
        
        let updated = cache.update(&"counter".to_string(), |v| v + 1).await.unwrap();
        assert_eq!(updated, Some(1));

        let value = cache.get(&"counter".to_string()).await;
        assert_eq!(value, Some(1));
    }

    #[tokio::test]
    async fn test_cache_remove_if() {
        let cache = BoundedCacheBuilder::<String, i32>::new()
            .max_entries(10)
            .build();

        for i in 0..5 {
            cache.put(format!("key{}", i), i, 8).await.unwrap();
        }

        // Remove all even values
        let removed = cache.remove_if(|_, v| v % 2 == 0).await;
        assert_eq!(removed.len(), 3); // 0, 2, 4

        assert_eq!(cache.entry_count(), 2); // 1, 3 remain
    }
}