use crate::core::errors::CacheError;
use crate::core::{Diagnostic, FileHash};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub version: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub integrity_hash: String,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_path: PathBuf,
    pub hash: FileHash,
    pub last_modified: SystemTime,
    pub diagnostics: Vec<Diagnostic>,
    pub access_count: u64,
    pub last_accessed: SystemTime,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub cache_dir: PathBuf,
    pub max_size_mb: usize,
    pub max_entries: usize,
    pub ttl: Duration,
    pub enable_compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: crate::config::cache_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("lsp-bridge-cache")),
            max_size_mb: 100,
            max_entries: 10000,
            ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            enable_compression: true,
        }
    }
}

pub struct PersistentCache {
    db: Db,
    entries_tree: Tree,
    metadata_tree: Tree,
    config: CacheConfig,
    stats: RwLock<CacheStats>,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub size_bytes: usize,
    pub last_cleanup: SystemTime,
    pub errors: u64,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            size_bytes: 0,
            last_cleanup: SystemTime::now(),
            errors: 0,
        }
    }
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

impl PersistentCache {
    pub async fn new(config: CacheConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", config.cache_dir))?;

        let db_path = config.cache_dir.join("cache.db");
        let db = sled::open(&db_path)
            .with_context(|| format!("Failed to open cache database at: {:?}", db_path))?;

        let entries_tree = db.open_tree("entries")?;
        let metadata_tree = db.open_tree("metadata")?;

        let cache = Self {
            db,
            entries_tree,
            metadata_tree,
            config,
            stats: RwLock::new(CacheStats::default()),
        };

        cache.initialize_metadata().await?;
        cache.validate_integrity().await?;

        info!(
            "Persistent cache initialized at {:?}",
            cache.config.cache_dir
        );
        Ok(cache)
    }

    async fn initialize_metadata(&self) -> Result<(), CacheError> {
        if self.metadata_tree.is_empty() {
            let metadata = CacheMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                integrity_hash: self.compute_integrity_hash().await?,
                entry_count: 0,
            };

            let serialized = bincode::serialize(&metadata)?;
            self.metadata_tree.insert("metadata", serialized)?;
            self.db.flush()?;
        }
        Ok(())
    }

    async fn compute_integrity_hash(&self) -> Result<String, CacheError> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(env!("CARGO_PKG_VERSION"));
        hasher.update(self.config.cache_dir.to_string_lossy().as_bytes());

        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn validate_integrity(&self) -> Result<(), CacheError> {
        if let Some(metadata_bytes) = self.metadata_tree.get("metadata")? {
            let metadata: CacheMetadata = bincode::deserialize(&metadata_bytes)?;

            // Check version compatibility
            if metadata.version != env!("CARGO_PKG_VERSION") {
                warn!("Cache version mismatch, clearing cache");
                self.clear_all().await?;
                return Ok(());
            }

            // Validate integrity hash
            let current_hash = self.compute_integrity_hash().await?;
            if metadata.integrity_hash != current_hash {
                warn!("Cache integrity compromised, clearing cache");
                self.clear_all().await?;
                return Ok(());
            }

            // Update access time
            let updated_metadata = CacheMetadata {
                last_accessed: SystemTime::now(),
                ..metadata
            };

            let serialized = bincode::serialize(&updated_metadata)?;
            self.metadata_tree.insert("metadata", serialized)?;
        }

        Ok(())
    }

    pub async fn get(&self, file_path: &Path) -> Option<CacheEntry> {
        let key = self.path_to_key(file_path);

        match self.entries_tree.get(&key) {
            Ok(Some(data)) => {
                match bincode::deserialize::<CacheEntry>(&data) {
                    Ok(mut entry) => {
                        // Check TTL
                        if self.is_expired(&entry) {
                            self.remove_expired(&key).await;
                            self.record_miss().await;
                            return None;
                        }

                        // Update access info
                        entry.access_count += 1;
                        entry.last_accessed = SystemTime::now();

                        // Write back updated entry
                        if let Ok(serialized) = bincode::serialize(&entry) {
                            let _ = self.entries_tree.insert(&key, serialized);
                        }

                        self.record_hit().await;
                        Some(entry)
                    }
                    Err(e) => {
                        error!("Failed to deserialize cache entry: {}", e);
                        self.record_error().await;
                        None
                    }
                }
            }
            Ok(None) => {
                self.record_miss().await;
                None
            }
            Err(e) => {
                error!("Cache read error: {}", e);
                self.record_error().await;
                None
            }
        }
    }

    pub async fn put(&self, entry: CacheEntry) -> Result<(), CacheError> {
        let key = self.path_to_key(&entry.file_path);

        // Check if we need to evict entries first
        if self.should_evict().await? {
            self.evict_lru().await?;
        }

        let serialized = bincode::serialize(&entry)?;
        self.entries_tree.insert(&key, serialized)?;

        // Update metadata
        self.update_metadata().await?;

        debug!("Cached entry for {:?}", entry.file_path);
        Ok(())
    }

    pub async fn remove(&self, file_path: &Path) -> Result<bool, CacheError> {
        let key = self.path_to_key(file_path);
        Ok(self.entries_tree.remove(&key)?.is_some())
    }

    pub async fn clear_all(&self) -> Result<(), CacheError> {
        self.entries_tree.clear()?;
        self.metadata_tree.clear()?;
        self.db.flush()?;

        {
            let mut stats = self.stats.write().await;
            *stats = CacheStats::default();
        }

        self.initialize_metadata().await?;
        info!("Cache cleared completely");
        Ok(())
    }

    pub async fn cleanup_expired(&self) -> Result<usize, CacheError> {
        let mut removed_count = 0;
        let mut to_remove = Vec::new();

        for result in self.entries_tree.iter() {
            match result {
                Ok((key, value)) => {
                    if let Ok(entry) = bincode::deserialize::<CacheEntry>(&value) {
                        if self.is_expired(&entry) {
                            to_remove.push(key);
                        }
                    }
                }
                Err(e) => {
                    error!("Error during cleanup iteration: {}", e);
                }
            }
        }

        for key in to_remove {
            if self.entries_tree.remove(&key)?.is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            self.update_metadata().await?;
            info!("Cleaned up {} expired cache entries", removed_count);
        }

        Ok(removed_count)
    }

    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.entries = self.entries_tree.len();
        result
    }

    pub async fn optimize(&self) -> Result<(), CacheError> {
        // Perform database optimization
        self.cleanup_expired().await?;

        // Compact the database
        let space_amplification =
            self.db.size_on_disk()? as f64 / self.calculate_logical_size().await as f64;
        if space_amplification > 2.0 {
            info!(
                "Compacting database (space amplification: {:.2})",
                space_amplification
            );
            // Note: sled doesn't have explicit compaction, but we can force a flush
            self.db.flush()?;
        }

        Ok(())
    }

    // Private helper methods

    fn path_to_key(&self, path: &Path) -> Vec<u8> {
        path.to_string_lossy().as_bytes().to_vec()
    }

    fn is_expired(&self, entry: &CacheEntry) -> bool {
        match SystemTime::now().duration_since(entry.last_accessed) {
            Ok(age) => age > self.config.ttl,
            Err(_) => true, // Clock went backwards, consider expired
        }
    }

    async fn remove_expired(&self, key: &[u8]) {
        if let Err(e) = self.entries_tree.remove(key) {
            error!("Failed to remove expired entry: {}", e);
        }
    }

    async fn should_evict(&self) -> Result<bool, CacheError> {
        let current_entries = self.entries_tree.len();
        let current_size_mb = self.calculate_size_mb().await;

        Ok(
            current_entries >= self.config.max_entries
                || current_size_mb >= self.config.max_size_mb,
        )
    }

    async fn evict_lru(&self) -> Result<(), CacheError> {
        let mut entries_with_access: Vec<(Vec<u8>, SystemTime, u64)> = Vec::new();

        // Collect all entries with their access times
        for result in self.entries_tree.iter() {
            if let Ok((key, value)) = result {
                if let Ok(entry) = bincode::deserialize::<CacheEntry>(&value) {
                    entries_with_access.push((
                        key.to_vec(),
                        entry.last_accessed,
                        entry.access_count,
                    ));
                }
            }
        }

        // Sort by last accessed time (oldest first)
        entries_with_access.sort_by_key(|(_, accessed, _)| *accessed);

        // Remove oldest 25% of entries
        let to_remove = entries_with_access.len() / 4;
        let mut removed = 0;

        for (key, _, _) in entries_with_access.iter().take(to_remove) {
            if self.entries_tree.remove(key)?.is_some() {
                removed += 1;
            }
        }

        info!("Evicted {} LRU cache entries", removed);
        Ok(())
    }

    async fn calculate_size_mb(&self) -> usize {
        (self.db.size_on_disk().unwrap_or(0) / (1024 * 1024)) as usize
    }

    async fn calculate_logical_size(&self) -> usize {
        let mut total_size = 0;
        for result in self.entries_tree.iter() {
            if let Ok((key, value)) = result {
                total_size += key.len() + value.len();
            }
        }
        total_size
    }

    async fn update_metadata(&self) -> Result<(), CacheError> {
        if let Some(metadata_bytes) = self.metadata_tree.get("metadata")? {
            let mut metadata: CacheMetadata = bincode::deserialize(&metadata_bytes)?;
            metadata.last_accessed = SystemTime::now();
            metadata.entry_count = self.entries_tree.len();

            let serialized = bincode::serialize(&metadata)?;
            self.metadata_tree.insert("metadata", serialized)?;
        }
        Ok(())
    }

    async fn record_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.hits += 1;
    }

    async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.misses += 1;
    }

    async fn record_error(&self) {
        let mut stats = self.stats.write().await;
        stats.errors += 1;
    }
}

impl Drop for PersistentCache {
    fn drop(&mut self) {
        if let Err(e) = self.db.flush() {
            error!("Failed to flush cache on drop: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_basic_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_size_mb: 10,
            max_entries: 100,
            ttl: Duration::from_secs(3600),
            enable_compression: false,
        };

        let cache = PersistentCache::new(config).await?;

        let file_path = PathBuf::from("/test/file.rs");
        let entry = CacheEntry {
            file_path: file_path.clone(),
            hash: FileHash::new(b"test content"),
            last_modified: SystemTime::now(),
            diagnostics: vec![],
            access_count: 0,
            last_accessed: SystemTime::now(),
        };

        // Test put and get
        cache.put(entry.clone()).await?;
        let retrieved = cache.get(&file_path).await;
        assert!(retrieved.is_some());

        // Test stats
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_expiration() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_size_mb: 10,
            max_entries: 100,
            ttl: Duration::from_millis(100), // Very short TTL
            enable_compression: false,
        };

        let cache = PersistentCache::new(config).await?;

        let file_path = PathBuf::from("/test/file.rs");
        let entry = CacheEntry {
            file_path: file_path.clone(),
            hash: FileHash::new(b"test content"),
            last_modified: SystemTime::now(),
            diagnostics: vec![],
            access_count: 0,
            last_accessed: SystemTime::now() - Duration::from_secs(1),
        };

        cache.put(entry).await?;

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(200)).await;

        let retrieved = cache.get(&file_path).await;
        assert!(retrieved.is_none());

        Ok(())
    }
}
