use crate::core::Diagnostic;
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHash(String);

impl FileHash {
    pub fn new(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = format!("{:x}", hasher.finalize());
        Self(hash)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read(path)?;
        Ok(Self::new(&content))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub hash: FileHash,
    pub last_modified: SystemTime,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_files: usize,
    pub changed_files: usize,
    pub cached_files: usize,
    pub processing_time: Duration,
    pub cache_hit_rate: f32,
}

pub struct IncrementalProcessor {
    file_hashes: RwLock<HashMap<PathBuf, FileHash>>,
    last_diagnostics: RwLock<HashMap<PathBuf, Vec<Diagnostic>>>,
    file_metadata: RwLock<HashMap<PathBuf, SystemTime>>,
    parallel_chunk_size: usize,
    enable_parallel: bool,
}

impl IncrementalProcessor {
    pub fn new() -> Self {
        Self {
            file_hashes: RwLock::new(HashMap::with_capacity(1000)), // Typical project has hundreds of files
            last_diagnostics: RwLock::new(HashMap::with_capacity(1000)),
            file_metadata: RwLock::new(HashMap::with_capacity(1000)),
            parallel_chunk_size: 100,
            enable_parallel: true,
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.parallel_chunk_size = size;
        self
    }

    pub fn with_parallel(mut self, enabled: bool) -> Self {
        self.enable_parallel = enabled;
        self
    }

    pub async fn detect_changed_files<P: AsRef<Path>>(&self, files: &[P]) -> Result<Vec<PathBuf>> {
        let start = std::time::Instant::now();

        let file_paths: Vec<PathBuf> = files.iter().map(|p| p.as_ref().to_path_buf()).collect();

        let changed_files = if self.enable_parallel && file_paths.len() > self.parallel_chunk_size {
            self.detect_changed_files_parallel(&file_paths).await?
        } else {
            self.detect_changed_files_sequential(&file_paths).await?
        };

        let elapsed = start.elapsed();
        debug!(
            "Change detection completed in {:?} for {} files, {} changed",
            elapsed,
            file_paths.len(),
            changed_files.len()
        );

        Ok(changed_files)
    }

    async fn detect_changed_files_sequential(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut changed_files = Vec::with_capacity(files.len() / 4); // Assume ~25% of files change
        let file_hashes = self.file_hashes.read().await;
        let file_metadata = self.file_metadata.read().await;

        for file_path in files {
            if self
                .is_file_changed(&file_hashes, &file_metadata, file_path)
                .await?
            {
                changed_files.push(file_path.clone());
            }
        }

        Ok(changed_files)
    }

    async fn detect_changed_files_parallel(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let file_hashes = self.file_hashes.read().await.clone();
        let file_metadata = self.file_metadata.read().await.clone();

        let chunk_results: Result<Vec<Vec<PathBuf>>> = files
            .par_chunks(self.parallel_chunk_size)
            .map(|chunk| {
                let mut chunk_changed = Vec::with_capacity(chunk.len() / 4); // Assume ~25% of files change
                for file_path in chunk {
                    if self.is_file_changed_sync(&file_hashes, &file_metadata, file_path)? {
                        chunk_changed.push(file_path.clone());
                    }
                }
                Ok(chunk_changed)
            })
            .collect();

        let changed_files = chunk_results?.into_iter().flatten().collect::<Vec<_>>();

        Ok(changed_files)
    }

    async fn is_file_changed(
        &self,
        file_hashes: &HashMap<PathBuf, FileHash>,
        file_metadata: &HashMap<PathBuf, SystemTime>,
        file_path: &Path,
    ) -> Result<bool> {
        if !file_path.exists() {
            // Non-existent files are considered "changed" (new files to process)
            return Ok(true);
        }

        let metadata = fs::metadata(file_path)?;
        let modified_time = metadata.modified()?;

        if let Some(cached_time) = file_metadata.get(file_path) {
            if modified_time <= *cached_time {
                return Ok(false);
            }
        }

        let current_hash = FileHash::from_file(file_path)?;
        let is_changed = match file_hashes.get(file_path) {
            Some(cached_hash) => current_hash != *cached_hash,
            None => true,
        };

        Ok(is_changed)
    }

    fn is_file_changed_sync(
        &self,
        file_hashes: &HashMap<PathBuf, FileHash>,
        file_metadata: &HashMap<PathBuf, SystemTime>,
        file_path: &Path,
    ) -> Result<bool> {
        if !file_path.exists() {
            // Non-existent files are considered "changed" (new files to process)
            return Ok(true);
        }

        let metadata = fs::metadata(file_path)?;
        let modified_time = metadata.modified()?;

        if let Some(cached_time) = file_metadata.get(file_path) {
            if modified_time <= *cached_time {
                return Ok(false);
            }
        }

        let current_hash = FileHash::from_file(file_path)?;
        let is_changed = match file_hashes.get(file_path) {
            Some(cached_hash) => current_hash != *cached_hash,
            None => true,
        };

        Ok(is_changed)
    }

    pub async fn update_file_cache(
        &self,
        file_path: PathBuf,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<()> {
        // Skip caching for non-existent files (e.g., test files)
        if !file_path.exists() {
            debug!(
                "Skipping cache update for non-existent file: {:?}",
                file_path
            );
            return Ok(());
        }

        let hash = FileHash::from_file(&file_path)?;
        let metadata = fs::metadata(&file_path)?;
        let modified_time = metadata.modified()?;

        {
            let mut file_hashes = self.file_hashes.write().await;
            file_hashes.insert(file_path.clone(), hash);
        }

        {
            let mut last_diagnostics = self.last_diagnostics.write().await;
            last_diagnostics.insert(file_path.clone(), diagnostics);
        }

        {
            let mut file_metadata = self.file_metadata.write().await;
            file_metadata.insert(file_path, modified_time);
        }

        Ok(())
    }

    pub async fn get_cached_diagnostics(&self, file_path: &Path) -> Option<Vec<Diagnostic>> {
        let last_diagnostics = self.last_diagnostics.read().await;
        last_diagnostics.get(file_path).cloned()
    }

    pub async fn merge_diagnostics(
        &self,
        changed_files: &[PathBuf],
        new_diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
    ) -> Result<Vec<Diagnostic>> {
        let mut all_diagnostics = Vec::new();
        let cached_diagnostics = self.last_diagnostics.read().await;

        for (file_path, diagnostics) in new_diagnostics {
            all_diagnostics.extend(diagnostics.clone());
            self.update_file_cache(file_path, diagnostics).await?;
        }

        for (file_path, diagnostics) in cached_diagnostics.iter() {
            if !changed_files.contains(file_path) {
                all_diagnostics.extend(diagnostics.clone());
            }
        }

        Ok(all_diagnostics)
    }

    pub async fn process_files_incrementally<F, Fut>(
        &self,
        files: &[PathBuf],
        processor: F,
    ) -> Result<(Vec<Diagnostic>, ProcessingStats)>
    where
        F: Fn(Vec<PathBuf>) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<HashMap<PathBuf, Vec<Diagnostic>>>> + Send,
    {
        let start_time = std::time::Instant::now();

        let changed_files = self.detect_changed_files(files).await?;

        let new_diagnostics = if !changed_files.is_empty() {
            info!(
                "Processing {} changed files out of {} total",
                changed_files.len(),
                files.len()
            );
            processor(changed_files.clone()).await?
        } else {
            HashMap::new()
        };

        let all_diagnostics = self
            .merge_diagnostics(&changed_files, new_diagnostics)
            .await?;

        let processing_time = start_time.elapsed();
        let cache_hit_rate = if files.is_empty() {
            0.0
        } else {
            (files.len() - changed_files.len()) as f32 / files.len() as f32
        };

        let stats = ProcessingStats {
            total_files: files.len(),
            changed_files: changed_files.len(),
            cached_files: files.len() - changed_files.len(),
            processing_time,
            cache_hit_rate,
        };

        info!(
            "Incremental processing complete: {}/{} files cached ({}% hit rate), {:?}",
            stats.cached_files,
            stats.total_files,
            (stats.cache_hit_rate * 100.0) as u32,
            stats.processing_time
        );

        Ok((all_diagnostics, stats))
    }

    pub async fn invalidate_file(&self, file_path: &Path) -> Result<()> {
        {
            let mut file_hashes = self.file_hashes.write().await;
            file_hashes.remove(file_path);
        }

        {
            let mut last_diagnostics = self.last_diagnostics.write().await;
            last_diagnostics.remove(file_path);
        }

        {
            let mut file_metadata = self.file_metadata.write().await;
            file_metadata.remove(file_path);
        }

        debug!("Invalidated cache for file: {}", file_path.display());
        Ok(())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        {
            let mut file_hashes = self.file_hashes.write().await;
            file_hashes.clear();
        }

        {
            let mut last_diagnostics = self.last_diagnostics.write().await;
            last_diagnostics.clear();
        }

        {
            let mut file_metadata = self.file_metadata.write().await;
            file_metadata.clear();
        }

        info!("Incremental processor cache cleared");
        Ok(())
    }

    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let file_hashes = self.file_hashes.read().await;
        let last_diagnostics = self.last_diagnostics.read().await;

        // Count total number of diagnostics across all files
        let total_diagnostics = last_diagnostics
            .values()
            .map(|diags| diags.len())
            .sum::<usize>();

        (file_hashes.len(), total_diagnostics)
    }
}

impl Default for IncrementalProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_file_hash_creation() {
        let content = b"hello world";
        let hash1 = FileHash::new(content);
        let hash2 = FileHash::new(content);
        assert_eq!(hash1, hash2);

        let different_content = b"hello world!";
        let hash3 = FileHash::new(different_content);
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_incremental_detection() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "initial content")?;

        let processor = IncrementalProcessor::new();

        let changed = processor.detect_changed_files(&[&file_path]).await?;
        assert_eq!(changed.len(), 1);

        processor
            .update_file_cache(file_path.clone(), vec![])
            .await?;

        let changed = processor.detect_changed_files(&[&file_path]).await?;
        assert_eq!(changed.len(), 0);

        fs::write(&file_path, "modified content")?;

        let changed = processor.detect_changed_files(&[&file_path]).await?;
        assert_eq!(changed.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_operations() -> Result<()> {
        let processor = IncrementalProcessor::new();
        let file_path = PathBuf::from("/fake/path.rs");

        assert!(processor.get_cached_diagnostics(&file_path).await.is_none());

        let _diagnostics: Vec<Diagnostic> = vec![];

        let (hash_count, diag_count) = processor.get_cache_stats().await;
        assert_eq!(hash_count, 0);
        assert_eq!(diag_count, 0);

        processor.clear_cache().await?;
        let (hash_count, diag_count) = processor.get_cache_stats().await;
        assert_eq!(hash_count, 0);
        assert_eq!(diag_count, 0);

        Ok(())
    }
}
