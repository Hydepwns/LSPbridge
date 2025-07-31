//! Cross-repository analysis types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cross-repository type reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeReference {
    /// Type name
    pub type_name: String,

    /// Source repository where type is defined
    pub source_repo_id: String,

    /// Source file path
    pub source_file: PathBuf,

    /// Source line number
    pub source_line: usize,

    /// Target repositories using this type
    pub target_repos: Vec<TypeUsage>,
}

/// Type usage in a target repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeUsage {
    pub repo_id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub usage_context: String,
}

/// Import relationship between files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRelation {
    /// Source repository
    pub source_repo_id: String,

    /// Source file
    pub source_file: PathBuf,

    /// Imported module/package
    pub import_path: String,

    /// Target repository (if external)
    pub target_repo_id: Option<String>,

    /// Resolved target file
    pub target_file: Option<PathBuf>,

    /// Import type
    pub import_type: ImportType,
}

/// Types of imports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportType {
    /// Local file import
    Local,

    /// Package import
    Package,

    /// Relative import
    Relative,

    /// Workspace import (monorepo)
    Workspace,

    /// External repository import
    External,
}

/// Type definition information
#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub repo_id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
}