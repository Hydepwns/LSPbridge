use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Dependencies for a single file
#[derive(Debug, Clone)]
pub struct FileDependencies {
    pub file_path: PathBuf,
    pub imports: Vec<ImportDependency>,
    pub exports: Vec<ExportInfo>,
    pub type_references: Vec<TypeReference>,
    pub function_calls: Vec<ExternalFunctionCall>,
    pub last_modified: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct ImportDependency {
    pub source_file: PathBuf,
    pub imported_symbols: Vec<String>,
    pub import_type: ImportType,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ImportType {
    Default,
    Named(Vec<String>),
    Namespace(String),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub symbol_name: String,
    pub export_type: ExportType,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ExportType {
    Function,
    Type,
    Variable,
    Class,
    Default,
}

#[derive(Debug, Clone)]
pub struct TypeReference {
    pub type_name: String,
    pub source_file: Option<PathBuf>,
    pub line: u32,
    pub context: String, // The surrounding context where type is used
}

#[derive(Debug, Clone)]
pub struct ExternalFunctionCall {
    pub function_name: String,
    pub module_path: Option<PathBuf>,
    pub line: u32,
    pub arguments_count: usize,
}

/// Dependency graph for cross-file analysis
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Map from file to its dependencies
    pub file_dependencies: HashMap<PathBuf, FileDependencies>,
    /// Reverse map: which files depend on each file
    pub reverse_dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Files that import symbols used in diagnostic location
    pub impact_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
    TypeScript,
    Rust,
    Python,
    Unknown,
}