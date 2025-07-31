//! Multi-repository support for cross-repository analysis
//!
//! This module provides functionality for analyzing diagnostics across multiple
//! related repositories, including monorepo support, shared type definitions,
//! and team collaboration features.

pub mod aggregator;
pub mod collaboration;
pub mod cross_repo;
pub mod monorepo;
pub mod registry;

pub use aggregator::{AggregatedDiagnostic, DiagnosticAggregator};
pub use collaboration::{DiagnosticAssignment, TeamDatabase, TeamMember};
pub use cross_repo::CrossRepoAnalyzer;
pub use cross_repo::types::TypeReference;
pub use monorepo::{MonorepoDetector, SubprojectInfo, WorkspaceLayout, WorkspaceType};
pub use registry::{RepositoryInfo, RepositoryRegistry, RepositoryRelation};

use crate::core::config::ConfigDefaults;
use crate::impl_config_defaults;
use anyhow::Result;
use std::path::PathBuf;

/// Multi-repository configuration
///
/// **DEPRECATED**: This struct is deprecated in favor of the unified configuration system.
/// Use `crate::core::config::UnifiedConfig` with the `multi_repo` field instead.
/// This struct is kept for backward compatibility and will be removed in a future version.
#[deprecated(
    since = "0.2.0",
    note = "Use crate::core::config::UnifiedConfig instead"
)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiRepoConfig {
    /// Path to the repository registry database
    pub registry_path: PathBuf,

    /// Path to the team collaboration database
    pub team_db_path: Option<PathBuf>,

    /// Enable automatic monorepo detection
    pub auto_detect_monorepo: bool,

    /// Enable cross-repository type tracking
    pub enable_cross_repo_types: bool,

    /// Maximum number of repositories to analyze concurrently
    pub max_concurrent_repos: usize,

    /// Cache directory for cross-repo analysis
    pub cache_dir: PathBuf,
}

impl Default for MultiRepoConfig {
    fn default() -> Self {
        Self {
            registry_path: PathBuf::from(".lsp-bridge/repos.db"),
            team_db_path: None,
            auto_detect_monorepo: true,
            enable_cross_repo_types: true,
            max_concurrent_repos: 4,
            cache_dir: PathBuf::from(".lsp-bridge/cache/multi-repo"),
        }
    }
}

impl_config_defaults!(MultiRepoConfig, "multi-repo.toml", validate => |config: &MultiRepoConfig| {
    if config.max_concurrent_repos == 0 {
        anyhow::bail!("max_concurrent_repos must be greater than 0");
    }
    if !config.cache_dir.is_absolute() && !config.cache_dir.starts_with(".") {
        anyhow::bail!("cache_dir must be absolute or relative path starting with '.'");
    }
    Ok(())
});

/// Migration utilities for multi-repo configuration
impl MultiRepoConfig {
    /// Convert to the unified config multi-repo section
    pub fn to_unified(&self) -> crate::core::config::MultiRepoConfig {
        crate::core::config::MultiRepoConfig {
            registry_path: self.registry_path.clone(),
            team_db_path: self.team_db_path.clone(),
            auto_detect_monorepo: self.auto_detect_monorepo,
            enable_cross_repo_types: self.enable_cross_repo_types,
            max_concurrent_repos: self.max_concurrent_repos,
            cache_dir: self.cache_dir.clone(),
        }
    }

    /// Create from the unified config multi-repo section
    pub fn from_unified(unified: &crate::core::config::MultiRepoConfig) -> Self {
        Self {
            registry_path: unified.registry_path.clone(),
            team_db_path: unified.team_db_path.clone(),
            auto_detect_monorepo: unified.auto_detect_monorepo,
            enable_cross_repo_types: unified.enable_cross_repo_types,
            max_concurrent_repos: unified.max_concurrent_repos,
            cache_dir: unified.cache_dir.clone(),
        }
    }
}

/// Multi-repository analysis context
pub struct MultiRepoContext {
    config: MultiRepoConfig,
    registry: RepositoryRegistry,
    aggregator: DiagnosticAggregator,
    analyzer: CrossRepoAnalyzer,
    team_db: Option<TeamDatabase>,
}

impl MultiRepoContext {
    /// Create a new multi-repository context
    pub async fn new(config: MultiRepoConfig) -> Result<Self> {
        let registry = RepositoryRegistry::load_or_create(&config.registry_path).await?;
        let aggregator = DiagnosticAggregator::new(config.max_concurrent_repos);
        let analyzer = CrossRepoAnalyzer::new(config.enable_cross_repo_types);

        let team_db = if let Some(db_path) = &config.team_db_path {
            Some(TeamDatabase::connect(db_path).await?)
        } else {
            None
        };

        Ok(Self {
            config,
            registry,
            aggregator,
            analyzer,
            team_db,
        })
    }

    /// Register a new repository
    pub async fn register_repo(&mut self, info: RepositoryInfo) -> Result<()> {
        self.registry.register(info).await
    }

    /// Analyze diagnostics across all registered repositories
    pub async fn analyze_all(&mut self) -> Result<Vec<AggregatedDiagnostic>> {
        let repos = self.registry.list_active().await?;
        self.aggregator.analyze_repositories(repos).await
    }

    /// Find cross-repository type references
    pub async fn find_cross_repo_types(&mut self) -> Result<Vec<TypeReference>> {
        self.analyzer.analyze_type_references(&self.registry).await
    }

    /// Detect monorepo structure
    pub async fn detect_monorepo(&self, root: &PathBuf) -> Result<Option<WorkspaceLayout>> {
        if self.config.auto_detect_monorepo {
            let detector = MonorepoDetector::new();
            detector.detect(root).await
        } else {
            Ok(None)
        }
    }
}
