//! Monorepo detection and analysis
//! 
//! This module provides comprehensive monorepo detection capabilities for various
//! workspace types including npm, pnpm, Lerna, Cargo, Nx, Rush, Bazel, and custom structures.

pub mod detectors;
pub mod types;
pub mod utils;

// Re-export main types for convenience
pub use types::{SubprojectInfo, WorkspaceConfig, WorkspaceLayout, WorkspaceType};

use anyhow::Result;
use detectors::DetectorRegistry;
use std::path::Path;

/// Main entry point for monorepo detection
/// 
/// This struct provides a simple interface for detecting various monorepo structures.
/// It uses a registry of specialized detectors to identify workspace types.
/// 
/// # Example
/// 
/// ```rust
/// use lspbridge::multi_repo::monorepo::MonorepoDetector;
/// use std::path::Path;
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let detector = MonorepoDetector::new();
///     
///     if let Some(layout) = detector.detect(Path::new("./my-project")).await? {
///         println!("Found {} workspace with {} subprojects", 
///                  layout.workspace_type, 
///                  layout.subprojects.len());
///         
///         for project in &layout.subprojects {
///             println!("  - {}: {}", project.name, project.relative_path.display());
///         }
///     } else {
///         println!("No monorepo structure detected");
///     }
///     
///     Ok(())
/// }
/// ```
pub struct MonorepoDetector {
    registry: DetectorRegistry,
}

impl MonorepoDetector {
    /// Create a new monorepo detector with all standard workspace detectors
    pub fn new() -> Self {
        Self {
            registry: DetectorRegistry::new(),
        }
    }

    /// Detect monorepo structure in the given directory
    /// 
    /// This method attempts to detect the workspace type by trying each registered
    /// detector in order. The first successful detection is returned.
    /// 
    /// # Arguments
    /// 
    /// * `root` - Path to the potential monorepo root directory
    /// 
    /// # Returns
    /// 
    /// * `Ok(Some(WorkspaceLayout))` - If a workspace is detected
    /// * `Ok(None)` - If no workspace is detected
    /// * `Err(...)` - If an error occurs during detection
    pub async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
        self.registry.detect(root).await
    }

    /// Get a list of supported workspace types
    pub fn supported_types(&self) -> Vec<&'static str> {
        vec![
            "npm",
            "pnpm", 
            "lerna",
            "cargo",
            "nx",
            "rush",
            "bazel",
            "custom"
        ]
    }
}

impl Default for MonorepoDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_detector_creation() {
        let detector = MonorepoDetector::new();
        let supported = detector.supported_types();
        
        assert!(supported.contains(&"npm"));
        assert!(supported.contains(&"cargo"));
        assert!(supported.contains(&"nx"));
    }

    #[tokio::test]
    async fn test_npm_workspace_detection() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create package.json with workspaces
        let package_json = r#"{
            "name": "test-workspace",
            "workspaces": ["packages/*"]
        }"#;
        fs::write(root.join("package.json"), package_json).await?;

        // Create a subproject
        fs::create_dir_all(root.join("packages/sub1")).await?;
        let sub_package = r#"{
            "name": "sub1",
            "version": "1.0.0"
        }"#;
        fs::write(root.join("packages/sub1/package.json"), sub_package).await?;

        let detector = MonorepoDetector::new();
        let result = detector.detect(root).await?;

        assert!(result.is_some());
        let layout = result.unwrap();
        assert_eq!(layout.workspace_type, WorkspaceType::NpmWorkspace);
        assert_eq!(layout.subprojects.len(), 1);
        assert_eq!(layout.subprojects[0].name, "sub1");

        Ok(())
    }

    #[tokio::test]
    async fn test_cargo_workspace_detection() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create Cargo.toml with workspace
        let cargo_toml = r#"
[workspace]
members = ["crates/*"]
"#;
        fs::write(root.join("Cargo.toml"), cargo_toml).await?;

        // Create a subproject
        fs::create_dir_all(root.join("crates/sub1")).await?;
        let sub_cargo = r#"
[package]
name = "sub1"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(root.join("crates/sub1/Cargo.toml"), sub_cargo).await?;

        let detector = MonorepoDetector::new();
        let result = detector.detect(root).await?;

        assert!(result.is_some());
        let layout = result.unwrap();
        assert_eq!(layout.workspace_type, WorkspaceType::CargoWorkspace);
        assert_eq!(layout.subprojects.len(), 1);
        assert_eq!(layout.subprojects[0].name, "sub1");

        Ok(())
    }

    #[tokio::test]
    async fn test_no_workspace_detected() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create just a regular directory with no workspace files
        fs::write(root.join("README.md"), "# Not a workspace").await?;

        let detector = MonorepoDetector::new();
        let result = detector.detect(root).await?;

        assert!(result.is_none());

        Ok(())
    }
}