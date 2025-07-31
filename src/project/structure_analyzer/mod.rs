//! Project structure analysis module

pub mod analyzer;
pub mod detection;
pub mod renderer;
pub mod types;

use analyzer::CoreAnalyzer;
use anyhow::Result;
use detection::MonorepoDetector;
use renderer::TreeRenderer;
use std::collections::HashSet;
use std::path::Path;

/// Main structure analyzer that orchestrates the analysis
pub struct StructureAnalyzer {
    core_analyzer: CoreAnalyzer,
    monorepo_detector: MonorepoDetector,
    tree_renderer: TreeRenderer,
}

impl Default for StructureAnalyzer {
    fn default() -> Self {
        let mut ignore_patterns = HashSet::new();
        ignore_patterns.insert("node_modules".to_string());
        ignore_patterns.insert("target".to_string());
        ignore_patterns.insert("dist".to_string());
        ignore_patterns.insert("build".to_string());
        ignore_patterns.insert(".git".to_string());
        ignore_patterns.insert(".svn".to_string());
        ignore_patterns.insert(".hg".to_string());
        ignore_patterns.insert("__pycache__".to_string());
        ignore_patterns.insert(".pytest_cache".to_string());
        ignore_patterns.insert(".tox".to_string());
        ignore_patterns.insert("venv".to_string());
        ignore_patterns.insert(".venv".to_string());
        ignore_patterns.insert("env".to_string());
        ignore_patterns.insert(".env".to_string());

        let mut source_indicators = HashSet::new();
        source_indicators.insert("src".to_string());
        source_indicators.insert("lib".to_string());
        source_indicators.insert("app".to_string());
        source_indicators.insert("source".to_string());
        source_indicators.insert("sources".to_string());
        source_indicators.insert("pkg".to_string());
        source_indicators.insert("cmd".to_string());
        source_indicators.insert("internal".to_string());

        let mut test_indicators = HashSet::new();
        test_indicators.insert("test".to_string());
        test_indicators.insert("tests".to_string());
        test_indicators.insert("spec".to_string());
        test_indicators.insert("specs".to_string());
        test_indicators.insert("__tests__".to_string());
        test_indicators.insert("test_".to_string());
        test_indicators.insert("_test".to_string());
        test_indicators.insert("testing".to_string());

        let mut config_extensions = HashSet::new();
        config_extensions.insert("toml".to_string());
        config_extensions.insert("yaml".to_string());
        config_extensions.insert("yml".to_string());
        config_extensions.insert("json".to_string());
        config_extensions.insert("ini".to_string());
        config_extensions.insert("conf".to_string());
        config_extensions.insert("config".to_string());
        config_extensions.insert("cfg".to_string());
        config_extensions.insert("xml".to_string());

        let mut doc_extensions = HashSet::new();
        doc_extensions.insert("md".to_string());
        doc_extensions.insert("rst".to_string());
        doc_extensions.insert("txt".to_string());
        doc_extensions.insert("adoc".to_string());
        doc_extensions.insert("textile".to_string());

        Self {
            core_analyzer: CoreAnalyzer::new(
                ignore_patterns,
                source_indicators,
                test_indicators,
                config_extensions,
                doc_extensions,
            ),
            monorepo_detector: MonorepoDetector::new(),
            tree_renderer: TreeRenderer::new(),
        }
    }
}

impl StructureAnalyzer {
    /// Create a new structure analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a project structure
    pub fn analyze(&self, project_root: &Path) -> Result<types::ProjectStructure> {
        let mut structure = self.core_analyzer.analyze(project_root)?;

        // Detect if this is a monorepo
        structure.is_monorepo = self
            .monorepo_detector
            .detect(project_root, &structure.subprojects);

        Ok(structure)
    }

    /// Get a formatted tree summary of the project structure
    pub fn get_file_tree_summary(&self, structure: &types::ProjectStructure, max_depth: usize) -> String {
        self.tree_renderer.get_file_tree_summary(structure, max_depth)
    }
}

// Re-export key types
pub use types::{DirectoryNode, ProjectStructure};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_basic_structure_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap(); // Should be ignored

        // Create files
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src/lib.rs"), "pub mod utils;").unwrap();
        fs::write(root.join("tests/test_main.rs"), "#[test] fn test() {}").unwrap();
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();
        fs::write(root.join("README.md"), "# Project").unwrap();
        fs::write(root.join("docs/guide.md"), "# Guide").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();

        assert_eq!(structure.total_files, 6); // node_modules should be ignored
        assert_eq!(structure.source_dirs.len(), 1);
        assert_eq!(structure.test_dirs.len(), 1);
        assert!(structure
            .config_files
            .iter()
            .any(|p| p.ends_with("Cargo.toml")));
        assert_eq!(structure.documentation_files.len(), 2);
        assert_eq!(structure.file_types.get("rs"), Some(&3));
        assert_eq!(structure.file_types.get("md"), Some(&2));
    }

    #[test]
    fn test_monorepo_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a yarn workspaces monorepo
        let package_json = r#"{
            "name": "monorepo",
            "workspaces": ["packages/*"]
        }"#;
        fs::write(root.join("package.json"), package_json).unwrap();

        fs::create_dir_all(root.join("packages/app")).unwrap();
        fs::create_dir_all(root.join("packages/lib")).unwrap();
        fs::write(root.join("packages/app/package.json"), "{}").unwrap();
        fs::write(root.join("packages/lib/package.json"), "{}").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();

        assert!(structure.is_monorepo);
        assert_eq!(structure.subprojects.len(), 2);
    }

    #[test]
    fn test_language_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create TypeScript project
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/index.ts"), "export {}").unwrap();
        fs::write(root.join("src/utils.ts"), "export {}").unwrap();
        fs::write(root.join("src/types.ts"), "export {}").unwrap();
        fs::write(root.join("test.js"), "test()").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();

        assert_eq!(
            structure.get_main_language(),
            Some("TypeScript".to_string())
        );
    }

    #[test]
    fn test_file_tree_summary() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src/utils")).unwrap();
        fs::write(root.join("src/main.rs"), "").unwrap();
        fs::write(root.join("src/utils/helpers.rs"), "").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();
        let tree = analyzer.get_file_tree_summary(&structure, 3);

        assert!(tree.contains("src/"));
        assert!(tree.contains("utils/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("helpers.rs"));
    }
}