use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitFileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Staged,
    Conflict,
}

#[derive(Debug, Clone)]
pub struct GitFileInfo {
    pub path: PathBuf,
    pub status: GitFileStatus,
    pub last_commit_hash: Option<String>,
    pub last_modified: SystemTime,
    pub staged: bool,
}

#[derive(Debug, Clone)]
pub struct GitRepositoryInfo {
    pub root_path: PathBuf,
    pub current_branch: String,
    pub last_commit_hash: String,
    pub is_dirty: bool,
    pub ahead_behind: (usize, usize), // (ahead, behind) relative to remote
}

pub struct GitIntegration {
    repo_root: Option<PathBuf>,
    last_scan: RwLock<Option<SystemTime>>,
    file_status_cache: RwLock<HashMap<PathBuf, GitFileInfo>>,
    repo_info_cache: RwLock<Option<GitRepositoryInfo>>,
    scan_interval: Duration,
}

impl GitIntegration {
    pub async fn new() -> Result<Self> {
        let repo_root = Self::find_git_root().await?;

        let integration = Self {
            repo_root,
            last_scan: RwLock::new(None),
            file_status_cache: RwLock::new(HashMap::new()),
            repo_info_cache: RwLock::new(None),
            scan_interval: Duration::from_secs(30), // Scan every 30 seconds
        };

        // Initial scan
        integration.refresh_git_status().await?;

        Ok(integration)
    }

    pub async fn new_with_repo(repo_path: PathBuf) -> Result<Self> {
        // Verify this is actually a git repo
        let git_root = {
            let output = Command::new("git")
                .current_dir(&repo_path)
                .args(["rev-parse", "--show-toplevel"])
                .output()?;

            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                Some(PathBuf::from(path_str.trim()))
            } else {
                return Err(anyhow!("Not a git repository"));
            }
        };

        let integration = Self {
            repo_root: git_root,
            last_scan: RwLock::new(None),
            file_status_cache: RwLock::new(HashMap::new()),
            repo_info_cache: RwLock::new(None),
            scan_interval: Duration::from_secs(30), // Scan every 30 seconds
        };

        // Initial scan
        integration.refresh_git_status().await?;

        Ok(integration)
    }

    pub async fn find_git_root() -> Result<Option<PathBuf>> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let trimmed = path_str.trim();
                Ok(Some(PathBuf::from(trimmed)))
            }
            Ok(_) => {
                info!("Not in a Git repository");
                Ok(None)
            }
            Err(e) => {
                warn!("Git not available: {}", e);
                Ok(None)
            }
        }
    }

    pub async fn is_git_available(&self) -> bool {
        self.repo_root.is_some()
    }

    pub async fn get_repository_info(&self) -> Option<GitRepositoryInfo> {
        let cache = self.repo_info_cache.read().await;
        cache.clone()
    }

    pub async fn get_changed_files_since_commit(&self, commit_hash: &str) -> Result<Vec<PathBuf>> {
        let repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["diff", "--name-only", commit_hash])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| repo_root.join(line))
            .collect();

        Ok(files)
    }

    pub async fn get_modified_files(&self) -> Result<Vec<PathBuf>> {
        self.maybe_refresh().await?;

        let cache = self.file_status_cache.read().await;
        let modified_files = cache
            .values()
            .filter(|info| {
                matches!(
                    info.status,
                    GitFileStatus::Modified | GitFileStatus::Added | GitFileStatus::Untracked
                )
            })
            .map(|info| info.path.clone())
            .collect();

        Ok(modified_files)
    }

    pub async fn get_file_status(&self, file_path: &Path) -> Option<GitFileInfo> {
        self.maybe_refresh().await.ok()?;

        let cache = self.file_status_cache.read().await;
        cache.get(file_path).cloned()
    }

    pub async fn is_file_ignored(&self, file_path: &Path) -> Result<bool> {
        let repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        // Convert to relative path from repo root
        let relative_path_buf = if file_path.is_absolute() {
            // Canonicalize both paths to handle symlinks
            let file_canonical = file_path.canonicalize()?;
            let repo_canonical = repo_root.canonicalize()?;

            file_canonical
                .strip_prefix(&repo_canonical)
                .map_err(|_| anyhow!("File path is not within repository"))?
                .to_path_buf()
        } else {
            file_path.to_path_buf()
        };

        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["check-ignore", relative_path_buf.to_string_lossy().as_ref()])
            .output()?;

        // Git check-ignore returns 0 if file is ignored, 1 if not ignored
        Ok(output.status.success())
    }

    pub async fn get_last_commit_for_file(&self, file_path: &Path) -> Result<Option<String>> {
        let repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        let output = Command::new("git")
            .current_dir(repo_root)
            .args([
                "log",
                "-1",
                "--format=%H",
                "--",
                file_path.to_string_lossy().as_ref(),
            ])
            .output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let hash_str = String::from_utf8_lossy(&output.stdout);
        let hash = hash_str.trim();
        if hash.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hash.to_string()))
        }
    }

    pub async fn get_branch_info(&self) -> Result<(String, Option<(usize, usize)>)> {
        let repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        // Get current branch
        let branch_output = Command::new("git")
            .current_dir(repo_root)
            .args(["branch", "--show-current"])
            .output()?;

        if !branch_output.status.success() {
            return Err(anyhow!("Failed to get current branch"));
        }

        let current_branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Get ahead/behind info if remote exists
        let remote_output = Command::new("git")
            .current_dir(repo_root)
            .args([
                "rev-list",
                "--left-right",
                "--count",
                &format!("origin/{current_branch}...HEAD"),
            ])
            .output();

        let ahead_behind = if let Ok(output) = remote_output {
            if output.status.success() {
                let count_output = String::from_utf8_lossy(&output.stdout);
                let count_str = count_output.trim();
                let parts: Vec<&str> = count_str.split_whitespace().collect();
                if parts.len() == 2 {
                    let behind = parts[0].parse().unwrap_or(0);
                    let ahead = parts[1].parse().unwrap_or(0);
                    Some((ahead, behind))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok((current_branch, ahead_behind))
    }

    pub async fn refresh_git_status(&self) -> Result<()> {
        let _repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        debug!("Refreshing Git status");

        // Get repository info
        let repo_info = self.scan_repository_info().await?;

        // Get file statuses
        let file_statuses = self.scan_file_statuses().await?;

        // Update caches
        {
            let mut repo_cache = self.repo_info_cache.write().await;
            *repo_cache = Some(repo_info);
        }

        {
            let mut file_cache = self.file_status_cache.write().await;
            *file_cache = file_statuses;
        }

        {
            let mut last_scan = self.last_scan.write().await;
            *last_scan = Some(SystemTime::now());
        }

        debug!("Git status refresh completed");
        Ok(())
    }

    async fn maybe_refresh(&self) -> Result<()> {
        let last_scan = *self.last_scan.read().await;

        let should_refresh = match last_scan {
            Some(last) => last.elapsed().unwrap_or(Duration::MAX) >= self.scan_interval,
            None => true,
        };

        if should_refresh {
            self.refresh_git_status().await?;
        }

        Ok(())
    }

    async fn scan_repository_info(&self) -> Result<GitRepositoryInfo> {
        let repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        // Get current commit hash
        let hash_output = Command::new("git")
            .current_dir(repo_root)
            .args(["rev-parse", "HEAD"])
            .output()?;

        if !hash_output.status.success() {
            return Err(anyhow!("Failed to get current commit hash"));
        }

        let last_commit_hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        // Get branch info
        let (current_branch, ahead_behind) = self.get_branch_info().await?;

        // Check if repository is dirty
        let status_output = Command::new("git")
            .current_dir(repo_root)
            .args(["status", "--porcelain"])
            .output()?;

        let is_dirty = !status_output.stdout.is_empty();

        Ok(GitRepositoryInfo {
            root_path: repo_root.clone(),
            current_branch,
            last_commit_hash,
            is_dirty,
            ahead_behind: ahead_behind.unwrap_or((0, 0)),
        })
    }

    async fn scan_file_statuses(&self) -> Result<HashMap<PathBuf, GitFileInfo>> {
        let repo_root = self
            .repo_root
            .as_ref()
            .ok_or_else(|| anyhow!("No Git repository"))?;

        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["status", "--porcelain=v1", "-z"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Git status failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let mut file_statuses = HashMap::new();
        let status_text = String::from_utf8_lossy(&output.stdout);

        // Parse porcelain output (null-terminated entries)
        for entry in status_text.split('\0') {
            if entry.len() < 3 {
                continue;
            }

            let index_status = entry.chars().nth(0).unwrap_or(' ');
            let work_tree_status = entry.chars().nth(1).unwrap_or(' ');
            let file_path = &entry[3..];

            if file_path.is_empty() {
                continue;
            }

            let full_path = repo_root.join(file_path);
            let status = self.parse_git_status(index_status, work_tree_status);
            let staged = index_status != ' ';

            // Get last commit hash for this file
            let last_commit_hash = self
                .get_last_commit_for_file(&full_path)
                .await
                .ok()
                .flatten();

            // Get file modification time
            let last_modified = std::fs::metadata(&full_path)
                .and_then(|m| m.modified())
                .unwrap_or_else(|_| SystemTime::now());

            let file_info = GitFileInfo {
                path: full_path.clone(),
                status,
                last_commit_hash,
                last_modified,
                staged,
            };

            file_statuses.insert(full_path, file_info);
        }

        Ok(file_statuses)
    }

    pub fn parse_git_status(&self, index_status: char, work_tree_status: char) -> GitFileStatus {
        match (index_status, work_tree_status) {
            ('M', _) | (_, 'M') => GitFileStatus::Modified,
            ('A', _) => GitFileStatus::Added,
            ('D', _) | (_, 'D') => GitFileStatus::Deleted,
            ('R', _) => GitFileStatus::Renamed,
            ('C', _) => GitFileStatus::Copied,
            ('?', '?') => GitFileStatus::Untracked,
            ('!', '!') => GitFileStatus::Ignored,
            ('U', _) | (_, 'U') => GitFileStatus::Conflict,
            _ if index_status != ' ' => GitFileStatus::Staged,
            _ => GitFileStatus::Modified, // Default fallback
        }
    }

    pub async fn get_files_changed_since(&self, since: SystemTime) -> Result<Vec<PathBuf>> {
        self.maybe_refresh().await?;

        let cache = self.file_status_cache.read().await;
        let changed_files = cache
            .values()
            .filter(|info| info.last_modified >= since)
            .map(|info| info.path.clone())
            .collect();

        Ok(changed_files)
    }

    pub async fn get_conflicted_files(&self) -> Result<Vec<PathBuf>> {
        self.maybe_refresh().await?;

        let cache = self.file_status_cache.read().await;
        let conflicted_files = cache
            .values()
            .filter(|info| matches!(info.status, GitFileStatus::Conflict))
            .map(|info| info.path.clone())
            .collect();

        Ok(conflicted_files)
    }

    pub async fn is_repository_clean(&self) -> Result<bool> {
        let repo_info = self
            .get_repository_info()
            .await
            .ok_or_else(|| anyhow!("No repository info available"))?;
        Ok(!repo_info.is_dirty)
    }

    pub async fn get_untracked_files(&self) -> Result<Vec<PathBuf>> {
        self.maybe_refresh().await?;

        let cache = self.file_status_cache.read().await;
        let untracked_files = cache
            .values()
            .filter(|info| matches!(info.status, GitFileStatus::Untracked))
            .map(|info| info.path.clone())
            .collect();

        Ok(untracked_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    async fn setup_test_repo() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(repo_path)
            .args(&["init"])
            .output()?;

        // Configure git
        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .output()?;

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .output()?;

        // Create and commit initial file
        let test_file = repo_path.join("test.txt");
        fs::write(&test_file, "initial content")?;

        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "test.txt"])
            .output()?;

        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .output()?;

        Ok(temp_dir)
    }

    #[tokio::test]
    async fn test_git_integration_creation() -> Result<()> {
        // This test will only work if we're in a git repository
        if let Ok(integration) = GitIntegration::new().await {
            assert!(integration.is_git_available().await);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_git_root_detection() -> Result<()> {
        let root = GitIntegration::find_git_root().await?;
        // This may be None if we're not in a git repo, which is fine
        if let Some(root_path) = root {
            assert!(root_path.exists());
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_git_status_parsing() -> Result<()> {
        let integration = GitIntegration {
            repo_root: Some(PathBuf::from("/tmp")),
            last_scan: RwLock::new(None),
            file_status_cache: RwLock::new(HashMap::new()),
            repo_info_cache: RwLock::new(None),
            scan_interval: Duration::from_secs(30),
        };

        // Test various status combinations
        assert_eq!(
            integration.parse_git_status('M', ' '),
            GitFileStatus::Modified
        );
        assert_eq!(integration.parse_git_status('A', ' '), GitFileStatus::Added);
        assert_eq!(
            integration.parse_git_status('D', ' '),
            GitFileStatus::Deleted
        );
        assert_eq!(
            integration.parse_git_status('?', '?'),
            GitFileStatus::Untracked
        );
        assert_eq!(
            integration.parse_git_status('U', 'U'),
            GitFileStatus::Conflict
        );

        Ok(())
    }
}
