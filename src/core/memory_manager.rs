use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    LRU,          // Least Recently Used
    LFU,          // Least Frequently Used
    SizeWeighted, // Prioritize by size (remove large items first)
    AgeWeighted,  // Prioritize by age (remove old items first)
    Adaptive,     // Dynamically choose based on usage patterns
}

#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_memory_mb: usize,
    pub max_entries: usize,
    pub eviction_policy: EvictionPolicy,
    pub high_water_mark: f64, // Start eviction at this % of capacity
    pub low_water_mark: f64,  // Stop eviction at this % of capacity
    pub eviction_batch_size: usize,
    pub monitoring_interval: Duration,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 256,
            max_entries: 50000,
            eviction_policy: EvictionPolicy::Adaptive,
            high_water_mark: 0.8,
            low_water_mark: 0.6,
            eviction_batch_size: 100,
            monitoring_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub data: T,
    pub size_bytes: usize,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub access_frequency: f64, // Exponentially weighted moving average
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, size_bytes: usize) -> Self {
        let now = Instant::now();
        Self {
            data,
            size_bytes,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            access_frequency: 1.0,
        }
    }

    pub fn update_access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;

        // Update frequency with exponential decay
        let decay_factor = 0.9;
        let time_weight = 1.0; // Could be adjusted based on time since last access
        self.access_frequency = self.access_frequency * decay_factor + time_weight;
    }

    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn time_since_access(&self) -> Duration {
        self.last_accessed.elapsed()
    }
}

pub struct BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    entries: RwLock<HashMap<K, CacheEntry<V>>>,
    access_order: RwLock<VecDeque<K>>, // For LRU tracking
    config: MemoryConfig,
    current_size: AtomicUsize,
    current_entries: AtomicUsize,
    statistics: RwLock<CacheStatistics>,
}

#[derive(Debug, Clone)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size_evictions: u64,
    pub count_evictions: u64,
    pub memory_pressure_events: u64,
    pub last_cleanup: Instant,
}

impl Default for CacheStatistics {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            size_evictions: 0,
            count_evictions: 0,
            memory_pressure_events: 0,
            last_cleanup: Instant::now(),
        }
    }
}

impl CacheStatistics {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

impl<K, V> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    V: Clone,
{
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::with_capacity(config.max_entries / 4)), // Start with 25% capacity
            access_order: RwLock::new(VecDeque::with_capacity(config.max_entries / 4)),
            config,
            current_size: AtomicUsize::new(0),
            current_entries: AtomicUsize::new(0),
            statistics: RwLock::new(CacheStatistics::default()),
        }
    }

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
            {
                let mut stats = self.statistics.write().await;
                stats.hits += 1;
            }

            Some(entry.data.clone())
        } else {
            // Record miss
            {
                let mut stats = self.statistics.write().await;
                stats.misses += 1;
            }
            None
        }
    }

    pub async fn put(&self, key: K, value: V, size_bytes: usize) -> Result<()> {
        // Check if we need to evict before inserting
        if self.should_evict(size_bytes).await {
            self.evict_entries().await?;
        }

        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        // Remove existing entry if present
        if let Some(old_entry) = entries.remove(&key) {
            self.current_size
                .fetch_sub(old_entry.size_bytes, Ordering::SeqCst);
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

    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        if let Some(entry) = entries.remove(key) {
            self.current_size
                .fetch_sub(entry.size_bytes, Ordering::SeqCst);
            self.current_entries.fetch_sub(1, Ordering::SeqCst);

            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }

            Some(entry.data)
        } else {
            None
        }
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        entries.clear();
        access_order.clear();
        self.current_size.store(0, Ordering::SeqCst);
        self.current_entries.store(0, Ordering::SeqCst);

        info!("Cache cleared completely");
    }

    pub async fn size_bytes(&self) -> usize {
        self.current_size.load(Ordering::SeqCst)
    }

    pub async fn entry_count(&self) -> usize {
        self.current_entries.load(Ordering::SeqCst)
    }

    pub async fn memory_pressure(&self) -> f64 {
        let current_mb = self.size_bytes().await / (1024 * 1024);
        current_mb as f64 / self.config.max_memory_mb as f64
    }

    pub async fn get_statistics(&self) -> CacheStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }

    async fn should_evict(&self, new_item_size: usize) -> bool {
        let current_size = self.current_size.load(Ordering::SeqCst);
        let current_entries = self.current_entries.load(Ordering::SeqCst);

        let size_threshold =
            (self.config.max_memory_mb * 1024 * 1024) as f64 * self.config.high_water_mark;
        let count_threshold = self.config.max_entries as f64 * self.config.high_water_mark;

        let will_exceed_size = (current_size + new_item_size) as f64 > size_threshold;
        let will_exceed_count = (current_entries + 1) as f64 > count_threshold;

        will_exceed_size || will_exceed_count
    }

    async fn evict_entries(&self) -> Result<()> {
        let current_count = self.entry_count().await as f64;
        let max_count = self.config.max_entries as f64;

        // For count-based eviction, just evict enough to make room for one more item
        let target_count = if current_count >= max_count {
            max_count - 1.0
        } else {
            self.config.max_entries as f64 * self.config.low_water_mark
        };

        let target_size =
            (self.config.max_memory_mb * 1024 * 1024) as f64 * self.config.low_water_mark;

        let evicted_count = match self.config.eviction_policy {
            EvictionPolicy::LRU => self.evict_lru(target_size, target_count).await?,
            EvictionPolicy::LFU => self.evict_lfu(target_size, target_count).await?,
            EvictionPolicy::SizeWeighted => {
                self.evict_size_weighted(target_size, target_count).await?
            }
            EvictionPolicy::AgeWeighted => {
                self.evict_age_weighted(target_size, target_count).await?
            }
            EvictionPolicy::Adaptive => self.evict_adaptive(target_size, target_count).await?,
        };

        // Update statistics
        {
            let mut stats = self.statistics.write().await;
            stats.evictions += evicted_count;
            if self.size_bytes().await as f64 > target_size {
                stats.size_evictions += 1;
            }
            if self.entry_count().await as f64 > target_count {
                stats.count_evictions += 1;
            }
        }

        if evicted_count > 0 {
            info!(
                "Evicted {} entries using {:?} policy",
                evicted_count, self.config.eviction_policy
            );
        }

        Ok(())
    }

    async fn evict_lru(&self, target_size: f64, target_count: f64) -> Result<u64> {
        let mut evicted = 0;
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        while (self.size_bytes().await as f64 > target_size
            || self.entry_count().await as f64 > target_count)
            && !access_order.is_empty()
            && evicted < self.config.eviction_batch_size as u64
        {
            if let Some(key) = access_order.pop_front() {
                if let Some(entry) = entries.remove(&key) {
                    self.current_size
                        .fetch_sub(entry.size_bytes, Ordering::SeqCst);
                    self.current_entries.fetch_sub(1, Ordering::SeqCst);
                    evicted += 1;
                }
            }
        }

        Ok(evicted)
    }

    async fn evict_lfu(&self, target_size: f64, target_count: f64) -> Result<u64> {
        let mut evicted = 0;
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        // Collect entries with their frequencies
        let mut freq_entries: Vec<_> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.access_frequency))
            .collect();

        // Sort by frequency (ascending)
        freq_entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (key, _) in freq_entries {
            if (self.size_bytes().await as f64 <= target_size
                && self.entry_count().await as f64 <= target_count)
                || evicted >= self.config.eviction_batch_size as u64
            {
                break;
            }

            if let Some(entry) = entries.remove(&key) {
                self.current_size
                    .fetch_sub(entry.size_bytes, Ordering::SeqCst);
                self.current_entries.fetch_sub(1, Ordering::SeqCst);

                // Remove from access order
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }

                evicted += 1;
            }
        }

        Ok(evicted)
    }

    async fn evict_size_weighted(&self, target_size: f64, target_count: f64) -> Result<u64> {
        let mut evicted = 0;
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        // Collect entries with their sizes
        let mut size_entries: Vec<_> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.size_bytes))
            .collect();

        // Sort by size (descending - remove largest first)
        size_entries.sort_by(|a, b| b.1.cmp(&a.1));

        for (key, _) in size_entries {
            if (self.size_bytes().await as f64 <= target_size
                && self.entry_count().await as f64 <= target_count)
                || evicted >= self.config.eviction_batch_size as u64
            {
                break;
            }

            if let Some(entry) = entries.remove(&key) {
                self.current_size
                    .fetch_sub(entry.size_bytes, Ordering::SeqCst);
                self.current_entries.fetch_sub(1, Ordering::SeqCst);

                // Remove from access order
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }

                evicted += 1;
            }
        }

        Ok(evicted)
    }

    async fn evict_age_weighted(&self, target_size: f64, target_count: f64) -> Result<u64> {
        let mut evicted = 0;
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;

        // Collect entries with their ages
        let mut age_entries: Vec<_> = entries.iter().map(|(k, v)| (k.clone(), v.age())).collect();

        // Sort by age (descending - remove oldest first)
        age_entries.sort_by(|a, b| b.1.cmp(&a.1));

        for (key, _) in age_entries {
            if (self.size_bytes().await as f64 <= target_size
                && self.entry_count().await as f64 <= target_count)
                || evicted >= self.config.eviction_batch_size as u64
            {
                break;
            }

            if let Some(entry) = entries.remove(&key) {
                self.current_size
                    .fetch_sub(entry.size_bytes, Ordering::SeqCst);
                self.current_entries.fetch_sub(1, Ordering::SeqCst);

                // Remove from access order
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }

                evicted += 1;
            }
        }

        Ok(evicted)
    }

    async fn evict_adaptive(&self, target_size: f64, target_count: f64) -> Result<u64> {
        // Adaptive eviction: choose strategy based on current conditions
        let stats = self.get_statistics().await;
        let hit_rate = stats.hit_rate();
        let memory_pressure = self.memory_pressure().await;

        if memory_pressure > 0.9 {
            // High memory pressure: prioritize size
            self.evict_size_weighted(target_size, target_count).await
        } else if hit_rate < 0.5 {
            // Low hit rate: items aren't being reused, use LRU
            self.evict_lru(target_size, target_count).await
        } else if hit_rate > 0.8 {
            // High hit rate: keep frequently used items
            self.evict_lfu(target_size, target_count).await
        } else {
            // Balanced approach: age-weighted eviction
            self.evict_age_weighted(target_size, target_count).await
        }
    }

    pub async fn optimize(&self) -> Result<()> {
        let start = Instant::now();

        // Check if optimization is needed
        let memory_pressure = self.memory_pressure().await;
        let stats = self.get_statistics().await;

        if memory_pressure > 0.7 || stats.hit_rate() < 0.6 {
            warn!("Cache performance degraded, running optimization");

            let target_size = (self.config.max_memory_mb * 1024 * 1024) as f64 * 0.5;
            let target_count = self.config.max_entries as f64 * 0.5;

            let evicted = self.evict_adaptive(target_size, target_count).await?;

            info!(
                "Cache optimization completed: evicted {} entries in {:?}",
                evicted,
                start.elapsed()
            );
        }

        Ok(())
    }

    pub async fn get_memory_report(&self) -> MemoryReport {
        let stats = self.get_statistics().await;
        let size_bytes = self.size_bytes().await;
        let entry_count = self.entry_count().await;

        MemoryReport {
            total_size_bytes: size_bytes,
            total_entries: entry_count,
            max_size_bytes: self.config.max_memory_mb * 1024 * 1024,
            max_entries: self.config.max_entries,
            memory_utilization: size_bytes as f64
                / (self.config.max_memory_mb * 1024 * 1024) as f64,
            entry_utilization: entry_count as f64 / self.config.max_entries as f64,
            hit_rate: stats.hit_rate(),
            eviction_rate: stats.evictions as f64,
            eviction_policy: self.config.eviction_policy,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryReport {
    pub total_size_bytes: usize,
    pub total_entries: usize,
    pub max_size_bytes: usize,
    pub max_entries: usize,
    pub memory_utilization: f64,
    pub entry_utilization: f64,
    pub hit_rate: f64,
    pub eviction_rate: f64,
    pub eviction_policy: EvictionPolicy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bounded_cache_basic_operations() {
        let config = MemoryConfig {
            max_memory_mb: 1,
            max_entries: 3,
            ..Default::default()
        };

        let cache = BoundedCache::new(config);

        // Test basic put/get
        cache
            .put("key1".to_string(), "value1".to_string(), 100)
            .await
            .unwrap();
        assert_eq!(
            cache.get(&"key1".to_string()).await,
            Some("value1".to_string())
        );
        assert_eq!(cache.get(&"nonexistent".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_eviction_by_count() {
        let config = MemoryConfig {
            max_memory_mb: 10,
            max_entries: 2,
            high_water_mark: 0.9,
            low_water_mark: 0.5,
            eviction_policy: EvictionPolicy::LRU,
            ..Default::default()
        };

        let cache = BoundedCache::new(config);

        // Fill cache to capacity
        cache
            .put("key1".to_string(), "value1".to_string(), 100)
            .await
            .unwrap();
        cache
            .put("key2".to_string(), "value2".to_string(), 100)
            .await
            .unwrap();

        // This should trigger eviction
        cache
            .put("key3".to_string(), "value3".to_string(), 100)
            .await
            .unwrap();

        // key1 should be evicted (LRU)
        assert_eq!(cache.get(&"key1".to_string()).await, None);
        assert_eq!(
            cache.get(&"key2".to_string()).await,
            Some("value2".to_string())
        );
        assert_eq!(
            cache.get(&"key3".to_string()).await,
            Some("value3".to_string())
        );
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let config = MemoryConfig {
            max_memory_mb: 10,
            max_entries: 3,
            high_water_mark: 1.0, // Allow all 3 entries before evicting
            low_water_mark: 0.5,
            eviction_policy: EvictionPolicy::LRU,
            ..Default::default()
        };

        let cache = BoundedCache::new(config);

        // Fill cache
        cache
            .put("key1".to_string(), "value1".to_string(), 100)
            .await
            .unwrap();
        cache
            .put("key2".to_string(), "value2".to_string(), 100)
            .await
            .unwrap();
        cache
            .put("key3".to_string(), "value3".to_string(), 100)
            .await
            .unwrap();

        // Access key1 to make it most recently used
        cache.get(&"key1".to_string()).await;

        // Add new item, should evict key2 (oldest after key1 access)
        cache
            .put("key4".to_string(), "value4".to_string(), 100)
            .await
            .unwrap();

        assert_eq!(
            cache.get(&"key1".to_string()).await,
            Some("value1".to_string())
        );
        assert_eq!(cache.get(&"key2".to_string()).await, None);
        assert_eq!(
            cache.get(&"key3".to_string()).await,
            Some("value3".to_string())
        );
        assert_eq!(
            cache.get(&"key4".to_string()).await,
            Some("value4".to_string())
        );
    }

    #[tokio::test]
    async fn test_memory_report() {
        let config = MemoryConfig {
            max_memory_mb: 1,
            max_entries: 10,
            ..Default::default()
        };

        let cache = BoundedCache::new(config);

        cache
            .put("key1".to_string(), "value1".to_string(), 1000)
            .await
            .unwrap();

        let report = cache.get_memory_report().await;
        assert!(report.memory_utilization > 0.0);
        assert!(report.entry_utilization > 0.0);
    }
}
