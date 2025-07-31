//! Nx workspace detector

use super::super::types::{SubprojectInfo, WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::{find_nx_projects, find_shared_configs};
use super::WorkspaceDetector;
use crate::core::constants::languages;
use crate::core::utils::FileUtils;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Detector for Nx workspaces
pub struct NxDetector;

#[async_trait]
impl WorkspaceDetector for NxDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
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
            let content = FileUtils::read_with_context(&workspace_json_path, "workspace.json file").await?;
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
            subprojects = find_nx_projects(root).await?;
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
                shared_configs: find_shared_configs(
                    root,
                    &["tsconfig.base.json", ".eslintrc.json"],
                )
                .await?,
            }));
        }

        Ok(None)
    }

    fn workspace_type(&self) -> &'static str {
        "nx"
    }
}