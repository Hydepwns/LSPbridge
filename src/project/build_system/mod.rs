//! # Build System Detection and Configuration
//!
//! This module provides automatic detection of build systems in software projects
//! and extraction of build configuration, including commands and dependencies.
//!
//! ## Supported Build Systems
//!
//! - **Cargo** - Rust projects using Cargo.toml
//! - **Npm/Yarn/Pnpm** - Node.js projects using package.json
//! - **Poetry/Pip** - Python projects using pyproject.toml or requirements.txt
//! - **Maven/Gradle** - Java projects using pom.xml or build.gradle
//! - **Go** - Go projects using go.mod
//! - **Make** - Projects using Makefile
//!
//! ## Key Components
//!
//! - **BuildSystem**: Enum representing different build systems
//! - **BuildConfig**: Configuration including commands and dependencies
//! - **BuildSystemDetector**: Main detection logic
//! - **Language-specific detectors**: Specialized detection for each build system

pub mod detectors;
pub mod types;

pub use types::*;

use anyhow::Result;
use std::path::Path;

/// Main entry point for build system detection
pub struct BuildSystemDetector;

impl BuildSystemDetector {
    /// Detect the build system for a project
    pub fn detect(project_root: &Path) -> Result<BuildConfig> {
        detectors::detect_build_system(project_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_cargo() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1.0"

[dev-dependencies]
mockito = "0.31"

[alias]
ci = "check --all-features"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Cargo);
        assert_eq!(config.commands.build, Some("cargo build".to_string()));
        assert!(config.dependencies.contains(&"serde".to_string()));
        assert!(config.dev_dependencies.contains(&"mockito".to_string()));
    }

    #[test]
    fn test_detect_npm() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = r#"{
    "name": "test-project",
    "version": "1.0.0",
    "scripts": {
        "build": "webpack",
        "test": "jest",
        "dev": "nodemon server.js",
        "custom-task": "echo custom"
    },
    "dependencies": {
        "express": "^4.17.1",
        "lodash": "^4.17.21"
    },
    "devDependencies": {
        "jest": "^27.0.0"
    }
}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();
        // NPM detector requires package-lock.json
        fs::write(temp_dir.path().join("package-lock.json"), "{}").unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Npm);
        assert_eq!(config.commands.build, Some("npm run build".to_string()));
        assert_eq!(config.commands.run, Some("npm run dev".to_string()));
        assert!(config.dependencies.contains(&"express".to_string()));
        assert!(config.dev_dependencies.contains(&"jest".to_string()));
        assert_eq!(
            config.commands.custom.get("custom-task"),
            Some(&"npm run custom-task".to_string())
        );
    }

    #[test]
    fn test_detect_yarn() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(temp_dir.path().join("yarn.lock"), "").unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Yarn);
    }

    #[test]
    fn test_detect_poetry() {
        let temp_dir = TempDir::new().unwrap();
        let pyproject = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.26.0"

[tool.poetry.dev-dependencies]
pytest = "^6.2.5"

[tool.poetry.scripts]
serve = "myapp:serve"
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();
        // Poetry detector requires poetry.lock
        fs::write(temp_dir.path().join("poetry.lock"), "").unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Poetry);
        assert_eq!(config.commands.build, Some("poetry build".to_string()));
        assert!(config.dependencies.contains(&"requests".to_string()));
        assert!(config.dev_dependencies.contains(&"pytest".to_string()));
        assert_eq!(
            config.commands.custom.get("serve"),
            Some(&"poetry run serve".to_string())
        );
    }

    #[test]
    fn test_unknown_build_system() {
        let temp_dir = TempDir::new().unwrap();
        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Unknown);
        assert!(config.commands.build.is_none());
    }
}