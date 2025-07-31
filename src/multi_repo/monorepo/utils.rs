//! Utility functions for monorepo analysis

use super::types::{SubprojectInfo};
use crate::core::constants::{build_systems, languages};
use crate::core::utils::FileUtils;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find subprojects matching the given patterns
pub async fn find_subprojects(
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

                    let (name, language, build_system, package_config) = if config_file == "package.json" {
                        let content = FileUtils::read_with_context(&path, "subproject package.json").await?;
                        let package_json: serde_json::Value = serde_json::from_str(&content)?;

                        let name = package_json
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&relative_path.to_string_lossy())
                            .to_string();

                        let lang = Some(languages::TYPESCRIPT.to_string());
                        let build = detect_npm_build_system(&package_json);
                        (name, lang, build, Some(package_json))
                    } else if config_file == "Cargo.toml" {
                        let content = FileUtils::read_with_context(&path, "subproject Cargo.toml").await?;
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
    analyze_dependencies(&mut subprojects).await?;

    Ok(subprojects)
}

/// Find Nx projects by looking for project.json files
pub async fn find_nx_projects(root: &Path) -> Result<Vec<SubprojectInfo>> {
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
pub async fn analyze_dependencies(subprojects: &mut [SubprojectInfo]) -> Result<()> {
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
pub async fn find_shared_configs(root: &Path, patterns: &[&str]) -> Result<Vec<PathBuf>> {
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
pub fn extract_dependencies(package_json: &serde_json::Value) -> HashMap<String, String> {
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
pub fn extract_cargo_dependencies(cargo_toml: &toml::Value) -> HashMap<String, String> {
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
pub fn detect_npm_build_system(package_json: &serde_json::Value) -> Option<String> {
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