//! Git integration wrapper

use crate::core::{GitIntegration, GitRepositoryInfo};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Wrapper for Git integration functionality
pub struct GitIntegrationWrapper {
    git: Option<Arc<GitIntegration>>,
}

impl GitIntegrationWrapper {
    /// Create a new Git integration wrapper
    pub fn new(git: Option<Arc<GitIntegration>>) -> Self {
        Self { git }
    }

    /// Get repository information
    pub async fn get_repository_info(&self) -> Option<GitRepositoryInfo> {
        self.git.as_ref()?.get_repository_info().await
    }

    /// Get modified files
    pub async fn get_modified_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(git) = &self.git {
            git.get_modified_files().await
        } else {
            Ok(vec![])
        }
    }

    /// Get conflicted files
    pub async fn get_conflicted_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(git) = &self.git {
            git.get_conflicted_files().await
        } else {
            Ok(vec![])
        }
    }

    /// Get untracked files
    pub async fn get_untracked_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(git) = &self.git {
            git.get_untracked_files().await
        } else {
            Ok(vec![])
        }
    }

    /// Check if file is ignored
    pub async fn is_file_ignored(&self, file_path: &Path) -> Result<bool> {
        if let Some(git) = &self.git {
            git.is_file_ignored(file_path).await
        } else {
            Ok(false)
        }
    }

    /// Check if repository is clean
    pub async fn is_repository_clean(&self) -> Result<bool> {
        if let Some(git) = &self.git {
            git.is_repository_clean().await
        } else {
            Ok(true) // Assume clean if no git
        }
    }

    /// Refresh Git status
    pub async fn refresh_git_status(&self) -> Result<()> {
        if let Some(git) = &self.git {
            git.refresh_git_status().await
        } else {
            Ok(())
        }
    }
}