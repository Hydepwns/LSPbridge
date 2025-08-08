//! Memory management types and data structures
//!
//! This module defines the core types used throughout the memory management system,
//! including cache entries, eviction policies, and configuration structures.

use std::time::{Duration, Instant};

/// Memory eviction policies for cache management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least Recently Used - evict items that haven't been accessed recently
    LRU,
    
    /// Least Frequently Used - evict items with lowest access frequency
    LFU,
    
    /// Size Weighted - prioritize evicting large items first
    SizeWeighted,
    
    /// Age Weighted - prioritize evicting old items first
    AgeWeighted,
    
    /// Adaptive - dynamically choose based on usage patterns
    Adaptive,
}

/// Configuration for memory management
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum memory usage in megabytes
    pub max_memory_mb: usize,
    
    /// Maximum number of cache entries
    pub max_entries: usize,
    
    /// Eviction policy to use
    pub eviction_policy: EvictionPolicy,
    
    /// Start eviction at this percentage of capacity (0.0 - 1.0)
    pub high_water_mark: f64,
    
    /// Stop eviction at this percentage of capacity (0.0 - 1.0)
    pub low_water_mark: f64,
    
    /// Number of items to evict in a single batch
    pub eviction_batch_size: usize,
    
    /// Interval for memory monitoring
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

impl MemoryConfig {
    /// Create a new memory configuration with custom settings
    pub fn new(max_memory_mb: usize, max_entries: usize) -> Self {
        Self {
            max_memory_mb,
            max_entries,
            ..Default::default()
        }
    }

    /// Set eviction policy
    pub fn with_eviction_policy(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = policy;
        self
    }

    /// Set water marks for eviction
    pub fn with_water_marks(mut self, high: f64, low: f64) -> Self {
        self.high_water_mark = high.clamp(0.0, 1.0);
        self.low_water_mark = low.clamp(0.0, high);
        self
    }

    /// Set eviction batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.eviction_batch_size = size.max(1);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_memory_mb == 0 {
            return Err("max_memory_mb must be greater than 0".to_string());
        }

        if self.max_entries == 0 {
            return Err("max_entries must be greater than 0".to_string());
        }

        if self.high_water_mark > 1.0 || self.high_water_mark < 0.0 {
            return Err("high_water_mark must be between 0.0 and 1.0".to_string());
        }

        if self.low_water_mark > self.high_water_mark || self.low_water_mark < 0.0 {
            return Err("low_water_mark must be between 0.0 and high_water_mark".to_string());
        }

        if self.eviction_batch_size == 0 {
            return Err("eviction_batch_size must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Cache entry with metadata for eviction decisions
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached data
    pub data: T,
    
    /// Size of the entry in bytes
    pub size_bytes: usize,
    
    /// When the entry was created
    pub created_at: Instant,
    
    /// When the entry was last accessed
    pub last_accessed: Instant,
    
    /// Total number of accesses
    pub access_count: u64,
    
    /// Exponentially weighted moving average of access frequency
    pub access_frequency: f64,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
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

    /// Update access statistics
    pub fn update_access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;

        // Update frequency with exponential decay
        let decay_factor = 0.9;
        let time_weight = 1.0; // Could be adjusted based on time since last access
        self.access_frequency = self.access_frequency * decay_factor + time_weight;
    }

    /// Get the age of the entry
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get time since last access
    pub fn time_since_access(&self) -> Duration {
        self.last_accessed.elapsed()
    }

    /// Calculate a composite score for eviction priority
    pub fn eviction_score(&self, policy: EvictionPolicy) -> f64 {
        match policy {
            EvictionPolicy::LRU => self.time_since_access().as_secs_f64(),
            EvictionPolicy::LFU => 1.0 / (self.access_frequency + 1.0),
            EvictionPolicy::SizeWeighted => self.size_bytes as f64,
            EvictionPolicy::AgeWeighted => self.age().as_secs_f64(),
            EvictionPolicy::Adaptive => {
                // Composite score considering multiple factors
                let age_factor = self.age().as_secs_f64() / 3600.0; // Normalize to hours
                let access_factor = 1.0 / (self.access_frequency + 1.0);
                let size_factor = (self.size_bytes as f64) / (1024.0 * 1024.0); // Normalize to MB
                let recency_factor = self.time_since_access().as_secs_f64() / 60.0; // Normalize to minutes

                // Weighted combination
                age_factor * 0.2 + access_factor * 0.3 + size_factor * 0.2 + recency_factor * 0.3
            }
        }
    }
}

/// Statistics for cache performance monitoring
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Number of cache hits
    pub hits: u64,
    
    /// Number of cache misses
    pub misses: u64,
    
    /// Total number of evictions
    pub evictions: u64,
    
    /// Number of size-based evictions
    pub size_evictions: u64,
    
    /// Number of count-based evictions
    pub count_evictions: u64,
    
    /// Number of memory pressure events
    pub memory_pressure_events: u64,
    
    /// Last time cleanup was performed
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
    /// Calculate the cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate the miss rate
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Merge statistics from another instance
    pub fn merge(&mut self, other: &CacheStatistics) {
        self.hits += other.hits;
        self.misses += other.misses;
        self.evictions += other.evictions;
        self.size_evictions += other.size_evictions;
        self.count_evictions += other.count_evictions;
        self.memory_pressure_events += other.memory_pressure_events;
    }
}

/// Memory usage report
#[derive(Debug, Clone)]
pub struct MemoryReport {
    /// Total size in bytes currently used
    pub total_size_bytes: usize,
    
    /// Total number of entries currently stored
    pub total_entries: usize,
    
    /// Maximum allowed size in bytes
    pub max_size_bytes: usize,
    
    /// Maximum allowed number of entries
    pub max_entries: usize,
    
    /// Memory utilization as a percentage (0.0 - 1.0)
    pub memory_utilization: f64,
    
    /// Entry count utilization as a percentage (0.0 - 1.0)
    pub entry_utilization: f64,
    
    /// Cache hit rate
    pub hit_rate: f64,
    
    /// Eviction rate (evictions per operation)
    pub eviction_rate: f64,
    
    /// Current eviction policy
    pub eviction_policy: EvictionPolicy,
}

impl MemoryReport {
    /// Check if memory is under pressure
    pub fn is_memory_pressure(&self) -> bool {
        self.memory_utilization > 0.8 || self.entry_utilization > 0.8
    }

    /// Get memory health status
    pub fn health_status(&self) -> MemoryHealthStatus {
        if self.memory_utilization > 0.9 || self.entry_utilization > 0.9 {
            MemoryHealthStatus::Critical
        } else if self.memory_utilization > 0.8 || self.entry_utilization > 0.8 {
            MemoryHealthStatus::Warning
        } else if self.memory_utilization > 0.6 || self.entry_utilization > 0.6 {
            MemoryHealthStatus::Normal
        } else {
            MemoryHealthStatus::Healthy
        }
    }

    /// Format report as a string
    pub fn format_report(&self) -> String {
        format!(
            "Memory Report:\n\
             - Memory Usage: {:.1}% ({} / {} bytes)\n\
             - Entry Count: {:.1}% ({} / {} entries)\n\
             - Hit Rate: {:.1}%\n\
             - Eviction Policy: {:?}\n\
             - Health Status: {:?}",
            self.memory_utilization * 100.0,
            self.total_size_bytes,
            self.max_size_bytes,
            self.entry_utilization * 100.0,
            self.total_entries,
            self.max_entries,
            self.hit_rate * 100.0,
            self.eviction_policy,
            self.health_status()
        )
    }
}

/// Memory health status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryHealthStatus {
    /// Memory usage is low and healthy
    Healthy,
    
    /// Memory usage is moderate
    Normal,
    
    /// Memory usage is high - consider optimization
    Warning,
    
    /// Memory usage is critical - immediate action needed
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_config_validation() {
        let valid_config = MemoryConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_config = MemoryConfig {
            max_memory_mb: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_water_marks = MemoryConfig {
            high_water_mark: 0.5,
            low_water_mark: 0.8, // Higher than high water mark
            ..Default::default()
        };
        assert!(invalid_water_marks.validate().is_err());
    }

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new("test_data", 100);
        assert_eq!(entry.data, "test_data");
        assert_eq!(entry.size_bytes, 100);
        assert_eq!(entry.access_count, 1);
        assert_eq!(entry.access_frequency, 1.0);
    }

    #[test]
    fn test_cache_statistics() {
        let mut stats = CacheStatistics::default();
        stats.hits = 80;
        stats.misses = 20;

        assert!((stats.hit_rate() - 0.8).abs() < 0.001);
        assert!((stats.miss_rate() - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_memory_report_health_status() {
        let report = MemoryReport {
            total_size_bytes: 900,
            total_entries: 90,
            max_size_bytes: 1000,
            max_entries: 100,
            memory_utilization: 0.9,
            entry_utilization: 0.9,
            hit_rate: 0.8,
            eviction_rate: 0.1,
            eviction_policy: EvictionPolicy::LRU,
        };

        assert_eq!(report.health_status(), MemoryHealthStatus::Warning);
        assert!(report.is_memory_pressure());
    }
}