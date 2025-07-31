use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::path::Path;

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

pub struct NpmDetector;

impl BuildSystemDetector for NpmDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Npm
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "package.json") && 
        utils::has_file(project_root, "package-lock.json")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        detect_node_config(project_root, BuildSystem::Npm)
    }
}

pub struct YarnDetector;

impl BuildSystemDetector for YarnDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Yarn
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "package.json") && 
        utils::has_file(project_root, "yarn.lock")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        detect_node_config(project_root, BuildSystem::Yarn)
    }
}

pub struct PnpmDetector;

impl BuildSystemDetector for PnpmDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Pnpm
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "package.json") && 
        utils::has_file(project_root, "pnpm-lock.yaml")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        detect_node_config(project_root, BuildSystem::Pnpm)
    }
}

/// Common detection logic for Node.js-based build systems
fn detect_node_config(project_root: &Path, system: BuildSystem) -> Result<BuildConfig> {
    let package_json_path = utils::get_file_path(project_root, "package.json");
    let content = utils::read_file(&package_json_path)?;
    let package_json: JsonValue = serde_json::from_str(&content)
        .context("Failed to parse package.json")?;

    let mut commands = BuildCommands::default();
    
    // Extract scripts from package.json
    if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
        // Map common script names to standard commands
        let cmd_prefix = match system {
            BuildSystem::Npm => "npm run",
            BuildSystem::Yarn => "yarn",
            BuildSystem::Pnpm => "pnpm",
            _ => "npm run",
        };

        // Standard script mappings
        if scripts.contains_key("build") {
            commands.build = Some(format!("{} build", cmd_prefix));
        }
        if scripts.contains_key("test") {
            commands.test = Some(format!("{} test", cmd_prefix));
        }
        if scripts.contains_key("lint") {
            commands.lint = Some(format!("{} lint", cmd_prefix));
        }
        if scripts.contains_key("format") || scripts.contains_key("fmt") {
            let script_name = if scripts.contains_key("format") { "format" } else { "fmt" };
            commands.format = Some(format!("{} {}", cmd_prefix, script_name));
        }
        if scripts.contains_key("start") {
            commands.run = Some(format!("{} start", cmd_prefix));
        } else if scripts.contains_key("dev") {
            commands.run = Some(format!("{} dev", cmd_prefix));
        }
        if scripts.contains_key("clean") {
            commands.clean = Some(format!("{} clean", cmd_prefix));
        }

        // Add other scripts as custom commands
        for (name, _) in scripts {
            if !matches!(name.as_str(), "build" | "test" | "lint" | "format" | "fmt" | "start" | "dev" | "clean") {
                commands.custom.insert(name.clone(), format!("{} {}", cmd_prefix, name));
            }
        }
    }

    // Default install command
    let install_cmd = match system {
        BuildSystem::Npm => "npm install",
        BuildSystem::Yarn => "yarn install",
        BuildSystem::Pnpm => "pnpm install",
        _ => "npm install",
    };
    commands.custom.insert("install".to_string(), install_cmd.to_string());

    // Extract dependencies
    let mut dependencies = vec![];
    let mut dev_dependencies = vec![];

    if let Some(deps) = package_json.get("dependencies").and_then(|d| d.as_object()) {
        for (name, _) in deps {
            dependencies.push(name.clone());
        }
    }

    if let Some(deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, _) in deps {
            dev_dependencies.push(name.clone());
        }
    }

    // Collect config files
    let mut config_files = vec![package_json_path];
    
    match system {
        BuildSystem::Npm => {
            if utils::has_file(project_root, "package-lock.json") {
                config_files.push(utils::get_file_path(project_root, "package-lock.json"));
            }
        }
        BuildSystem::Yarn => {
            if utils::has_file(project_root, "yarn.lock") {
                config_files.push(utils::get_file_path(project_root, "yarn.lock"));
            }
            if utils::has_file(project_root, ".yarnrc.yml") {
                config_files.push(utils::get_file_path(project_root, ".yarnrc.yml"));
            }
        }
        BuildSystem::Pnpm => {
            if utils::has_file(project_root, "pnpm-lock.yaml") {
                config_files.push(utils::get_file_path(project_root, "pnpm-lock.yaml"));
            }
            if utils::has_file(project_root, ".pnpmfile.cjs") {
                config_files.push(utils::get_file_path(project_root, ".pnpmfile.cjs"));
            }
        }
        _ => {}
    }

    // Common Node.js config files
    if utils::has_file(project_root, ".npmrc") {
        config_files.push(utils::get_file_path(project_root, ".npmrc"));
    }
    if utils::has_file(project_root, "tsconfig.json") {
        config_files.push(utils::get_file_path(project_root, "tsconfig.json"));
    }
    if utils::has_file(project_root, ".eslintrc.json") {
        config_files.push(utils::get_file_path(project_root, ".eslintrc.json"));
    }
    if utils::has_file(project_root, ".prettierrc") {
        config_files.push(utils::get_file_path(project_root, ".prettierrc"));
    }

    Ok(BuildConfig {
        system,
        root_path: project_root.to_path_buf(),
        config_files,
        commands,
        dependencies,
        dev_dependencies,
    })
}