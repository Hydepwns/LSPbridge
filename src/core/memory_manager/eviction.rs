//! Eviction strategies for memory management
//!
//! This module implements various cache eviction strategies including LRU, LFU,
//! size-weighted, age-weighted, and adaptive eviction policies.

use anyhow::Result;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use tokio::sync::RwLock;
use tracing::info;

use super::types::{CacheEntry, CacheStatistics, EvictionPolicy, MemoryConfig};

/// Enum to represent eviction strategy choice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionChoice {
    LRU,
    LFU,
    SizeWeighted,
    AgeWeighted,
}

/// Trait for eviction strategies
pub trait EvictionStrategy<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Evict entries to meet target size and count constraints
    #[allow(async_fn_in_trait)]
    async fn evict(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        target_size: f64,
        target_count: f64,
        batch_size: usize,
    ) -> Result<u64>;
}

/// LRU (Least Recently Used) eviction strategy
pub struct LRUEviction;

impl<K, V> EvictionStrategy<K, V> for LRUEviction
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    async fn evict(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        target_size: f64,
        target_count: f64,
        batch_size: usize,
    ) -> Result<u64> {
        let mut evicted = 0;
        let mut entries_guard = entries.write().await;
        let mut access_order_guard = access_order.write().await;

        while (current_size.load(AtomicOrdering::SeqCst) as f64 > target_size
            || current_entries.load(AtomicOrdering::SeqCst) as f64 > target_count)
            && !access_order_guard.is_empty()
            && evicted < batch_size as u64
        {
            if let Some(key) = access_order_guard.pop_front() {
                if let Some(entry) = entries_guard.remove(&key) {
                    current_size.fetch_sub(entry.size_bytes, AtomicOrdering::SeqCst);
                    current_entries.fetch_sub(1, AtomicOrdering::SeqCst);
                    evicted += 1;
                }
            }
        }

        Ok(evicted)
    }
}

/// LFU (Least Frequently Used) eviction strategy
pub struct LFUEviction;

impl<K, V> EvictionStrategy<K, V> for LFUEviction
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    async fn evict(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        target_size: f64,
        target_count: f64,
        batch_size: usize,
    ) -> Result<u64> {
        let mut evicted = 0;
        let mut entries_guard = entries.write().await;
        let mut access_order_guard = access_order.write().await;

        // Collect entries with their frequencies
        let mut freq_entries: Vec<_> = entries_guard
            .iter()
            .map(|(k, v)| (k.clone(), v.access_frequency))
            .collect();

        // Sort by frequency (ascending - lowest frequency first)
        freq_entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        for (key, _) in freq_entries {
            if (current_size.load(AtomicOrdering::SeqCst) as f64 <= target_size
                && current_entries.load(AtomicOrdering::SeqCst) as f64 <= target_count)
                || evicted >= batch_size as u64
            {
                break;
            }

            if let Some(entry) = entries_guard.remove(&key) {
                current_size.fetch_sub(entry.size_bytes, AtomicOrdering::SeqCst);
                current_entries.fetch_sub(1, AtomicOrdering::SeqCst);

                // Remove from access order
                if let Some(pos) = access_order_guard.iter().position(|k| k == &key) {
                    access_order_guard.remove(pos);
                }

                evicted += 1;
            }
        }

        Ok(evicted)
    }
}

/// Size-weighted eviction strategy (evict largest items first)
pub struct SizeWeightedEviction;

impl<K, V> EvictionStrategy<K, V> for SizeWeightedEviction
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    async fn evict(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        target_size: f64,
        target_count: f64,
        batch_size: usize,
    ) -> Result<u64> {
        let mut evicted = 0;
        let mut entries_guard = entries.write().await;
        let mut access_order_guard = access_order.write().await;

        // Collect entries with their sizes
        let mut size_entries: Vec<_> = entries_guard
            .iter()
            .map(|(k, v)| (k.clone(), v.size_bytes))
            .collect();

        // Sort by size (descending - largest first)
        size_entries.sort_by(|a, b| b.1.cmp(&a.1));

        for (key, _) in size_entries {
            if (current_size.load(AtomicOrdering::SeqCst) as f64 <= target_size
                && current_entries.load(AtomicOrdering::SeqCst) as f64 <= target_count)
                || evicted >= batch_size as u64
            {
                break;
            }

            if let Some(entry) = entries_guard.remove(&key) {
                current_size.fetch_sub(entry.size_bytes, AtomicOrdering::SeqCst);
                current_entries.fetch_sub(1, AtomicOrdering::SeqCst);

                // Remove from access order
                if let Some(pos) = access_order_guard.iter().position(|k| k == &key) {
                    access_order_guard.remove(pos);
                }

                evicted += 1;
            }
        }

        Ok(evicted)
    }
}

/// Age-weighted eviction strategy (evict oldest items first)
pub struct AgeWeightedEviction;

impl<K, V> EvictionStrategy<K, V> for AgeWeightedEviction
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    async fn evict(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        target_size: f64,
        target_count: f64,
        batch_size: usize,
    ) -> Result<u64> {
        let mut evicted = 0;
        let mut entries_guard = entries.write().await;
        let mut access_order_guard = access_order.write().await;

        // Collect entries with their ages
        let mut age_entries: Vec<_> = entries_guard
            .iter()
            .map(|(k, v)| (k.clone(), v.age()))
            .collect();

        // Sort by age (descending - oldest first)
        age_entries.sort_by(|a, b| b.1.cmp(&a.1));

        for (key, _) in age_entries {
            if (current_size.load(AtomicOrdering::SeqCst) as f64 <= target_size
                && current_entries.load(AtomicOrdering::SeqCst) as f64 <= target_count)
                || evicted >= batch_size as u64
            {
                break;
            }

            if let Some(entry) = entries_guard.remove(&key) {
                current_size.fetch_sub(entry.size_bytes, AtomicOrdering::SeqCst);
                current_entries.fetch_sub(1, AtomicOrdering::SeqCst);

                // Remove from access order
                if let Some(pos) = access_order_guard.iter().position(|k| k == &key) {
                    access_order_guard.remove(pos);
                }

                evicted += 1;
            }
        }

        Ok(evicted)
    }
}

/// Adaptive eviction strategy that chooses the best strategy based on current conditions
pub struct AdaptiveEviction {
    lru: LRUEviction,
    lfu: LFUEviction,
    size_weighted: SizeWeightedEviction,
    age_weighted: AgeWeightedEviction,
}

impl Default for AdaptiveEviction {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveEviction {
    /// Create a new adaptive eviction strategy
    pub fn new() -> Self {
        Self {
            lru: LRUEviction,
            lfu: LFUEviction,
            size_weighted: SizeWeightedEviction,
            age_weighted: AgeWeightedEviction,
        }
    }

    /// Choose the best eviction strategy based on current conditions
    pub fn choose_strategy(
        &self,
        memory_pressure: f64,
        hit_rate: f64,
        average_entry_size: usize,
    ) -> EvictionChoice {
        if memory_pressure > 0.9 {
            // Very high memory pressure: prioritize freeing space quickly
            EvictionChoice::SizeWeighted
        } else if hit_rate < 0.5 {
            // Low hit rate: items aren't being reused much, use LRU
            EvictionChoice::LRU
        } else if hit_rate > 0.8 {
            // High hit rate: keep frequently used items
            EvictionChoice::LFU
        } else if average_entry_size > 1024 * 1024 {
            // Large average entry size: size-weighted to free more memory per eviction
            EvictionChoice::SizeWeighted
        } else {
            // Balanced approach: age-weighted eviction
            EvictionChoice::AgeWeighted
        }
    }
}

impl<K, V> EvictionStrategy<K, V> for AdaptiveEviction
where
    K: Clone + Eq + Hash + 'static,
    V: Clone + 'static,
{
    async fn evict(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        target_size: f64,
        target_count: f64,
        batch_size: usize,
    ) -> Result<u64> {
        // Calculate current conditions
        let memory_pressure = current_size.load(AtomicOrdering::SeqCst) as f64 / target_size;
        
        // This is a simplified calculation - in real use, these would come from cache statistics
        let hit_rate = 0.7; // Default to balanced value
        let average_entry_size = if current_entries.load(AtomicOrdering::SeqCst) > 0 {
            current_size.load(AtomicOrdering::SeqCst) / current_entries.load(AtomicOrdering::SeqCst)
        } else {
            1024 // Default 1KB
        };

        let strategy_choice = self.choose_strategy(memory_pressure, hit_rate, average_entry_size);
        
        info!(
            "Adaptive eviction choosing strategy based on: memory_pressure={:.2}, hit_rate={:.2}, avg_size={}",
            memory_pressure, hit_rate, average_entry_size
        );

        match strategy_choice {
            EvictionChoice::LRU => {
                self.lru.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    batch_size,
                ).await
            },
            EvictionChoice::LFU => {
                self.lfu.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    batch_size,
                ).await
            },
            EvictionChoice::SizeWeighted => {
                self.size_weighted.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    batch_size,
                ).await
            },
            EvictionChoice::AgeWeighted => {
                self.age_weighted.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    batch_size,
                ).await
            },
        }
    }
}

/// Eviction manager that coordinates eviction operations
pub struct EvictionManager {
    config: MemoryConfig,
    lru: LRUEviction,
    lfu: LFUEviction,
    size_weighted: SizeWeightedEviction,
    age_weighted: AgeWeightedEviction,
    adaptive: AdaptiveEviction,
}

impl EvictionManager {
    /// Create a new eviction manager
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            lru: LRUEviction,
            lfu: LFUEviction,
            size_weighted: SizeWeightedEviction,
            age_weighted: AgeWeightedEviction,
            adaptive: AdaptiveEviction::new(),
        }
    }

    /// Get the configured eviction policy
    pub fn get_strategy_policy(&self) -> &EvictionPolicy {
        &self.config.eviction_policy
    }

    /// Perform eviction if needed
    pub async fn evict_if_needed<K, V>(
        &self,
        entries: &RwLock<HashMap<K, CacheEntry<V>>>,
        access_order: &RwLock<VecDeque<K>>,
        current_size: &AtomicUsize,
        current_entries: &AtomicUsize,
        statistics: &RwLock<CacheStatistics>,
        new_item_size: usize,
    ) -> Result<u64>
    where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        let size_threshold = (self.config.max_memory_mb * 1024 * 1024) as f64 * self.config.high_water_mark;
        let count_threshold = self.config.max_entries as f64 * self.config.high_water_mark;

        let current_size_val = current_size.load(AtomicOrdering::SeqCst);
        let current_entries_val = current_entries.load(AtomicOrdering::SeqCst);

        let will_exceed_size = (current_size_val + new_item_size) as f64 > size_threshold;
        let will_exceed_count = (current_entries_val + 1) as f64 > count_threshold;

        if !will_exceed_size && !will_exceed_count {
            return Ok(0);
        }

        // Calculate target levels
        let target_size = (self.config.max_memory_mb * 1024 * 1024) as f64 * self.config.low_water_mark;
        let target_count = self.config.max_entries as f64 * self.config.low_water_mark;

        // Perform eviction based on policy
        let evicted = match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                self.lru.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    self.config.eviction_batch_size,
                ).await?
            },
            EvictionPolicy::LFU => {
                self.lfu.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    self.config.eviction_batch_size,
                ).await?
            },
            EvictionPolicy::SizeWeighted => {
                self.size_weighted.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    self.config.eviction_batch_size,
                ).await?
            },
            EvictionPolicy::AgeWeighted => {
                self.age_weighted.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    self.config.eviction_batch_size,
                ).await?
            },
            EvictionPolicy::Adaptive => {
                self.adaptive.evict(
                    entries,
                    access_order,
                    current_size,
                    current_entries,
                    target_size,
                    target_count,
                    self.config.eviction_batch_size,
                ).await?
            },
        };

        // Update statistics
        if evicted > 0 {
            let mut stats = statistics.write().await;
            stats.evictions += evicted;
            
            if current_size.load(AtomicOrdering::SeqCst) as f64 > target_size {
                stats.size_evictions += 1;
            }
            if current_entries.load(AtomicOrdering::SeqCst) as f64 > target_count {
                stats.count_evictions += 1;
            }
            
            info!(
                "Evicted {} entries using {:?} policy",
                evicted, self.config.eviction_policy
            );
        }

        Ok(evicted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_lru_eviction() {
        let entries = RwLock::new(HashMap::new());
        let access_order = RwLock::new(VecDeque::new());
        let current_size = AtomicUsize::new(0);
        let current_entries = AtomicUsize::new(0);

        // Add some entries
        {
            let mut entries_guard = entries.write().await;
            let mut access_order_guard = access_order.write().await;

            for i in 1..=5 {
                let key = format!("key{}", i);
                entries_guard.insert(key.clone(), CacheEntry::new(format!("value{}", i), 100));
                access_order_guard.push_back(key);
                current_size.fetch_add(100, AtomicOrdering::SeqCst);
                current_entries.fetch_add(1, AtomicOrdering::SeqCst);
            }
        }

        let lru = LRUEviction;
        let evicted = lru
            .evict(
                &entries,
                &access_order,
                &current_size,
                &current_entries,
                300.0,  // Target size
                3.0,    // Target count
                10,     // Batch size
            )
            .await
            .unwrap();

        assert_eq!(evicted, 2);
        assert_eq!(current_entries.load(AtomicOrdering::SeqCst), 3);
        assert_eq!(current_size.load(AtomicOrdering::SeqCst), 300);
    }

    #[test]
    fn test_adaptive_strategy_selection() {
        let adaptive = AdaptiveEviction::new();

        // High memory pressure - should choose size-weighted
        let choice = adaptive.choose_strategy(0.95, 0.7, 1024);
        assert_eq!(choice, EvictionChoice::SizeWeighted);

        // Low hit rate - should choose LRU
        let choice = adaptive.choose_strategy(0.5, 0.3, 1024);
        assert_eq!(choice, EvictionChoice::LRU);

        // High hit rate - should choose LFU
        let choice = adaptive.choose_strategy(0.7, 0.85, 512);
        assert_eq!(choice, EvictionChoice::LFU);

        // Large entries - should choose size-weighted
        let choice = adaptive.choose_strategy(0.6, 0.6, 2 * 1024 * 1024);
        assert_eq!(choice, EvictionChoice::SizeWeighted);

        // Default case - should choose age-weighted
        let choice = adaptive.choose_strategy(0.7, 0.6, 512);
        assert_eq!(choice, EvictionChoice::AgeWeighted);
    }
}