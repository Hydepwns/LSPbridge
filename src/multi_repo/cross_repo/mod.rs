//! Cross-repository analysis for shared types and dependencies

pub mod analyzers;
pub mod caching;
pub mod synchronization;
pub mod types;

use crate::multi_repo::registry::RepositoryRegistry;
use analyzers::{DependencyAnalyzer, TypeAnalyzer};
use anyhow::Result;
use types::{ImportRelation, TypeReference};

/// Analyzes cross-repository dependencies and type usage
pub struct CrossRepoAnalyzer {
    /// Type analyzer instance
    type_analyzer: TypeAnalyzer,

    /// Dependency analyzer instance
    dependency_analyzer: DependencyAnalyzer,

    /// Whether to analyze type references
    analyze_types: bool,
}

impl CrossRepoAnalyzer {
    /// Create a new cross-repository analyzer
    pub fn new(analyze_types: bool) -> Self {
        Self {
            type_analyzer: TypeAnalyzer::new(),
            dependency_analyzer: DependencyAnalyzer::new(),
            analyze_types,
        }
    }

    /// Analyze type references across repositories
    pub async fn analyze_type_references(
        &self,
        registry: &RepositoryRegistry,
    ) -> Result<Vec<TypeReference>> {
        if !self.analyze_types {
            return Ok(Vec::new());
        }

        self.type_analyzer.analyze_type_references(registry).await
    }

    /// Resolve import paths across repositories
    pub async fn resolve_imports(
        &self,
        registry: &RepositoryRegistry,
    ) -> Result<Vec<ImportRelation>> {
        self.dependency_analyzer.resolve_imports(registry).await
    }
}

impl Default for CrossRepoAnalyzer {
    fn default() -> Self {
        Self::new(true)
    }
}