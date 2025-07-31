use crate::history::storage::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    inserted_at: SystemTime,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            inserted_at: SystemTime::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.inserted_at)
            .map(|elapsed| elapsed > ttl)
            .unwrap_or(true)
    }
}

pub struct QueryCache {
    file_snapshots: Arc<RwLock<HashMap<PathBuf, CacheEntry<Vec<DiagnosticSnapshot>>>>>,
    file_stats: Arc<RwLock<HashMap<PathBuf, CacheEntry<Option<FileHistoryStats>>>>>,
    patterns: Arc<RwLock<Option<CacheEntry<Vec<HistoricalErrorPattern>>>>>,
    ttl: Duration,
}

impl QueryCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            file_snapshots: Arc::new(RwLock::new(HashMap::new())),
            file_stats: Arc::new(RwLock::new(HashMap::new())),
            patterns: Arc::new(RwLock::new(None)),
            ttl,
        }
    }

    pub async fn get_file_snapshots(
        &self,
        file_path: &Path,
    ) -> Option<Vec<DiagnosticSnapshot>> {
        let cache = self.file_snapshots.read().await;
        cache
            .get(file_path)
            .filter(|entry| !entry.is_expired(self.ttl))
            .map(|entry| entry.data.clone())
    }

    pub async fn cache_file_snapshots(
        &self,
        file_path: &Path,
        snapshots: Vec<DiagnosticSnapshot>,
    ) {
        let mut cache = self.file_snapshots.write().await;
        cache.insert(file_path.to_path_buf(), CacheEntry::new(snapshots));
        
        // Clean up expired entries periodically
        if cache.len() > 100 {
            self.cleanup_file_snapshots(&mut cache).await;
        }
    }

    pub async fn get_file_stats(
        &self,
        file_path: &Path,
    ) -> Option<Option<FileHistoryStats>> {
        let cache = self.file_stats.read().await;
        cache
            .get(file_path)
            .filter(|entry| !entry.is_expired(self.ttl))
            .map(|entry| entry.data.clone())
    }

    pub async fn cache_file_stats(
        &self,
        file_path: &Path,
        stats: Option<FileHistoryStats>,
    ) {
        let mut cache = self.file_stats.write().await;
        cache.insert(file_path.to_path_buf(), CacheEntry::new(stats));
    }

    pub async fn get_patterns(&self) -> Option<Vec<HistoricalErrorPattern>> {
        let cache = self.patterns.read().await;
        cache
            .as_ref()
            .filter(|entry| !entry.is_expired(self.ttl))
            .map(|entry| entry.data.clone())
    }

    pub async fn cache_patterns(&self, patterns: Vec<HistoricalErrorPattern>) {
        let mut cache = self.patterns.write().await;
        *cache = Some(CacheEntry::new(patterns));
    }

    pub async fn invalidate_file(&self, file_path: &Path) {
        let mut snapshots = self.file_snapshots.write().await;
        snapshots.remove(file_path);
        
        let mut stats = self.file_stats.write().await;
        stats.remove(file_path);
    }

    pub async fn invalidate_all(&self) {
        self.file_snapshots.write().await.clear();
        self.file_stats.write().await.clear();
        *self.patterns.write().await = None;
    }

    async fn cleanup_file_snapshots(
        &self,
        cache: &mut HashMap<PathBuf, CacheEntry<Vec<DiagnosticSnapshot>>>,
    ) {
        cache.retain(|_, entry| !entry.is_expired(self.ttl));
    }
}