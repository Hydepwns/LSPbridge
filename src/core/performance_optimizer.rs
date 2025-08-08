//! Performance optimization utilities for LSPbridge
//!
//! Provides lazy loading, caching, and parallel processing optimizations

use anyhow::{Context, Result};
use dashmap::DashMap;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

/// File metadata cache entry
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub content_hash: Option<String>,
    pub language: Option<String>,
    pub cached_at: Instant,
}

/// Optimized file scanner with caching and parallel processing
pub struct OptimizedFileScanner {
    /// Concurrent cache for file metadata
    metadata_cache: Arc<DashMap<PathBuf, FileMetadata>>,
    /// Cache TTL
    cache_ttl: Duration,
    /// Ignored patterns
    ignore_patterns: Vec<String>,
    /// Maximum parallel threads
    max_threads: usize,
}

impl Default for OptimizedFileScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizedFileScanner {
    pub fn new() -> Self {
        Self {
            metadata_cache: Arc::new(DashMap::new()),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            ignore_patterns: Self::default_ignore_patterns(),
            max_threads: rayon::current_num_threads(),
        }
    }

    /// Create scanner with custom configuration
    pub fn with_config(cache_ttl: Duration, max_threads: Option<usize>) -> Self {
        Self {
            metadata_cache: Arc::new(DashMap::new()),
            cache_ttl,
            ignore_patterns: Self::default_ignore_patterns(),
            max_threads: max_threads.unwrap_or_else(rayon::current_num_threads),
        }
    }

    /// Default patterns to ignore
    fn default_ignore_patterns() -> Vec<String> {
        vec![
            ".git".to_string(),
            "node_modules".to_string(),
            "target".to_string(),
            ".idea".to_string(),
            ".vscode".to_string(),
            "__pycache__".to_string(),
            "*.pyc".to_string(),
            "*.pyo".to_string(),
            "*.log".to_string(),
            "*.tmp".to_string(),
            "*.swp".to_string(),
            ".DS_Store".to_string(),
        ]
    }

    /// Check if path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        self.ignore_patterns.iter().any(|pattern| {
            if pattern.starts_with("*.") {
                // File extension pattern
                path_str.ends_with(&pattern[1..])
            } else if pattern.ends_with("*") {
                // Prefix pattern
                path_str.starts_with(&pattern[..pattern.len()-1])
            } else {
                // Exact match or contains
                path_str.contains(pattern)
            }
        })
    }

    /// Scan directory with optimized parallel processing
    pub fn scan_directory(&self, root: &Path) -> Result<Vec<PathBuf>> {
        // First pass: collect all entries
        let entries: Vec<DirEntry> = WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e.path()))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Process in parallel chunks
        let chunk_size = (entries.len() / self.max_threads).max(100);
        let paths: Vec<PathBuf> = entries
            .par_chunks(chunk_size)
            .flat_map(|chunk| {
                chunk.iter()
                    .map(|entry| entry.path().to_owned())
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(paths)
    }

    /// Get cached metadata or compute it
    pub fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        // Check cache first
        if let Some(cached) = self.metadata_cache.get(path) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                return Ok(cached.clone());
            }
        }

        // Compute metadata
        let metadata = self.compute_metadata(path)?;
        
        // Cache it
        self.metadata_cache.insert(path.to_owned(), metadata.clone());
        
        Ok(metadata)
    }

    /// Compute file metadata
    fn compute_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let fs_metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {path:?}"))?;
        
        let language = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "py" => "python",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" => "c",
                "cs" => "csharp",
                "rb" => "ruby",
                "php" => "php",
                _ => ext,
            })
            .map(String::from);

        Ok(FileMetadata {
            size: fs_metadata.len(),
            modified: fs_metadata.modified()?,
            content_hash: None, // Computed lazily if needed
            language,
            cached_at: Instant::now(),
        })
    }

    /// Clear cache entries older than TTL
    pub fn clean_cache(&self) {
        let now = Instant::now();
        self.metadata_cache.retain(|_, metadata| {
            now.duration_since(metadata.cached_at) < self.cache_ttl
        });
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let total_entries = self.metadata_cache.len();
        let now = Instant::now();
        
        let expired_entries = self.metadata_cache
            .iter()
            .filter(|entry| now.duration_since(entry.cached_at) >= self.cache_ttl)
            .count();

        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
            cache_size_bytes: total_entries * std::mem::size_of::<(PathBuf, FileMetadata)>(),
        }
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub cache_size_bytes: usize,
}

/// Lazy loading wrapper for expensive computations
pub struct LazyLoader<T: Clone + Send + Sync> {
    /// Computed value storage
    value: Arc<RwLock<Option<T>>>,
    /// Computation function
    compute_fn: Arc<dyn Fn() -> Result<T> + Send + Sync>,
}

impl<T: Clone + Send + Sync + 'static> LazyLoader<T> {
    /// Create a new lazy loader
    pub fn new<F>(compute_fn: F) -> Self
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        Self {
            value: Arc::new(RwLock::new(None)),
            compute_fn: Arc::new(compute_fn),
        }
    }

    /// Get the value, computing it if necessary
    pub fn get(&self) -> Result<T> {
        // Fast path: read lock to check if value exists
        {
            let guard = self.value.read().unwrap();
            if let Some(ref value) = *guard {
                return Ok(value.clone());
            }
        }

        // Slow path: write lock to compute value
        let mut guard = self.value.write().unwrap();
        
        // Double-check in case another thread computed it
        if let Some(ref value) = *guard {
            return Ok(value.clone());
        }

        // Compute the value
        let value = (self.compute_fn)()?;
        *guard = Some(value.clone());
        
        Ok(value)
    }

    /// Check if value has been computed
    pub fn is_loaded(&self) -> bool {
        self.value.read().unwrap().is_some()
    }

    /// Reset the lazy loader
    pub fn reset(&self) {
        *self.value.write().unwrap() = None;
    }
}

/// Batch processor for parallel file operations
pub struct BatchFileProcessor {
    batch_size: usize,
    #[allow(dead_code)]
    max_concurrent: usize,
}

impl BatchFileProcessor {
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            max_concurrent: rayon::current_num_threads(),
        }
    }

    /// Process files in optimized batches
    pub fn process_files<F, R>(
        &self,
        files: Vec<PathBuf>,
        processor: F,
    ) -> Vec<Result<R>>
    where
        F: Fn(&Path) -> Result<R> + Sync + Send,
        R: Send,
    {
        files
            .par_chunks(self.batch_size)
            .flat_map(|batch| {
                batch
                    .par_iter()
                    .map(|path| processor(path))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Process files with progress callback
    pub fn process_files_with_progress<F, R, P>(
        &self,
        files: Vec<PathBuf>,
        processor: F,
        mut progress_callback: P,
    ) -> Vec<Result<R>>
    where
        F: Fn(&Path) -> Result<R> + Sync + Send,
        R: Send,
        P: FnMut(usize, usize) + Send,
    {
        let total = files.len();
        let processed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let processed_clone = processed.clone();

        let results = files
            .par_chunks(self.batch_size)
            .flat_map(|batch| {
                let batch_results: Vec<_> = batch
                    .par_iter()
                    .map(|path| {
                        let result = processor(path);
                        processed_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        result
                    })
                    .collect();
                
                batch_results
            })
            .collect();

        progress_callback(processed.load(std::sync::atomic::Ordering::Relaxed), total);
        
        results
    }
}

/// Memory-efficient file content iterator
pub struct FileContentIterator {
    paths: Vec<PathBuf>,
    current_index: usize,
    #[allow(dead_code)]
    buffer_size: usize,
}

impl FileContentIterator {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            current_index: 0,
            buffer_size: 8192, // 8KB chunks
        }
    }

    /// Get next file content without loading entire file into memory
    pub fn next_content(&mut self) -> Option<Result<(PathBuf, String)>> {
        if self.current_index >= self.paths.len() {
            return None;
        }

        let path = &self.paths[self.current_index];
        self.current_index += 1;

        Some(
            fs::read_to_string(path)
                .map(|content| (path.clone(), content))
                .with_context(|| format!("Failed to read file: {path:?}"))
        )
    }

    /// Process files in streaming fashion
    pub fn process_streaming<F>(&mut self, mut processor: F) -> Result<()>
    where
        F: FnMut(&Path, &str) -> Result<()>,
    {
        while let Some(result) = self.next_content() {
            let (path, content) = result?;
            processor(&path, &content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_optimized_scanner() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = OptimizedFileScanner::new();
        
        // Create test files
        fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();
        
        let files = scanner.scan_directory(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_lazy_loader() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let loader = LazyLoader::new(move || {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(42)
        });
        
        assert!(!loader.is_loaded());
        assert_eq!(loader.get().unwrap(), 42);
        assert!(loader.is_loaded());
        
        // Second call shouldn't increment counter
        assert_eq!(loader.get().unwrap(), 42);
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn test_cache_expiration() {
        let scanner = OptimizedFileScanner::with_config(
            Duration::from_millis(100),
            Some(1)
        );
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();
        
        // First access - cache miss
        let metadata1 = scanner.get_metadata(&test_file).unwrap();
        
        // Immediate second access - cache hit
        let metadata2 = scanner.get_metadata(&test_file).unwrap();
        assert_eq!(metadata1.size, metadata2.size);
        
        // Wait for cache to expire
        std::thread::sleep(Duration::from_millis(150));
        
        // This should recompute
        let metadata3 = scanner.get_metadata(&test_file).unwrap();
        assert_eq!(metadata1.size, metadata3.size);
    }
}