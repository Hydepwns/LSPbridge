//! Monorepo detection and analysis

use crate::core::constants::{build_systems, languages};
use crate::core::utils::FileUtils;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Information about a subproject in a monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubprojectInfo {
    /// Subproject name
    pub name: String,

    /// Path relative to monorepo root
    pub relative_path: PathBuf,

    /// Absolute path
    pub absolute_path: PathBuf,

    /// Primary language
    pub language: Option<String>,

    /// Build system
    pub build_system: Option<String>,

    /// Dependencies on other subprojects
    pub internal_deps: Vec<String>,

    /// External dependencies
    pub external_deps: Vec<String>,

    /// Package configuration (package.json, Cargo.toml, etc.)
    pub package_config: Option<serde_json::Value>,
}

/// Workspace layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    /// Monorepo root path
    pub root: PathBuf,

    /// Workspace type
    pub workspace_type: WorkspaceType,

    /// Subprojects in the workspace
    pub subprojects: Vec<SubprojectInfo>,

    /// Workspace configuration
    pub config: WorkspaceConfig,

    /// Shared configuration files
    pub shared_configs: Vec<PathBuf>,
}

/// Types of monorepo workspaces
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkspaceType {
    /// npm/Yarn workspaces
    NpmWorkspace,

    /// pnpm workspace
    PnpmWorkspace,

    /// Lerna monorepo
    Lerna,

    /// Cargo workspace
    CargoWorkspace,

    /// Bazel workspace
    Bazel,

    /// Nx workspace
    Nx,

    /// Rush monorepo
    Rush,

    /// Custom/Unknown
    Custom,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace patterns (globs)
    pub patterns: Vec<String>,

    /// Excluded patterns
    pub excludes: Vec<String>,

    /// Workspace-level dependencies
    pub dependencies: HashMap<String, String>,

    /// Build tool configuration
    pub build_config: Option<serde_json::Value>,
}

/// Detects and analyzes monorepo structures
pub struct MonorepoDetector;

impl MonorepoDetector {
    /// Detect monorepo structure in the given directory
    pub async fn detect(root: &Path) -> Result<Option<WorkspaceLayout>> {
        // Check for various monorepo indicators
        if let Some(layout) = Self::detect_npm_workspace(root).await? {
            return Ok(Some(layout));
        }

        if let Some(layout) = Self::detect_pnpm_workspace(root).await? {
            return Ok(Some(layout));
        }

        if let Some(layout) = Self::detect_lerna(root).await? {
            return Ok(Some(layout));
        }

        if let Some(layout) = Self::detect_cargo_workspace(root).await? {
            return Ok(Some(layout));
        }

        if let Some(layout) = Self::detect_nx_workspace(root).await? {
            return Ok(Some(layout));
        }

        if let Some(layout) = Self::detect_rush_monorepo(root).await? {
            return Ok(Some(layout));
        }

        if let Some(layout) = Self::detect_bazel_workspace(root).await? {
            return Ok(Some(layout));
        }

        // Check for custom monorepo structure
        if let Some(layout) = Self::detect_custom_monorepo(root).await? {
            return Ok(Some(layout));
        }

        Ok(None)
    }

    /// Detect npm/Yarn workspace
    async fn detect_npm_workspace(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let package_json_path = root.join("package.json");
        if !package_json_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&package_json_path, "package.json file").await?;
        let package_json: serde_json::Value = serde_json::from_str(&content)?;

        // Check for workspaces field
        if let Some(workspaces) = package_json.get("workspaces") {
            let patterns = if let Some(arr) = workspaces.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            } else if let Some(obj) = workspaces.as_object() {
                if let Some(packages) = obj.get("packages").and_then(|v| v.as_array()) {
                    packages
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            if !patterns.is_empty() {
                let subprojects = Self::find_subprojects(root, &patterns, "package.json").await?;

                return Ok(Some(WorkspaceLayout {
                    root: root.to_path_buf(),
                    workspace_type: WorkspaceType::NpmWorkspace,
                    subprojects,
                    config: WorkspaceConfig {
                        patterns,
                        excludes: vec![],
                        dependencies: Self::extract_dependencies(&package_json),
                        build_config: Some(package_json.clone()),
                    },
                    shared_configs: Self::find_shared_configs(
                        root,
                        &["tsconfig.json", ".eslintrc", ".prettierrc"],
                    )
                    .await?,
                }));
            }
        }

        Ok(None)
    }

    /// Detect pnpm workspace
    async fn detect_pnpm_workspace(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let workspace_yaml_path = root.join("pnpm-workspace.yaml");
        if !workspace_yaml_path.exists() {
            return Ok(None);
        }

        let content =
            FileUtils::read_with_context(&workspace_yaml_path, "workspace.yaml file").await?;
        let workspace_config: serde_yaml::Value = serde_yaml::from_str(&content)?;

        if let Some(packages) = workspace_config
            .get("packages")
            .and_then(|v| v.as_sequence())
        {
            let patterns: Vec<String> = packages
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();

            let subprojects = Self::find_subprojects(root, &patterns, "package.json").await?;

            return Ok(Some(WorkspaceLayout {
                root: root.to_path_buf(),
                workspace_type: WorkspaceType::PnpmWorkspace,
                subprojects,
                config: WorkspaceConfig {
                    patterns,
                    excludes: vec![],
                    dependencies: HashMap::new(),
                    build_config: Some(serde_json::to_value(&workspace_config)?),
                },
                shared_configs: Self::find_shared_configs(
                    root,
                    &["tsconfig.json", ".eslintrc", ".prettierrc"],
                )
                .await?,
            }));
        }

        Ok(None)
    }

    /// Detect Lerna monorepo
    async fn detect_lerna(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let lerna_json_path = root.join("lerna.json");
        if !lerna_json_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&lerna_json_path, "lerna.json file").await?;
        let lerna_config: serde_json::Value = serde_json::from_str(&content)?;

        let patterns =
            if let Some(packages) = lerna_config.get("packages").and_then(|v| v.as_array()) {
                packages
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                vec!["packages/*".to_string()]
            };

        let subprojects = Self::find_subprojects(root, &patterns, "package.json").await?;

        Ok(Some(WorkspaceLayout {
            root: root.to_path_buf(),
            workspace_type: WorkspaceType::Lerna,
            subprojects,
            config: WorkspaceConfig {
                patterns,
                excludes: vec![],
                dependencies: HashMap::new(),
                build_config: Some(lerna_config),
            },
            shared_configs: Self::find_shared_configs(
                root,
                &["tsconfig.json", ".eslintrc", ".prettierrc"],
            )
            .await?,
        }))
    }

    /// Detect Cargo workspace
    async fn detect_cargo_workspace(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let cargo_toml_path = root.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&cargo_toml_path, "Cargo.toml file").await?;
        let cargo_toml: toml::Value = toml::from_str(&content)?;

        if let Some(workspace) = cargo_toml.get("workspace") {
            let patterns =
                if let Some(members) = workspace.get("members").and_then(|v| v.as_array()) {
                    members
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    vec![]
                };

            if !patterns.is_empty() {
                let subprojects = Self::find_subprojects(root, &patterns, "Cargo.toml").await?;

                return Ok(Some(WorkspaceLayout {
                    root: root.to_path_buf(),
                    workspace_type: WorkspaceType::CargoWorkspace,
                    subprojects,
                    config: WorkspaceConfig {
                        patterns,
                        excludes: workspace
                            .get("exclude")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default(),
                        dependencies: Self::extract_cargo_dependencies(&cargo_toml),
                        build_config: Some(serde_json::to_value(&cargo_toml)?),
                    },
                    shared_configs: Self::find_shared_configs(
                        root,
                        &["rustfmt.toml", ".cargo/config.toml"],
                    )
                    .await?,
                }));
            }
        }

        Ok(None)
    }

    /// Detect Nx workspace
    async fn detect_nx_workspace(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let nx_json_path = root.join("nx.json");
        if !nx_json_path.exists() {
            return Ok(None);
        }

        // Nx uses workspace.json or project.json files
        let workspace_json_path = root.join("workspace.json");
        let has_workspace_json = workspace_json_path.exists();

        // Find all project.json files
        let mut subprojects = Vec::new();

        if has_workspace_json {
            let content =
                FileUtils::read_with_context(&workspace_json_path, "workspace.json file").await?;
            let workspace: serde_json::Value = serde_json::from_str(&content)?;

            if let Some(projects) = workspace.get("projects").and_then(|v| v.as_object()) {
                for (name, config) in projects {
                    if let Some(root_path) = config.get("root").and_then(|v| v.as_str()) {
                        let abs_path = root.join(root_path);
                        subprojects.push(SubprojectInfo {
                            name: name.clone(),
                            relative_path: PathBuf::from(root_path),
                            absolute_path: abs_path,
                            language: Some(languages::TYPESCRIPT.to_string()),
                            build_system: Some("nx".to_string()),
                            internal_deps: vec![],
                            external_deps: vec![],
                            package_config: Some(config.clone()),
                        });
                    }
                }
            }
        } else {
            // Look for project.json files
            subprojects = Self::find_nx_projects(root).await?;
        }

        if !subprojects.is_empty() {
            return Ok(Some(WorkspaceLayout {
                root: root.to_path_buf(),
                workspace_type: WorkspaceType::Nx,
                subprojects,
                config: WorkspaceConfig {
                    patterns: vec![],
                    excludes: vec![],
                    dependencies: HashMap::new(),
                    build_config: None,
                },
                shared_configs: Self::find_shared_configs(
                    root,
                    &["tsconfig.base.json", ".eslintrc.json"],
                )
                .await?,
            }));
        }

        Ok(None)
    }

    /// Detect Rush monorepo
    async fn detect_rush_monorepo(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let rush_json_path = root.join("rush.json");
        if !rush_json_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&rush_json_path, "rush.json file").await?;
        let rush_config: serde_json::Value = serde_json::from_str(&content)?;

        let mut subprojects = Vec::new();

        if let Some(projects) = rush_config.get("projects").and_then(|v| v.as_array()) {
            for project in projects {
                if let (Some(name), Some(path)) = (
                    project.get("packageName").and_then(|v| v.as_str()),
                    project.get("projectFolder").and_then(|v| v.as_str()),
                ) {
                    let abs_path = root.join(path);
                    let package_json_path = abs_path.join("package.json");

                    let package_config = if package_json_path.exists() {
                        let pkg_content = FileUtils::read_with_context(
                            &package_json_path,
                            "subproject package.json",
                        )
                        .await?;
                        Some(serde_json::from_str(&pkg_content)?)
                    } else {
                        None
                    };

                    subprojects.push(SubprojectInfo {
                        name: name.to_string(),
                        relative_path: PathBuf::from(path),
                        absolute_path: abs_path,
                        language: Some(languages::TYPESCRIPT.to_string()),
                        build_system: Some("rush".to_string()),
                        internal_deps: vec![],
                        external_deps: vec![],
                        package_config,
                    });
                }
            }
        }

        if !subprojects.is_empty() {
            return Ok(Some(WorkspaceLayout {
                root: root.to_path_buf(),
                workspace_type: WorkspaceType::Rush,
                subprojects,
                config: WorkspaceConfig {
                    patterns: vec![],
                    excludes: vec![],
                    dependencies: HashMap::new(),
                    build_config: Some(rush_config),
                },
                shared_configs: Self::find_shared_configs(root, &["common/config/rush/.npmrc"])
                    .await?,
            }));
        }

        Ok(None)
    }

    /// Detect Bazel workspace
    async fn detect_bazel_workspace(root: &Path) -> Result<Option<WorkspaceLayout>> {
        let workspace_path = root.join("WORKSPACE");
        let workspace_bazel_path = root.join("WORKSPACE.bazel");

        if !workspace_path.exists() && !workspace_bazel_path.exists() {
            return Ok(None);
        }

        // Find BUILD or BUILD.bazel files
        let mut subprojects = Vec::new();

        for entry in WalkDir::new(root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                if name == "BUILD" || name == "BUILD.bazel" {
                    if let Some(parent) = path.parent() {
                        let relative_path = parent.strip_prefix(root).unwrap_or(parent);

                        subprojects.push(SubprojectInfo {
                            name: relative_path.to_string_lossy().to_string(),
                            relative_path: relative_path.to_path_buf(),
                            absolute_path: parent.to_path_buf(),
                            language: None,
                            build_system: Some("bazel".to_string()),
                            internal_deps: vec![],
                            external_deps: vec![],
                            package_config: None,
                        });
                    }
                }
            }
        }

        if !subprojects.is_empty() {
            return Ok(Some(WorkspaceLayout {
                root: root.to_path_buf(),
                workspace_type: WorkspaceType::Bazel,
                subprojects,
                config: WorkspaceConfig {
                    patterns: vec![],
                    excludes: vec![],
                    dependencies: HashMap::new(),
                    build_config: None,
                },
                shared_configs: vec![],
            }));
        }

        Ok(None)
    }

    /// Detect custom monorepo structure
    async fn detect_custom_monorepo(root: &Path) -> Result<Option<WorkspaceLayout>> {
        // Look for common patterns
        let common_patterns = vec!["packages/*", "apps/*", "services/*", "libs/*", "modules/*"];

        let mut found_projects = Vec::new();

        for pattern in &common_patterns {
            let projects =
                Self::find_subprojects(root, &[pattern.to_string()], "package.json").await?;
            found_projects.extend(projects);
        }

        // Also check for Cargo.toml
        for pattern in &common_patterns {
            let projects =
                Self::find_subprojects(root, &[pattern.to_string()], "Cargo.toml").await?;
            found_projects.extend(projects);
        }

        // Deduplicate by path
        found_projects.sort_by(|a, b| a.absolute_path.cmp(&b.absolute_path));
        found_projects.dedup_by(|a, b| a.absolute_path == b.absolute_path);

        if found_projects.len() >= 2 {
            return Ok(Some(WorkspaceLayout {
                root: root.to_path_buf(),
                workspace_type: WorkspaceType::Custom,
                subprojects: found_projects,
                config: WorkspaceConfig {
                    patterns: common_patterns.into_iter().map(|s| s.to_string()).collect(),
                    excludes: vec![],
                    dependencies: HashMap::new(),
                    build_config: None,
                },
                shared_configs: vec![],
            }));
        }

        Ok(None)
    }

    /// Find subprojects matching patterns
    async fn find_subprojects(
        root: &Path,
        patterns: &[String],
        config_file: &str,
    ) -> Result<Vec<SubprojectInfo>> {
        let mut subprojects = Vec::new();

        for pattern in patterns {
            let glob_pattern = root.join(pattern).join(config_file);
            let glob_str = glob_pattern.to_string_lossy();

            for entry in glob::glob(&glob_str)? {
                if let Ok(path) = entry {
                    if let Some(parent) = path.parent() {
                        let relative_path = parent.strip_prefix(root).unwrap_or(parent);

                        let (name, language, build_system, package_config) = if config_file
                            == "package.json"
                        {
                            let content =
                                FileUtils::read_with_context(&path, "subproject package.json")
                                    .await?;
                            let package_json: serde_json::Value = serde_json::from_str(&content)?;

                            let name = package_json
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&relative_path.to_string_lossy())
                                .to_string();

                            let lang = Some(languages::TYPESCRIPT.to_string());
                            let build = Self::detect_npm_build_system(&package_json);
                            (name, lang, build, Some(package_json))
                        } else if config_file == "Cargo.toml" {
                            let content =
                                FileUtils::read_with_context(&path, "subproject Cargo.toml")
                                    .await?;
                            let cargo_toml: toml::Value = toml::from_str(&content)?;

                            let name = cargo_toml
                                .get("package")
                                .and_then(|p| p.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or(&relative_path.to_string_lossy())
                                .to_string();

                            let lang = Some(languages::RUST.to_string());
                            let build = Some(build_systems::CARGO.to_string());
                            (name, lang, build, Some(serde_json::to_value(&cargo_toml)?))
                        } else {
                            (
                                relative_path.to_string_lossy().to_string(),
                                None,
                                None,
                                None,
                            )
                        };

                        subprojects.push(SubprojectInfo {
                            name,
                            relative_path: relative_path.to_path_buf(),
                            absolute_path: parent.to_path_buf(),
                            language,
                            build_system,
                            internal_deps: vec![],
                            external_deps: vec![],
                            package_config,
                        });
                    }
                }
            }
        }

        // Analyze dependencies
        Self::analyze_dependencies(&mut subprojects).await?;

        Ok(subprojects)
    }

    /// Find Nx projects by looking for project.json files
    async fn find_nx_projects(root: &Path) -> Result<Vec<SubprojectInfo>> {
        let mut projects = Vec::new();

        for entry in WalkDir::new(root)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "project.json")
        {
            let path = entry.path();
            if let Some(parent) = path.parent() {
                let content = FileUtils::read_with_context(path, "project.json file").await?;
                let project_config: serde_json::Value = serde_json::from_str(&content)?;

                let name = project_config
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&parent.file_name().unwrap().to_string_lossy())
                    .to_string();

                let relative_path = parent.strip_prefix(root).unwrap_or(parent);

                projects.push(SubprojectInfo {
                    name,
                    relative_path: relative_path.to_path_buf(),
                    absolute_path: parent.to_path_buf(),
                    language: Some(languages::TYPESCRIPT.to_string()),
                    build_system: Some("nx".to_string()),
                    internal_deps: vec![],
                    external_deps: vec![],
                    package_config: Some(project_config),
                });
            }
        }

        Ok(projects)
    }

    /// Analyze dependencies between subprojects
    async fn analyze_dependencies(subprojects: &mut [SubprojectInfo]) -> Result<()> {
        let project_names: HashMap<String, usize> = subprojects
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.clone(), i))
            .collect();

        for i in 0..subprojects.len() {
            let mut internal_deps = Vec::new();
            let mut external_deps = Vec::new();

            if let Some(config) = &subprojects[i].package_config {
                // Check dependencies
                for dep_type in &["dependencies", "devDependencies", "peerDependencies"] {
                    if let Some(deps) = config.get(dep_type).and_then(|v| v.as_object()) {
                        for (dep_name, _) in deps {
                            if project_names.contains_key(dep_name) {
                                internal_deps.push(dep_name.clone());
                            } else {
                                external_deps.push(dep_name.clone());
                            }
                        }
                    }
                }
            }

            subprojects[i].internal_deps = internal_deps;
            subprojects[i].external_deps = external_deps;
        }

        Ok(())
    }

    /// Find shared configuration files
    async fn find_shared_configs(root: &Path, patterns: &[&str]) -> Result<Vec<PathBuf>> {
        let mut configs = Vec::new();

        for pattern in patterns {
            let path = root.join(pattern);
            if path.exists() {
                configs.push(path);
            }
        }

        Ok(configs)
    }

    /// Extract dependencies from package.json
    fn extract_dependencies(package_json: &serde_json::Value) -> HashMap<String, String> {
        let mut all_deps = HashMap::new();

        for dep_type in &["dependencies", "devDependencies"] {
            if let Some(deps) = package_json.get(dep_type).and_then(|v| v.as_object()) {
                for (name, version) in deps {
                    if let Some(version_str) = version.as_str() {
                        all_deps.insert(name.clone(), version_str.to_string());
                    }
                }
            }
        }

        all_deps
    }

    /// Extract dependencies from Cargo.toml
    fn extract_cargo_dependencies(cargo_toml: &toml::Value) -> HashMap<String, String> {
        let mut all_deps = HashMap::new();

        if let Some(deps) = cargo_toml.get("dependencies").and_then(|v| v.as_table()) {
            for (name, spec) in deps {
                let version = if let Some(version_str) = spec.as_str() {
                    version_str.to_string()
                } else if let Some(table) = spec.as_table() {
                    table
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string()
                } else {
                    "*".to_string()
                };

                all_deps.insert(name.clone(), version);
            }
        }

        all_deps
    }

    /// Detect npm build system from package.json
    fn detect_npm_build_system(package_json: &serde_json::Value) -> Option<String> {
        if let Some(scripts) = package_json.get("scripts").and_then(|v| v.as_object()) {
            // Check for specific build tools
            for (_, script) in scripts {
                if let Some(script_str) = script.as_str() {
                    if script_str.contains("nx ") {
                        return Some("nx".to_string());
                    }
                    if script_str.contains("lerna ") {
                        return Some("lerna".to_string());
                    }
                    if script_str.contains("rush ") {
                        return Some("rush".to_string());
                    }
                    if script_str.contains("turbo ") {
                        return Some("turbo".to_string());
                    }
                }
            }
        }

        // Check package manager
        if package_json.get("packageManager").is_some() {
            if let Some(pm) = package_json.get("packageManager").and_then(|v| v.as_str()) {
                if pm.starts_with("pnpm") {
                    return Some("pnpm".to_string());
                } else if pm.starts_with("yarn") {
                    return Some(build_systems::YARN.to_string());
                }
            }
        }

        Some(build_systems::NPM.to_string())
    }
}
