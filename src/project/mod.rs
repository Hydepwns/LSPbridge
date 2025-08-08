mod build_system;
mod structure_analyzer;

pub use build_system::{BuildCommands, BuildConfig, BuildSystem, BuildSystemDetector};
pub use structure_analyzer::{DirectoryNode, ProjectStructure, StructureAnalyzer};

/// Project type detection based on files and structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectType {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    Unknown,
}

/// Module for project structure detection functions
pub mod structure {
    use anyhow::Result;
    use std::path::Path;
    
    /// Detect if a directory is a monorepo
    pub fn detect_monorepo(_path: &Path) -> Result<bool> {
        // TODO: Implement monorepo detection logic
        Ok(false)
    }
    
    /// Find workspaces in a directory
    pub fn find_workspaces(_path: &Path) -> Result<Vec<String>> {
        // TODO: Implement workspace detection logic
        Ok(vec![])
    }
}

/// Simplified project analyzer for tests and basic usage
pub struct ProjectAnalyzer {
    #[allow(dead_code)]
    structure_analyzer: StructureAnalyzer,
}

impl ProjectAnalyzer {
    /// Create a new ProjectAnalyzer
    pub fn new() -> Result<Self> {
        Ok(Self {
            structure_analyzer: StructureAnalyzer::new(),
        })
    }
    
    /// Analyze a project at the given path
    pub fn analyze(&self, project_root: &Path) -> Result<ProjectInfo> {
        ProjectInfo::analyze(project_root)
    }
    
    /// Analyze a directory asynchronously (for compatibility with async tests)
    pub async fn analyze_directory(&self, project_root: &Path) -> Result<ProjectInfo> {
        // Just call the sync version - no async work needed currently
        self.analyze(project_root)
    }
    
    /// Detect the primary language of a file
    pub fn detect_language(&self, file_path: &Path) -> Result<String> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
            
        let language = match extension {
            "rs" => "rust",
            "ts" | "tsx" => "typescript", 
            "js" | "jsx" => "javascript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" => "cpp",
            _ => "unknown",
        };
        
        Ok(language.to_string())
    }
}

impl Default for ProjectAnalyzer {
    fn default() -> Self {
        Self::new().expect("Failed to create ProjectAnalyzer")
    }
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub build_config: BuildConfig,
    pub structure: ProjectStructure,
    pub project_type: ProjectType,
}

impl ProjectInfo {
    pub fn analyze(project_root: &Path) -> Result<Self> {
        let build_config = BuildSystemDetector::detect(project_root)?;
        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(project_root)?;
        
        // Simple project type detection based on build system
        let project_type = match build_config.system {
            BuildSystem::Cargo => ProjectType::Rust,
            BuildSystem::Npm | BuildSystem::Yarn => {
                // TODO: Distinguish between TypeScript and JavaScript
                ProjectType::TypeScript
            }
            _ => ProjectType::Unknown,
        };

        Ok(Self {
            build_config,
            structure,
            project_type,
        })
    }

    pub fn summary(&self) -> String {
        let mut summary = String::new();

        // Build system info
        summary.push_str(&format!("Build System: {:?}\n", self.build_config.system));
        summary.push_str("Available Commands:\n");
        for (name, cmd) in self.build_config.all_commands() {
            summary.push_str(&format!("  {name}: {cmd}\n"));
        }
        summary.push('\n');

        // Project structure info
        summary.push_str(&self.structure.summary());

        summary
    }

    pub fn get_context_for_diagnostics(&self) -> String {
        let mut context = String::new();

        context.push_str(&format!("Project Type: {:?}\n", self.build_config.system));

        if self.structure.is_monorepo {
            context.push_str("Structure: Monorepo\n");
        }

        if let Some(lang) = self.structure.get_main_language() {
            context.push_str(&format!("Primary Language: {lang}\n"));
        }

        context.push_str("\nHow to run common tasks:\n");
        if let Some(cmd) = self.build_config.get_command("build") {
            context.push_str(&format!("- Build: {cmd}\n"));
        }
        if let Some(cmd) = self.build_config.get_command("test") {
            context.push_str(&format!("- Test: {cmd}\n"));
        }
        if let Some(cmd) = self.build_config.get_command("lint") {
            context.push_str(&format!("- Lint: {cmd}\n"));
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_project_info_integration() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a Rust project
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#;
        fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

        let info = ProjectInfo::analyze(root).unwrap();

        assert_eq!(info.build_config.system, BuildSystem::Cargo);
        assert_eq!(info.structure.total_files, 2);

        let summary = info.summary();
        assert!(summary.contains("Build System: Cargo"));
        assert!(summary.contains("cargo build"));
    }
}
