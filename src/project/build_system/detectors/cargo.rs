use anyhow::{Context, Result};
use std::path::Path;

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

pub struct CargoDetector;

impl BuildSystemDetector for CargoDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Cargo
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "Cargo.toml")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let cargo_toml_path = utils::get_file_path(project_root, "Cargo.toml");
        let content = utils::read_file(&cargo_toml_path)?;
        let toml: toml::Value = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("cargo build".to_string());
        commands.test = Some("cargo test".to_string());
        commands.run = Some("cargo run".to_string());
        commands.clean = Some("cargo clean".to_string());
        commands.format = Some("cargo fmt".to_string());
        commands.lint = Some("cargo clippy".to_string());

        // Check for workspace
        if toml.get("workspace").is_some() {
            commands.build = Some("cargo build --workspace".to_string());
            commands.test = Some("cargo test --workspace".to_string());
        }

        // Extract dependencies
        let mut dependencies = vec![];
        let mut dev_dependencies = vec![];

        if let Some(deps) = toml.get("dependencies").and_then(|d| d.as_table()) {
            for (name, _) in deps {
                dependencies.push(name.clone());
            }
        }

        if let Some(deps) = toml.get("dev-dependencies").and_then(|d| d.as_table()) {
            for (name, _) in deps {
                dev_dependencies.push(name.clone());
            }
        }

        // Look for additional config files
        let mut config_files = vec![cargo_toml_path];
        if utils::has_file(project_root, "Cargo.lock") {
            config_files.push(utils::get_file_path(project_root, "Cargo.lock"));
        }
        if utils::has_file(project_root, ".cargo/config.toml") {
            config_files.push(utils::get_file_path(project_root, ".cargo/config.toml"));
        }
        if utils::has_file(project_root, "rust-toolchain.toml") {
            config_files.push(utils::get_file_path(project_root, "rust-toolchain.toml"));
        }

        Ok(BuildConfig {
            system: BuildSystem::Cargo,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies,
        })
    }
}