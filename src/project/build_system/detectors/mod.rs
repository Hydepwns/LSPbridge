use anyhow::Result;
use std::path::Path;

use crate::project::build_system::types::{BuildConfig, BuildSystem};

pub mod cargo;
pub mod node;
pub mod python;
pub mod java;
pub mod go;
pub mod make;
pub mod monorepo;

/// Trait for build system detection
pub trait BuildSystemDetector: Send + Sync {
    /// Get the build system this detector handles
    #[allow(dead_code)]
    fn build_system(&self) -> BuildSystem;

    /// Check if this detector can handle the given project
    fn can_detect(&self, project_root: &Path) -> bool;

    /// Detect and extract build configuration
    fn detect(&self, project_root: &Path) -> Result<BuildConfig>;
}

/// Detect the build system for a project
pub fn detect_build_system(project_root: &Path) -> Result<BuildConfig> {
    let detectors: Vec<Box<dyn BuildSystemDetector>> = vec![
        // Check for monorepos first as they often have precedence
        Box::new(monorepo::LernaDetector),
        Box::new(monorepo::NxDetector),
        Box::new(monorepo::RushDetector),
        Box::new(monorepo::YarnWorkspacesDetector),
        Box::new(monorepo::PnpmWorkspacesDetector),
        Box::new(monorepo::NpmWorkspacesDetector),
        // Regular build systems
        Box::new(cargo::CargoDetector),
        Box::new(node::NpmDetector),
        Box::new(node::YarnDetector),
        Box::new(node::PnpmDetector),
        Box::new(python::PoetryDetector),
        Box::new(python::PipDetector),
        Box::new(java::MavenDetector),
        Box::new(java::GradleDetector),
        Box::new(go::GoDetector),
        Box::new(make::MakeDetector),
    ];

    for detector in detectors {
        if detector.can_detect(project_root) {
            return detector.detect(project_root);
        }
    }

    // No build system detected
    Ok(BuildConfig {
        system: BuildSystem::Unknown,
        root_path: project_root.to_path_buf(),
        config_files: vec![],
        commands: Default::default(),
        dependencies: vec![],
        dev_dependencies: vec![],
    })
}

/// Common utilities for detectors
pub mod utils {
    use std::path::Path;
    use anyhow::{Context, Result};

    /// Read a file to string with context
    pub fn read_file(path: &Path) -> Result<String> {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))
    }

    /// Check if a file exists relative to project root
    pub fn has_file(project_root: &Path, file_name: &str) -> bool {
        project_root.join(file_name).exists()
    }

    /// Get the path to a file relative to project root
    pub fn get_file_path(project_root: &Path, file_name: &str) -> std::path::PathBuf {
        project_root.join(file_name)
    }
}