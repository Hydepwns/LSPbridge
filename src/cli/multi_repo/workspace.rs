//! Workspace synchronization and coordination utilities
//!
//! This module provides utilities for synchronizing workspaces across multiple repositories,
//! managing workspace state, and coordinating operations across the multi-repository system.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::multi_repo::{MultiRepoContext, RepositoryInfo};

/// Workspace synchronization manager
pub struct WorkspaceSynchronizer {
    /// Root workspace directory
    workspace_root: PathBuf,
    
    /// Synchronization mode
    sync_mode: SyncMode,
    
    /// Repositories to synchronize
    repositories: Vec<String>,
    
    /// Sync configuration
    config: WorkspaceSyncConfig,
}

impl WorkspaceSynchronizer {
    /// Create a new workspace synchronizer
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            sync_mode: SyncMode::Incremental,
            repositories: Vec::new(),
            config: WorkspaceSyncConfig::default(),
        }
    }

    /// Configure synchronization mode
    pub fn with_sync_mode(mut self, mode: SyncMode) -> Self {
        self.sync_mode = mode;
        self
    }

    /// Add repositories to synchronization
    pub fn with_repositories(mut self, repos: Vec<String>) -> Self {
        self.repositories = repos;
        self
    }

    /// Configure sync settings
    pub fn with_config(mut self, config: WorkspaceSyncConfig) -> Self {
        self.config = config;
        self
    }

    /// Synchronize workspace with registered repositories
    pub async fn synchronize_workspace(
        &self,
        context: &MultiRepoContext,
    ) -> Result<WorkspaceSyncResult> {
        let mut sync_result = WorkspaceSyncResult {
            synchronized_repos: Vec::new(),
            failed_repos: Vec::new(),
            sync_statistics: SyncStatistics::default(),
            warnings: Vec::new(),
        };

        // Get repositories to sync
        let repos_to_sync = if self.repositories.is_empty() {
            context.list_repositories(false).await?
        } else {
            self.get_specific_repositories(context).await?
        };

        // Create workspace structure
        self.ensure_workspace_structure().await?;

        // Sync each repository
        for repo in repos_to_sync {
            match self.sync_repository(&repo).await {
                Ok(repo_sync) => {
                    sync_result.synchronized_repos.push(repo_sync);
                    sync_result.sync_statistics.successful_syncs += 1;
                }
                Err(e) => {
                    sync_result.failed_repos.push(FailedSync {
                        repository_id: repo.id.clone(),
                        error: e.to_string(),
                    });
                    sync_result.sync_statistics.failed_syncs += 1;
                }
            }
        }

        // Generate workspace index
        self.generate_workspace_index(&sync_result).await?;

        // Update sync metadata
        sync_result.sync_statistics.total_files_synced = sync_result
            .synchronized_repos
            .iter()
            .map(|r| r.files_synced)
            .sum();

        sync_result.sync_statistics.total_size_synced = sync_result
            .synchronized_repos
            .iter()
            .map(|r| r.size_synced)
            .sum();

        Ok(sync_result)
    }

    /// Sync a single repository
    async fn sync_repository(&self, repo: &RepositoryInfo) -> Result<RepositorySyncResult> {
        let repo_workspace_path = self.workspace_root.join(&repo.name);
        
        // Create repository workspace directory
        fs::create_dir_all(&repo_workspace_path).await
            .context("Failed to create repository workspace directory")?;

        let mut sync_result = RepositorySyncResult {
            repository_id: repo.id.clone(),
            repository_name: repo.name.clone(),
            files_synced: 0,
            size_synced: 0,
            sync_mode: self.sync_mode.clone(),
            last_sync: chrono::Utc::now(),
        };

        match self.sync_mode {
            SyncMode::Full => {
                sync_result = self.perform_full_sync(repo, &repo_workspace_path, sync_result).await?;
            }
            SyncMode::Incremental => {
                sync_result = self.perform_incremental_sync(repo, &repo_workspace_path, sync_result).await?;
            }
            SyncMode::SymbolicLinks => {
                sync_result = self.perform_symlink_sync(repo, &repo_workspace_path, sync_result).await?;
            }
        }

        // Create repository metadata
        self.create_repository_metadata(repo, &repo_workspace_path).await?;

        Ok(sync_result)
    }

    /// Perform full synchronization (copy all files)
    async fn perform_full_sync(
        &self,
        repo: &RepositoryInfo,
        workspace_path: &Path,
        mut sync_result: RepositorySyncResult,
    ) -> Result<RepositorySyncResult> {
        // Copy files based on configuration
        let source_patterns = &self.config.include_patterns;
        let exclude_patterns = &self.config.exclude_patterns;

        for pattern in source_patterns {
            let matches = self.find_matching_files(&repo.path, pattern, exclude_patterns).await?;
            
            for source_file in matches {
                let relative_path = source_file.strip_prefix(&repo.path)?;
                let target_file = workspace_path.join(relative_path);

                // Ensure target directory exists
                if let Some(parent) = target_file.parent() {
                    fs::create_dir_all(parent).await?;
                }

                // Copy file
                fs::copy(&source_file, &target_file).await
                    .context("Failed to copy file to workspace")?;

                sync_result.files_synced += 1;
                
                // Update size
                if let Ok(metadata) = fs::metadata(&source_file).await {
                    sync_result.size_synced += metadata.len();
                }
            }
        }

        Ok(sync_result)
    }

    /// Perform incremental synchronization (only changed files)
    async fn perform_incremental_sync(
        &self,
        repo: &RepositoryInfo,
        workspace_path: &Path,
        mut sync_result: RepositorySyncResult,
    ) -> Result<RepositorySyncResult> {
        // Get last sync timestamp
        let last_sync = self.get_last_sync_timestamp(&repo.id).await?;
        
        let source_patterns = &self.config.include_patterns;
        let exclude_patterns = &self.config.exclude_patterns;

        for pattern in source_patterns {
            let matches = self.find_matching_files(&repo.path, pattern, exclude_patterns).await?;
            
            for source_file in matches {
                // Check if file was modified since last sync
                if let Ok(metadata) = fs::metadata(&source_file).await {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
                        
                        if modified_time > last_sync {
                            let relative_path = source_file.strip_prefix(&repo.path)?;
                            let target_file = workspace_path.join(relative_path);

                            // Ensure target directory exists
                            if let Some(parent) = target_file.parent() {
                                fs::create_dir_all(parent).await?;
                            }

                            // Copy file
                            fs::copy(&source_file, &target_file).await
                                .context("Failed to copy file to workspace")?;

                            sync_result.files_synced += 1;
                            sync_result.size_synced += metadata.len();
                        }
                    }
                }
            }
        }

        Ok(sync_result)
    }

    /// Perform symbolic link synchronization
    async fn perform_symlink_sync(
        &self,
        repo: &RepositoryInfo,
        workspace_path: &Path,
        mut sync_result: RepositorySyncResult,
    ) -> Result<RepositorySyncResult> {
        // Create symbolic link to the entire repository
        if workspace_path.exists() {
            fs::remove_dir_all(workspace_path).await?;
        }

        #[cfg(unix)]
        {
            tokio::fs::symlink(&repo.path, workspace_path).await
                .context("Failed to create symbolic link")?;
        }

        #[cfg(windows)]
        {
            tokio::fs::symlink_dir(&repo.path, workspace_path).await
                .context("Failed to create symbolic link")?;
        }

        sync_result.files_synced = 1; // One symlink created
        sync_result.size_synced = 0; // Symlinks don't take significant space

        Ok(sync_result)
    }

    /// Find files matching patterns
    async fn find_matching_files(
        &self,
        repo_path: &Path,
        include_pattern: &str,
        exclude_patterns: &[String],
    ) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let mut matching_files = Vec::new();

        for entry in WalkDir::new(repo_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Check include pattern
            if self.matches_pattern(&path_str, include_pattern) {
                // Check exclude patterns
                let should_exclude = exclude_patterns.iter()
                    .any(|pattern| self.matches_pattern(&path_str, pattern));

                if !should_exclude {
                    matching_files.push(path.to_path_buf());
                }
            }
        }

        Ok(matching_files)
    }

    /// Check if path matches pattern (simple glob-like matching)
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        // Simple implementation - could be enhanced with proper glob matching
        if pattern == "**" || pattern == "*" {
            return true;
        }

        if pattern.starts_with("*.") {
            let extension = &pattern[2..];
            return path.ends_with(extension);
        }

        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return path.starts_with(parts[0]) && path.ends_with(parts[1]);
            }
        }

        path.contains(pattern)
    }

    /// Get repositories by specific IDs
    async fn get_specific_repositories(
        &self,
        context: &MultiRepoContext,
    ) -> Result<Vec<RepositoryInfo>> {
        let all_repos = context.list_repositories(false).await?;
        let filtered_repos = all_repos
            .into_iter()
            .filter(|repo| self.repositories.contains(&repo.id) || self.repositories.contains(&repo.name))
            .collect();

        Ok(filtered_repos)
    }

    /// Ensure workspace directory structure exists
    async fn ensure_workspace_structure(&self) -> Result<()> {
        fs::create_dir_all(&self.workspace_root).await
            .context("Failed to create workspace root directory")?;

        // Create standard workspace directories
        let standard_dirs = ["cache", "logs", "metadata", "temp"];
        for dir in &standard_dirs {
            fs::create_dir_all(self.workspace_root.join(dir)).await
                .context("Failed to create workspace subdirectory")?;
        }

        Ok(())
    }

    /// Generate workspace index file
    async fn generate_workspace_index(&self, sync_result: &WorkspaceSyncResult) -> Result<()> {
        let index = WorkspaceIndex {
            workspace_root: self.workspace_root.clone(),
            last_sync: chrono::Utc::now(),
            synchronized_repositories: sync_result.synchronized_repos.clone(),
            sync_configuration: self.config.clone(),
            statistics: sync_result.sync_statistics.clone(),
        };

        let index_path = self.workspace_root.join("workspace_index.json");
        let index_json = serde_json::to_string_pretty(&index)?;
        fs::write(index_path, index_json).await
            .context("Failed to write workspace index")?;

        Ok(())
    }

    /// Create repository metadata file
    async fn create_repository_metadata(
        &self,
        repo: &RepositoryInfo,
        workspace_path: &Path,
    ) -> Result<()> {
        let metadata = RepositoryWorkspaceMetadata {
            original_path: repo.path.clone(),
            workspace_path: workspace_path.to_path_buf(),
            repository_info: repo.clone(),
            sync_mode: self.sync_mode.clone(),
            last_sync: chrono::Utc::now(),
        };

        let metadata_path = workspace_path.join(".lspbridge_metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(metadata_path, metadata_json).await
            .context("Failed to write repository metadata")?;

        Ok(())
    }

    /// Get last sync timestamp for a repository
    async fn get_last_sync_timestamp(&self, repo_id: &str) -> Result<chrono::DateTime<chrono::Utc>> {
        let metadata_path = self.workspace_root.join("metadata").join(format!("{}.json", repo_id));
        
        if metadata_path.exists() {
            let content = fs::read_to_string(metadata_path).await?;
            let metadata: RepositorySyncMetadata = serde_json::from_str(&content)?;
            Ok(metadata.last_sync)
        } else {
            // Return Unix epoch if no previous sync
            Ok(chrono::DateTime::from_timestamp(0, 0).unwrap_or_else(|| chrono::Utc::now()))
        }
    }
}

/// Workspace synchronization modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    /// Copy all files to workspace
    Full,
    /// Only copy changed files
    Incremental,
    /// Create symbolic links to original files
    SymbolicLinks,
}

/// Workspace synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSyncConfig {
    /// File patterns to include in sync
    pub include_patterns: Vec<String>,
    
    /// File patterns to exclude from sync
    pub exclude_patterns: Vec<String>,
    
    /// Maximum file size to sync (in bytes)
    pub max_file_size: Option<u64>,
    
    /// Whether to preserve file timestamps
    pub preserve_timestamps: bool,
    
    /// Whether to sync hidden files
    pub sync_hidden_files: bool,
}

impl Default for WorkspaceSyncConfig {
    fn default() -> Self {
        Self {
            include_patterns: vec![
                "*.rs".to_string(),
                "*.ts".to_string(),
                "*.js".to_string(),
                "*.py".to_string(),
                "*.go".to_string(),
                "*.java".to_string(),
                "*.cpp".to_string(),
                "*.c".to_string(),
                "*.h".to_string(),
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "*.md".to_string(),
            ],
            exclude_patterns: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
            ],
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            preserve_timestamps: true,
            sync_hidden_files: false,
        }
    }
}

/// Result of workspace synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSyncResult {
    pub synchronized_repos: Vec<RepositorySyncResult>,
    pub failed_repos: Vec<FailedSync>,
    pub sync_statistics: SyncStatistics,
    pub warnings: Vec<String>,
}

/// Result of synchronizing a single repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySyncResult {
    pub repository_id: String,
    pub repository_name: String,
    pub files_synced: usize,
    pub size_synced: u64,
    pub sync_mode: SyncMode,
    pub last_sync: chrono::DateTime<chrono::Utc>,
}

/// Failed synchronization record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedSync {
    pub repository_id: String,
    pub error: String,
}

/// Synchronization statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStatistics {
    pub successful_syncs: usize,
    pub failed_syncs: usize,
    pub total_files_synced: usize,
    pub total_size_synced: u64,
}

/// Workspace index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    pub workspace_root: PathBuf,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub synchronized_repositories: Vec<RepositorySyncResult>,
    pub sync_configuration: WorkspaceSyncConfig,
    pub statistics: SyncStatistics,
}

/// Repository workspace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryWorkspaceMetadata {
    pub original_path: PathBuf,
    pub workspace_path: PathBuf,
    pub repository_info: RepositoryInfo,
    pub sync_mode: SyncMode,
    pub last_sync: chrono::DateTime<chrono::Utc>,
}

/// Repository sync metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySyncMetadata {
    pub repository_id: String,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub files_synced: usize,
    pub sync_mode: SyncMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_workspace_synchronizer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let synchronizer = WorkspaceSynchronizer::new(workspace_path.clone())
            .with_sync_mode(SyncMode::Incremental)
            .with_repositories(vec!["repo1".to_string(), "repo2".to_string()]);

        assert_eq!(synchronizer.workspace_root, workspace_path);
        assert!(matches!(synchronizer.sync_mode, SyncMode::Incremental));
        assert_eq!(synchronizer.repositories.len(), 2);
    }

    #[test]
    fn test_pattern_matching() {
        let synchronizer = WorkspaceSynchronizer::new(PathBuf::from("/tmp"));

        assert!(synchronizer.matches_pattern("file.rs", "*.rs"));
        assert!(synchronizer.matches_pattern("src/main.rs", "*.rs"));
        assert!(!synchronizer.matches_pattern("file.js", "*.rs"));
        assert!(synchronizer.matches_pattern("anything", "**"));
        assert!(synchronizer.matches_pattern("src/lib.rs", "*lib*"));
    }

    #[tokio::test]
    async fn test_workspace_structure_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let synchronizer = WorkspaceSynchronizer::new(workspace_path.clone());
        synchronizer.ensure_workspace_structure().await.unwrap();

        assert!(workspace_path.join("cache").exists());
        assert!(workspace_path.join("logs").exists());
        assert!(workspace_path.join("metadata").exists());
        assert!(workspace_path.join("temp").exists());
    }

    #[tokio::test]
    async fn test_find_matching_files() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create test files
        fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(repo_path.join("lib.rs"), "// lib").unwrap();
        fs::write(repo_path.join("test.js"), "console.log").unwrap();
        fs::create_dir_all(repo_path.join("src")).unwrap();
        fs::write(repo_path.join("src/helper.rs"), "// helper").unwrap();

        let synchronizer = WorkspaceSynchronizer::new(PathBuf::from("/tmp"));
        let matches = synchronizer
            .find_matching_files(repo_path, "*.rs", &vec!["target/**".to_string()])
            .await
            .unwrap();

        assert_eq!(matches.len(), 3); // main.rs, lib.rs, src/helper.rs
    }

    #[test]
    fn test_sync_config_default() {
        let config = WorkspaceSyncConfig::default();
        
        assert!(config.include_patterns.contains(&"*.rs".to_string()));
        assert!(config.exclude_patterns.contains(&"target/**".to_string()));
        assert_eq!(config.max_file_size, Some(10 * 1024 * 1024));
        assert!(config.preserve_timestamps);
        assert!(!config.sync_hidden_files);
    }
}