//! Multi-repository CLI module
//!
//! This module provides comprehensive multi-repository management capabilities including
//! repository registration, cross-repository analysis, team collaboration, workspace 
//! synchronization, and monorepo detection.
//!
//! ## Architecture
//!
//! The multi-repo CLI is organized into focused modules:
//! - `types`: Command-line argument types and enums
//! - `handlers`: Command implementation and routing
//! - `discovery`: Repository discovery and metadata extraction
//! - `analysis`: Cross-repository impact analysis
//! - `workspace`: Workspace synchronization utilities  
//! - `config`: Configuration management and validation
//!
//! ## Usage
//!
//! ```rust
//! use crate::cli::multi_repo::{MultiRepoCommand, handle_multi_repo_command};
//!
//! // Handle a multi-repo command
//! let result = handle_multi_repo_command(cmd, config_path).await?;
//! ```

pub mod analysis;
pub mod config;
pub mod discovery;
pub mod handlers;
pub mod types;
pub mod workspace;

// Re-export commonly used types for convenience
pub use analysis::{
    CrossRepoAnalysisResult, 
    MultiRepoAnalyzer, 
    RepositoryImpactScore,
    RepositoryRelationship,
    SharedDependency,
    SharedType
};

pub use config::{
    MultiRepoCliConfig,
    MultiRepoConfigManager,
    OutputFormat,
    PathValidator,
    SyncMode
};

pub use discovery::{
    GitInfo,
    RepositoryCandidate,
    RepositoryDiscovery,
    RepositoryType,
    SubprojectInfo
};

pub use handlers::{
    detect_primary_language,
    display_diagnostics_table
};

pub use types::{
    AssignmentStatusArg,
    MultiRepoCommand,
    PriorityArg,
    RelationTypeArg,
    TeamCommand,
    TeamRoleArg
};

pub use workspace::{
    WorkspaceIndex,
    WorkspaceSynchronizer,
    WorkspaceSyncConfig,
    WorkspaceSyncResult
};

use anyhow::Result;
use std::path::PathBuf;

/// Main entry point for multi-repository CLI commands
///
/// This function routes multi-repository commands to their appropriate handlers
/// and manages the execution context.
///
/// # Arguments
///
/// * `cmd` - The multi-repository command to execute
/// * `config_path` - Optional path to configuration file
///
/// # Returns
///
/// Returns `Ok(())` on successful execution, or an error if the command fails.
///
/// # Example
///
/// ```rust
/// use crate::cli::multi_repo::{MultiRepoCommand, handle_multi_repo_command};
/// use std::path::PathBuf;
///
/// # async fn example() -> anyhow::Result<()> {
/// let cmd = MultiRepoCommand::List {
///     all: false,
///     tag: None,
///     format: crate::cli::multi_repo::OutputFormat::Table,
/// };
///
/// handle_multi_repo_command(cmd, None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn handle_multi_repo_command(
    cmd: MultiRepoCommand,
    config_path: Option<PathBuf>,
) -> Result<()> {
    handlers::handle_multi_repo_command(cmd, config_path).await
}

/// Initialize multi-repository CLI configuration
///
/// Sets up default configuration and creates necessary directories if they don't exist.
///
/// # Arguments
///
/// * `config_path` - Optional custom configuration path
///
/// # Returns
///
/// Returns the initialized configuration manager.
pub async fn initialize_config(config_path: Option<PathBuf>) -> Result<MultiRepoConfigManager> {
    let config_path = config_path.or_else(|| {
        config::ConfigUtils::default_config_path().ok()
    });

    let mut manager = MultiRepoConfigManager::new(config_path);
    manager.load_configuration().await?;
    
    Ok(manager)
}

/// Discover repositories in a given directory
///
/// Convenience function for repository discovery with default settings.
///
/// # Arguments
///
/// * `root_path` - Root directory to search for repositories
/// * `max_depth` - Maximum search depth (default: 5)
///
/// # Returns
///
/// Returns a list of discovered repository candidates.
pub async fn discover_repositories(
    root_path: &std::path::Path,
    max_depth: Option<usize>,
) -> Result<Vec<RepositoryCandidate>> {
    let mut discovery = RepositoryDiscovery::new();
    
    if let Some(depth) = max_depth {
        discovery = discovery.with_max_depth(depth);
    }

    discovery.discover_repositories(root_path).await
}

/// Perform cross-repository analysis
///
/// Convenience function for analyzing cross-repository relationships and impact.
///
/// # Arguments
///
/// * `context` - Multi-repository context
/// * `min_impact` - Minimum impact threshold (default: 0.3)
///
/// # Returns
///
/// Returns detailed cross-repository analysis results.
pub async fn analyze_cross_repo_impact(
    context: &crate::multi_repo::MultiRepoContext,
    min_impact: Option<f32>,
) -> Result<CrossRepoAnalysisResult> {
    let mut analyzer = MultiRepoAnalyzer::new();
    
    if let Some(threshold) = min_impact {
        analyzer = analyzer.with_min_impact(threshold);
    }

    analyzer.analyze_cross_repo_impact(context).await
}

/// Synchronize workspace with repositories
///
/// Convenience function for workspace synchronization with default settings.
///
/// # Arguments
///
/// * `workspace_root` - Workspace directory
/// * `context` - Multi-repository context
/// * `sync_mode` - Synchronization mode (default: Incremental)
///
/// # Returns
///
/// Returns workspace synchronization results.
pub async fn synchronize_workspace(
    workspace_root: PathBuf,
    context: &crate::multi_repo::MultiRepoContext,
    sync_mode: Option<SyncMode>,
) -> Result<WorkspaceSyncResult> {
    let mut synchronizer = WorkspaceSynchronizer::new(workspace_root);
    
    if let Some(mode) = sync_mode {
        synchronizer = synchronizer.with_sync_mode(mode);
    }

    synchronizer.synchronize_workspace(context).await
}

/// CLI module utilities and helpers
pub mod utils {
    use super::*;
    use colored::Colorize;

    /// Print a formatted success message
    pub fn print_success(message: &str) {
        println!("{} {}", "✓".green(), message);
    }

    /// Print a formatted info message  
    pub fn print_info(message: &str) {
        println!("{} {}", "→".blue(), message);
    }

    /// Print a formatted warning message
    pub fn print_warning(message: &str) {
        println!("{} {}", "!".yellow(), message);
    }

    /// Print a formatted error message
    pub fn print_error(message: &str) {
        println!("{} {}", "✗".red(), message);
    }

    /// Format file size in human-readable format
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Format duration in human-readable format
    pub fn format_duration(seconds: u64) -> String {
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    }

    /// Validate command arguments
    pub fn validate_command_args(cmd: &MultiRepoCommand) -> Result<()> {
        match cmd {
            MultiRepoCommand::Register { path, .. } => {
                PathValidator::validate_repository_path(path)?;
            }
            MultiRepoCommand::Analyze { min_impact, output, .. } => {
                if *min_impact < 0.0 || *min_impact > 1.0 {
                    return Err(anyhow::anyhow!("min_impact must be between 0.0 and 1.0"));
                }
                if let Some(output_path) = output {
                    PathValidator::validate_output_path(output_path)?;
                }
            }
            MultiRepoCommand::DetectMonorepo { path, .. } => {
                PathValidator::validate_repository_path(path)?;
            }
            _ => {} // Other commands don't need validation
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_initialize_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = initialize_config(Some(config_path.clone())).await.unwrap();
        assert!(config_path.exists());
        assert!(matches!(manager.config().default_output_format, OutputFormat::Table));
    }

    #[tokio::test]
    async fn test_discover_repositories() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        fs::create_dir_all(&repo_path).unwrap();
        fs::create_dir(repo_path.join(".git")).unwrap();
        fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();

        let candidates = discover_repositories(temp_dir.path(), Some(2)).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "test-repo");
        assert_eq!(candidates[0].repo_type, RepositoryType::Git);
    }

    #[test]
    fn test_utils_formatting() {
        assert_eq!(utils::format_file_size(1024), "1.0 KB");
        assert_eq!(utils::format_file_size(1048576), "1.0 MB");
        assert_eq!(utils::format_duration(65), "1m 5s");
        assert_eq!(utils::format_duration(3661), "1h 1m");
    }

    #[test]
    fn test_command_validation() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path()).unwrap();

        let valid_cmd = MultiRepoCommand::Register {
            path: temp_dir.path().to_path_buf(),
            name: None,
            remote_url: None,
            language: None,
            tags: None,
        };

        assert!(utils::validate_command_args(&valid_cmd).is_ok());

        let invalid_cmd = MultiRepoCommand::Analyze {
            min_impact: 1.5, // Invalid value > 1.0
            output: None,
            format: OutputFormat::Table,
        };

        assert!(utils::validate_command_args(&invalid_cmd).is_err());
    }

    #[test]
    fn test_module_exports() {
        // Test that key types are properly exported
        let _config = MultiRepoCliConfig::default();
        let _analyzer = MultiRepoAnalyzer::new();
        let _discovery = RepositoryDiscovery::new();
        let temp_path = std::path::PathBuf::from("/tmp");
        let _synchronizer = WorkspaceSynchronizer::new(temp_path);
    }
}