pub mod types;
pub mod resolvers;
pub mod cache;
pub mod analyzer;

pub use types::{
    FileDependencies, ImportDependency, ImportType, ExportInfo, ExportType,
    TypeReference, ExternalFunctionCall, DependencyGraph, Language
};
pub use analyzer::AnalysisEngine;

use crate::core::semantic_context::{DependencyInfo, DependencyType};
use crate::core::types::Diagnostic;
use anyhow::Result;
use std::path::Path;

/// Analyzes dependency relationships across files
pub struct DependencyAnalyzer {
    engine: AnalysisEngine,
}

impl DependencyAnalyzer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            engine: AnalysisEngine::new()?,
        })
    }

    /// Build dependency graph for a set of files
    pub async fn build_graph<P: AsRef<Path>>(&mut self, files: &[P]) -> Result<DependencyGraph> {
        self.engine.build_graph(files).await
    }

    /// Analyze dependencies for a specific diagnostic location
    pub async fn analyze_diagnostic_dependencies(
        &mut self,
        diagnostic: &Diagnostic,
        graph: &DependencyGraph,
    ) -> Result<Vec<DependencyInfo>> {
        self.engine.analyze_diagnostic_dependencies(diagnostic, graph).await
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new().unwrap()
    }
}