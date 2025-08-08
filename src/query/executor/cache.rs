//! Caching and validation utilities for query execution
//!
//! This module provides caching mechanisms for query results and validation
//! utilities to ensure query safety and performance.

use super::types::QueryResult;
use std::collections::HashMap;
use std::time::Instant;

/// Cached query result with timestamp
#[derive(Clone)]
pub struct CachedResult {
    pub result: QueryResult,
    pub cached_at: Instant,
}

/// Query result cache with TTL (Time To Live) support
pub struct QueryCache {
    cache: HashMap<String, CachedResult>,
    default_ttl_secs: u64,
    max_entries: usize,
}

impl QueryCache {
    /// Create a new query cache
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            default_ttl_secs: 300, // 5 minutes default
            max_entries: 1000,
        }
    }

    /// Create a cache with custom settings
    pub fn with_settings(ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            cache: HashMap::new(),
            default_ttl_secs: ttl_secs,
            max_entries,
        }
    }

    /// Get a cached result if it exists and hasn't expired
    pub fn get(&mut self, key: &str) -> Option<QueryResult> {
        self.cleanup_expired();
        
        if let Some(cached) = self.cache.get(key) {
            if cached.cached_at.elapsed().as_secs() < self.default_ttl_secs {
                let mut result = cached.result.clone();
                result.metadata.cache_hit = true;
                return Some(result);
            } else {
                // Remove expired entry
                self.cache.remove(key);
            }
        }
        
        None
    }

    /// Store a result in the cache
    pub fn insert(&mut self, key: String, result: QueryResult) {
        // Ensure we don't exceed max entries
        if self.cache.len() >= self.max_entries {
            self.evict_oldest();
        }

        let cached_result = CachedResult {
            result,
            cached_at: Instant::now(),
        };

        self.cache.insert(key, cached_result);
    }

    /// Clear all cached results
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.cache.len(),
            max_entries: self.max_entries,
            ttl_seconds: self.default_ttl_secs,
        }
    }

    /// Remove expired entries from cache
    fn cleanup_expired(&mut self) {
        let expired_keys: Vec<String> = self
            .cache
            .iter()
            .filter(|(_, cached)| cached.cached_at.elapsed().as_secs() >= self.default_ttl_secs)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.cache.remove(&key);
        }
    }

    /// Evict the oldest entry when cache is full
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, cached)| cached.cached_at)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            self.cache.remove(&oldest_key);
        }
    }

    /// Update TTL for the cache
    pub fn set_ttl(&mut self, ttl_secs: u64) {
        self.default_ttl_secs = ttl_secs;
    }

    /// Set maximum number of cache entries
    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries;
        
        // Evict entries if we're over the new limit
        while self.cache.len() > max_entries {
            self.evict_oldest();
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub max_entries: usize,
    pub ttl_seconds: u64,
}

/// Query validation utilities
pub struct QueryValidator;

impl QueryValidator {
    /// Generate a cache key from a query
    ///
    /// This creates a deterministic key that uniquely identifies queries
    /// for caching purposes. The key includes all query parameters that
    /// could affect the result.
    pub fn generate_cache_key(query: &crate::query::parser::Query) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // Hash all query components that affect results
        format!("{:?}", query.select).hash(&mut hasher);
        format!("{:?}", query.from).hash(&mut hasher);
        format!("{:?}", query.filters).hash(&mut hasher);
        format!("{:?}", query.group_by).hash(&mut hasher);
        format!("{:?}", query.order_by).hash(&mut hasher);
        query.limit.hash(&mut hasher);
        format!("{:?}", query.time_range).hash(&mut hasher);

        format!("query_{:x}", hasher.finish())
    }

    /// Validate query for performance and security concerns
    pub fn validate_query_safety(query: &crate::query::parser::Query) -> Result<(), Vec<String>> {
        let mut warnings = Vec::new();

        // Check for potential performance issues
        if query.limit.is_none() {
            warnings.push("Query without LIMIT may return large result sets".to_string());
        }

        if let Some(limit) = query.limit {
            if limit > 10000 {
                warnings.push("Very large LIMIT may impact performance".to_string());
            }
        }

        // Check for time range on history queries
        if matches!(query.from, crate::query::parser::FromClause::History) && query.time_range.is_none() {
            warnings.push("History queries should include time range for better performance".to_string());
        }

        // Check aggregation without GROUP BY
        if let crate::query::parser::SelectClause::Aggregations(aggs) = &query.select {
            if aggs.len() > 1 && query.group_by.is_none() {
                warnings.push("Multiple aggregations without GROUP BY may produce unexpected results".to_string());
            }
        }

        // Validate filter complexity
        if query.filters.len() > 10 {
            warnings.push("Too many filters may impact query performance".to_string());
        }

        if warnings.is_empty() {
            Ok(())
        } else {
            Err(warnings)
        }
    }

    /// Estimate query execution cost
    pub fn estimate_query_cost(query: &crate::query::parser::Query) -> QueryCost {
        let mut cost = QueryCost::default();

        // Base cost from data source
        cost.base_cost = match query.from {
            crate::query::parser::FromClause::Diagnostics => 10,
            crate::query::parser::FromClause::Files => 15,
            crate::query::parser::FromClause::History => 50,
            crate::query::parser::FromClause::Trends => 100,
            crate::query::parser::FromClause::Symbols => 20,
            crate::query::parser::FromClause::References => 25,
            crate::query::parser::FromClause::Projects => 30,
        };

        // Filter cost
        cost.filter_cost = (query.filters.len() * 5) as u32;

        // Aggregation cost
        if let crate::query::parser::SelectClause::Aggregations(aggs) = &query.select {
            cost.aggregation_cost = (aggs.len() * 20) as u32;
        }

        // Sorting cost
        if query.order_by.is_some() {
            cost.sorting_cost = 25;
        }

        // GROUP BY cost
        if query.group_by.is_some() {
            cost.grouping_cost = 30;
        }

        cost
    }
}

/// Query execution cost estimate
#[derive(Debug, Clone, Default)]
pub struct QueryCost {
    pub base_cost: u32,
    pub filter_cost: u32,
    pub aggregation_cost: u32,
    pub sorting_cost: u32,
    pub grouping_cost: u32,
}

impl QueryCost {
    /// Calculate total estimated cost
    pub fn total(&self) -> u32 {
        self.base_cost + self.filter_cost + self.aggregation_cost + self.sorting_cost + self.grouping_cost
    }

    /// Check if query is expensive
    pub fn is_expensive(&self) -> bool {
        self.total() > 200
    }

    /// Get cost category
    pub fn category(&self) -> CostCategory {
        match self.total() {
            0..=49 => CostCategory::Low,
            50..=150 => CostCategory::Medium,
            151..=300 => CostCategory::High,
            _ => CostCategory::VeryHigh,
        }
    }
}

/// Query cost categories
#[derive(Debug, Clone, PartialEq)]
pub enum CostCategory {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern-based query key generator for similar queries
pub struct QueryKeyGenerator;

impl QueryKeyGenerator {
    /// Generate a pattern-based key that groups similar queries
    ///
    /// This is useful for caching strategies where similar queries
    /// (differing only in parameter values) can share cached results.
    pub fn generate_pattern_key(query: &crate::query::parser::Query) -> String {
        let mut key_parts = Vec::new();

        // Add data source
        key_parts.push(format!("from:{:?}", query.from));

        // Add select type (not specific fields)
        let select_type = match &query.select {
            crate::query::parser::SelectClause::All => "select:*",
            crate::query::parser::SelectClause::Count => "select:count",
            crate::query::parser::SelectClause::Fields(_) => "select:fields",
            crate::query::parser::SelectClause::Aggregations(_) => "select:agg",
        };
        key_parts.push(select_type.to_string());

        // Add filter types (not specific values)
        let mut filter_types = Vec::new();
        for filter in &query.filters {
            let filter_type = match filter {
                crate::query::parser::QueryFilter::Path(_) => "path",
                crate::query::parser::QueryFilter::File(_) => "file",
                crate::query::parser::QueryFilter::Symbol(_) => "symbol",
                crate::query::parser::QueryFilter::Severity(_) => "severity",
                crate::query::parser::QueryFilter::Category(_) => "category",
                crate::query::parser::QueryFilter::Message(_) => "message",
                crate::query::parser::QueryFilter::TimeRange(_) => "time",
                crate::query::parser::QueryFilter::FileCount(_) => "filecount",
                crate::query::parser::QueryFilter::Custom(field, _) => return format!("custom:{field}"),
            };
            filter_types.push(filter_type);
        }
        filter_types.sort();
        key_parts.push(format!("filters:{}", filter_types.join(",")));

        // Add structural elements
        if query.group_by.is_some() {
            key_parts.push("grouped:true".to_string());
        }
        if query.order_by.is_some() {
            key_parts.push("ordered:true".to_string());
        }
        if query.limit.is_some() {
            key_parts.push("limited:true".to_string());
        }

        key_parts.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::{FromClause, Query, SelectClause};

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = QueryCache::new();
        
        let result = super::super::types::QueryResult::empty("test");
        cache.insert("key1".to_string(), result.clone());
        
        let retrieved = cache.get("key1");
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().metadata.cache_hit);
    }

    #[test]
    fn test_cache_expiration() {
        let mut cache = QueryCache::with_settings(1, 100); // 1 second TTL
        
        let result = super::super::types::QueryResult::empty("test");
        cache.insert("key1".to_string(), result);
        
        // Should be available immediately
        assert!(cache.get("key1").is_some());
        
        // Wait for expiration (in real test, we'd mock time)
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = QueryCache::with_settings(300, 2); // Max 2 entries
        
        let result = super::super::types::QueryResult::empty("test");
        
        cache.insert("key1".to_string(), result.clone());
        cache.insert("key2".to_string(), result.clone());
        cache.insert("key3".to_string(), result.clone()); // Should evict oldest
        
        assert_eq!(cache.cache.len(), 2);
        assert!(cache.get("key1").is_none()); // Should be evicted
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_query_cost_estimation() {
        let query = Query {
            select: SelectClause::All,
            from: FromClause::History,
            filters: vec![], // Would need actual filter types
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let cost = QueryValidator::estimate_query_cost(&query);
        assert_eq!(cost.base_cost, 50); // History base cost
        assert_eq!(cost.category(), CostCategory::Medium);
    }

    #[test]
    fn test_cache_key_generation() {
        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: Some(10),
            time_range: None,
        };

        let key1 = QueryValidator::generate_cache_key(&query);
        let key2 = QueryValidator::generate_cache_key(&query);
        
        // Same query should generate same key
        assert_eq!(key1, key2);
        
        // Different query should generate different key
        let mut different_query = query.clone();
        different_query.limit = Some(20);
        let key3 = QueryValidator::generate_cache_key(&different_query);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_pattern_key_generation() {
        let query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: vec![], // Would need actual filters
            group_by: None,
            order_by: None,
            limit: Some(10),
            time_range: None,
        };

        let pattern_key = QueryKeyGenerator::generate_pattern_key(&query);
        assert!(pattern_key.contains("from:Diagnostics"));
        assert!(pattern_key.contains("select:*"));
        assert!(pattern_key.contains("limited:true"));
    }
}