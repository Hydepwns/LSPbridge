pub mod typescript;
pub mod rust;
pub mod python;

use crate::core::dependency_analyzer::types::{ImportDependency, Language};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

/// Trait for language-specific dependency resolution
pub trait LanguageResolver {
    /// Extract import dependency from a syntax node
    fn extract_import_dependency(
        &self,
        node: &Node,
        source: &str,
        current_file: &Path,
        line: u32,
    ) -> Option<ImportDependency>;
    
    /// Resolve import path to actual file path
    fn resolve_import_path(&self, current_file: &Path, import_path: &str) -> Option<PathBuf>;
    
    /// Extract exported symbols from a file
    fn extract_exports(&self, root: &Node, source: &str) -> Vec<crate::core::dependency_analyzer::types::ExportInfo>;
}

/// Factory for creating language-specific resolvers
pub fn get_resolver(language: Language) -> Option<Box<dyn LanguageResolver>> {
    match language {
        Language::TypeScript => Some(Box::new(typescript::TypeScriptResolver::new())),
        Language::Rust => Some(Box::new(rust::RustResolver::new())),
        Language::Python => Some(Box::new(python::PythonResolver::new())),
        Language::Unknown => None,
    }
}