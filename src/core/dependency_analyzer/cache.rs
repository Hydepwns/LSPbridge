use crate::core::dependency_analyzer::types::FileDependencies;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// Manages caching of dependency analysis results
pub struct DependencyCache {
    /// Cache of file dependency graphs
    dependency_cache: HashMap<PathBuf, FileDependencies>,
    /// Cache of parsed syntax trees
    ast_cache: HashMap<PathBuf, tree_sitter::Tree>,
}

impl Default for DependencyCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyCache {
    pub fn new() -> Self {
        Self {
            dependency_cache: HashMap::new(),
            ast_cache: HashMap::new(),
        }
    }
    
    /// Get cached dependencies if still valid
    pub fn get_dependencies(&self, file_path: &Path) -> Option<&FileDependencies> {
        if let Some(cached) = self.dependency_cache.get(file_path) {
            // Check if file has been modified since cache
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    if cached.last_modified >= modified {
                        return Some(cached);
                    }
                }
            }
        }
        None
    }
    
    /// Cache dependency analysis results
    pub fn cache_dependencies(&mut self, file_path: PathBuf, dependencies: FileDependencies) {
        self.dependency_cache.insert(file_path, dependencies);
    }
    
    /// Get cached AST if available
    pub fn get_ast(&self, file_path: &Path) -> Option<&tree_sitter::Tree> {
        self.ast_cache.get(file_path)
    }
    
    /// Cache parsed AST
    pub fn cache_ast(&mut self, file_path: PathBuf, tree: tree_sitter::Tree) {
        self.ast_cache.insert(file_path, tree);
    }
    
    /// Clear cache for a specific file
    pub fn invalidate(&mut self, file_path: &Path) {
        self.dependency_cache.remove(file_path);
        self.ast_cache.remove(file_path);
    }
    
    /// Clear all caches
    pub fn clear(&mut self) {
        self.dependency_cache.clear();
        self.ast_cache.clear();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            dependency_entries: self.dependency_cache.len(),
            ast_entries: self.ast_cache.len(),
            total_size_estimate: self.estimate_size(),
        }
    }
    
    fn estimate_size(&self) -> usize {
        // Rough estimate of memory usage
        let dep_size = self.dependency_cache.len() * std::mem::size_of::<FileDependencies>();
        let ast_size = self.ast_cache.len() * 1024; // Rough estimate for AST size
        dep_size + ast_size
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub dependency_entries: usize,
    pub ast_entries: usize,
    pub total_size_estimate: usize,
}