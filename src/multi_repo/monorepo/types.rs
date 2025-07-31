//! Monorepo data structures and types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Information about a subproject in a monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubprojectInfo {
    /// Subproject name
    pub name: String,

    /// Path relative to monorepo root
    pub relative_path: PathBuf,

    /// Absolute path
    pub absolute_path: PathBuf,

    /// Primary language
    pub language: Option<String>,

    /// Build system
    pub build_system: Option<String>,

    /// Dependencies on other subprojects
    pub internal_deps: Vec<String>,

    /// External dependencies
    pub external_deps: Vec<String>,

    /// Package configuration (package.json, Cargo.toml, etc.)
    pub package_config: Option<serde_json::Value>,
}

/// Workspace layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    /// Monorepo root path
    pub root: PathBuf,

    /// Workspace type
    pub workspace_type: WorkspaceType,

    /// Subprojects in the workspace
    pub subprojects: Vec<SubprojectInfo>,

    /// Workspace configuration
    pub config: WorkspaceConfig,

    /// Shared configuration files
    pub shared_configs: Vec<PathBuf>,
}

/// Types of monorepo workspaces
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkspaceType {
    /// npm/Yarn workspaces
    NpmWorkspace,

    /// pnpm workspace
    PnpmWorkspace,

    /// Lerna monorepo
    Lerna,

    /// Cargo workspace
    CargoWorkspace,

    /// Bazel workspace
    Bazel,

    /// Nx workspace
    Nx,

    /// Rush monorepo
    Rush,

    /// Custom/Unknown
    Custom,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace patterns (globs)
    pub patterns: Vec<String>,

    /// Excluded patterns
    pub excludes: Vec<String>,

    /// Workspace-level dependencies
    pub dependencies: HashMap<String, String>,

    /// Build tool configuration
    pub build_config: Option<serde_json::Value>,
}