use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

/// Monorepo types we can detect
#[derive(Debug, Clone)]
pub enum MonorepoType {
    Lerna,
    NxWorkspace,
    RushJs,
    YarnWorkspaces,
    PnpmWorkspaces,
    NpmWorkspaces,
}

/// Lerna monorepo detector
pub struct LernaDetector;

impl BuildSystemDetector for LernaDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Lerna
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "lerna.json")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let lerna_json_path = utils::get_file_path(project_root, "lerna.json");
        let content = utils::read_file(&lerna_json_path)?;
        let lerna_config: JsonValue = serde_json::from_str(&content)
            .context("Failed to parse lerna.json")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("lerna run build".to_string());
        commands.test = Some("lerna run test".to_string());
        commands.lint = Some("lerna run lint".to_string());
        commands.clean = Some("lerna clean".to_string());
        commands.custom.insert("bootstrap".to_string(), "lerna bootstrap".to_string());
        commands.custom.insert("publish".to_string(), "lerna publish".to_string());

        // Get workspace packages
        let mut workspaces = vec![];
        if let Some(packages) = lerna_config.get("packages").and_then(|p| p.as_array()) {
            for package in packages {
                if let Some(path) = package.as_str() {
                    workspaces.push(path.to_string());
                }
            }
        }

        Ok(BuildConfig {
            system: BuildSystem::Lerna,
            root_path: project_root.to_path_buf(),
            config_files: vec![lerna_json_path],
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }
}

/// Nx workspace detector
pub struct NxDetector;

impl BuildSystemDetector for NxDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Nx
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "nx.json") || 
        utils::has_file(project_root, "workspace.json")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let mut config_files = vec![];
        
        if utils::has_file(project_root, "nx.json") {
            config_files.push(utils::get_file_path(project_root, "nx.json"));
        }
        if utils::has_file(project_root, "workspace.json") {
            config_files.push(utils::get_file_path(project_root, "workspace.json"));
        }

        let mut commands = BuildCommands::default();
        commands.build = Some("nx run-many --target=build --all".to_string());
        commands.test = Some("nx run-many --target=test --all".to_string());
        commands.lint = Some("nx run-many --target=lint --all".to_string());
        commands.custom.insert("affected:build".to_string(), "nx affected --target=build".to_string());
        commands.custom.insert("affected:test".to_string(), "nx affected --target=test".to_string());
        commands.custom.insert("graph".to_string(), "nx graph".to_string());

        Ok(BuildConfig {
            system: BuildSystem::Nx,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }
}

/// Rush.js monorepo detector
pub struct RushDetector;

impl BuildSystemDetector for RushDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Rush
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "rush.json")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let rush_json_path = utils::get_file_path(project_root, "rush.json");
        let content = utils::read_file(&rush_json_path)?;
        let _rush_config: JsonValue = serde_json::from_str(&content)
            .context("Failed to parse rush.json")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("rush build".to_string());
        commands.test = Some("rush test".to_string());
        commands.custom.insert("install".to_string(), "rush install".to_string());
        commands.custom.insert("update".to_string(), "rush update".to_string());
        commands.custom.insert("rebuild".to_string(), "rush rebuild".to_string());
        commands.custom.insert("publish".to_string(), "rush publish".to_string());

        Ok(BuildConfig {
            system: BuildSystem::Rush,
            root_path: project_root.to_path_buf(),
            config_files: vec![rush_json_path],
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }
}

/// Yarn Workspaces detector
pub struct YarnWorkspacesDetector;

impl BuildSystemDetector for YarnWorkspacesDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::YarnWorkspaces
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        if !utils::has_file(project_root, "package.json") {
            return false;
        }

        // Yarn workspaces are identified by having workspaces AND yarn.lock
        if !utils::has_file(project_root, "yarn.lock") {
            return false;
        }

        // Check if package.json has workspaces field
        if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "package.json")) {
            if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                return package_json.get("workspaces").is_some();
            }
        }
        false
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let package_json_path = utils::get_file_path(project_root, "package.json");
        let content = utils::read_file(&package_json_path)?;
        let package_json: JsonValue = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;

        let mut commands = BuildCommands::default();
        
        // Yarn workspaces commands
        commands.custom.insert("install".to_string(), "yarn install".to_string());
        commands.custom.insert("workspaces:info".to_string(), "yarn workspaces info".to_string());
        
        // If there are scripts in root package.json, use them
        if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
            if scripts.contains_key("build") {
                commands.build = Some("yarn build".to_string());
            }
            if scripts.contains_key("test") {
                commands.test = Some("yarn test".to_string());
            }
            if scripts.contains_key("lint") {
                commands.lint = Some("yarn lint".to_string());
            }
        } else {
            // Default to running in all workspaces
            commands.build = Some("yarn workspaces run build".to_string());
            commands.test = Some("yarn workspaces run test".to_string());
            commands.lint = Some("yarn workspaces run lint".to_string());
        }

        // Get workspace packages
        let mut workspaces = vec![];
        if let Some(ws) = package_json.get("workspaces") {
            match ws {
                JsonValue::Array(arr) => {
                    for item in arr {
                        if let Some(path) = item.as_str() {
                            workspaces.push(path.to_string());
                        }
                    }
                }
                JsonValue::Object(obj) => {
                    if let Some(packages) = obj.get("packages").and_then(|p| p.as_array()) {
                        for package in packages {
                            if let Some(path) = package.as_str() {
                                workspaces.push(path.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut config_files = vec![package_json_path];
        if utils::has_file(project_root, "yarn.lock") {
            config_files.push(utils::get_file_path(project_root, "yarn.lock"));
        }

        Ok(BuildConfig {
            system: BuildSystem::YarnWorkspaces,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }
}

/// pnpm Workspaces detector
pub struct PnpmWorkspacesDetector;

impl BuildSystemDetector for PnpmWorkspacesDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::PnpmWorkspaces
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "pnpm-workspace.yaml") ||
        utils::has_file(project_root, "pnpm-workspace.yml")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let workspace_file = if utils::has_file(project_root, "pnpm-workspace.yaml") {
            "pnpm-workspace.yaml"
        } else {
            "pnpm-workspace.yml"
        };
        
        let workspace_path = utils::get_file_path(project_root, workspace_file);

        let mut commands = BuildCommands::default();
        commands.build = Some("pnpm -r build".to_string());
        commands.test = Some("pnpm -r test".to_string());
        commands.lint = Some("pnpm -r lint".to_string());
        commands.custom.insert("install".to_string(), "pnpm install".to_string());
        commands.custom.insert("update".to_string(), "pnpm update -r".to_string());

        let mut config_files = vec![workspace_path];
        if utils::has_file(project_root, "package.json") {
            config_files.push(utils::get_file_path(project_root, "package.json"));
        }
        if utils::has_file(project_root, "pnpm-lock.yaml") {
            config_files.push(utils::get_file_path(project_root, "pnpm-lock.yaml"));
        }

        Ok(BuildConfig {
            system: BuildSystem::PnpmWorkspaces,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }
}

/// npm Workspaces detector (npm 7+)
pub struct NpmWorkspacesDetector;

impl BuildSystemDetector for NpmWorkspacesDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::NpmWorkspaces
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        if !utils::has_file(project_root, "package.json") {
            return false;
        }

        // Check if package.json has workspaces field and package-lock.json exists
        if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "package.json")) {
            if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                return package_json.get("workspaces").is_some() && 
                       utils::has_file(project_root, "package-lock.json");
            }
        }
        false
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let package_json_path = utils::get_file_path(project_root, "package.json");
        let content = utils::read_file(&package_json_path)?;
        let package_json: JsonValue = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("npm run build --workspaces".to_string());
        commands.test = Some("npm run test --workspaces".to_string());
        commands.lint = Some("npm run lint --workspaces".to_string());
        commands.custom.insert("install".to_string(), "npm install".to_string());
        commands.custom.insert("list".to_string(), "npm ls --workspaces".to_string());

        let mut config_files = vec![package_json_path];
        if utils::has_file(project_root, "package-lock.json") {
            config_files.push(utils::get_file_path(project_root, "package-lock.json"));
        }

        Ok(BuildConfig {
            system: BuildSystem::NpmWorkspaces,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }
}

/// Detect workspace packages in a monorepo
pub fn detect_workspace_packages(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut packages = vec![];

    // Check for different monorepo configurations
    if utils::has_file(project_root, "lerna.json") {
        // Lerna packages
        if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "lerna.json")) {
            if let Ok(config) = serde_json::from_str::<JsonValue>(&content) {
                if let Some(pkg_patterns) = config.get("packages").and_then(|p| p.as_array()) {
                    for pattern in pkg_patterns {
                        if let Some(path) = pattern.as_str() {
                            // TODO: Expand glob patterns
                            packages.push(project_root.join(path));
                        }
                    }
                }
            }
        }
    }

    // Check package.json for workspaces (Yarn/npm)
    if utils::has_file(project_root, "package.json") {
        if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "package.json")) {
            if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                if let Some(ws) = package_json.get("workspaces") {
                    match ws {
                        JsonValue::Array(arr) => {
                            for item in arr {
                                if let Some(path) = item.as_str() {
                                    // TODO: Expand glob patterns
                                    packages.push(project_root.join(path));
                                }
                            }
                        }
                        JsonValue::Object(obj) => {
                            if let Some(pkg_arr) = obj.get("packages").and_then(|p| p.as_array()) {
                                for package in pkg_arr {
                                    if let Some(path) = package.as_str() {
                                        // TODO: Expand glob patterns
                                        packages.push(project_root.join(path));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(packages)
}