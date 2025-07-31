use crate::security::validate_path;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Path validation utilities
pub struct PathValidator;

impl PathValidator {
    /// Validate a repository path
    pub fn validate_repository_path(path: &Path) -> Result<PathBuf> {
        let validated = validate_path(path)
            .context("Invalid repository path")?;

        if !validated.exists() {
            return Err(anyhow::anyhow!("Repository path does not exist: {}", validated.display()));
        }

        if !validated.is_dir() {
            return Err(anyhow::anyhow!("Repository path is not a directory: {}", validated.display()));
        }

        Ok(validated)
    }

    /// Validate a workspace path
    pub fn validate_workspace_path(path: &Path) -> Result<PathBuf> {
        let validated = validate_path(path)
            .context("Invalid workspace path")?;

        // Workspace path doesn't need to exist (we'll create it)
        // but its parent should exist
        if let Some(parent) = validated.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Workspace parent directory does not exist: {}", 
                    parent.display()
                ));
            }
        }

        Ok(validated)
    }

    /// Validate an output file path
    pub fn validate_output_path(path: &Path) -> Result<PathBuf> {
        let validated = validate_path(path)
            .context("Invalid output path")?;

        // Ensure parent directory exists
        if let Some(parent) = validated.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Output directory does not exist: {}", 
                    parent.display()
                ));
            }
        }

        Ok(validated)
    }
}