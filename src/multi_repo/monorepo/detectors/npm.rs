//! NPM/Yarn workspace detector

use super::super::types::{WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::{extract_dependencies, find_shared_configs, find_subprojects};
use super::WorkspaceDetector;
use crate::core::utils::FileUtils;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Detector for NPM/Yarn workspaces
pub struct NpmDetector;

#[async_trait]
impl WorkspaceDetector for NpmDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
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
                let subprojects = find_subprojects(root, &patterns, "package.json").await?;

                return Ok(Some(WorkspaceLayout {
                    root: root.to_path_buf(),
                    workspace_type: WorkspaceType::NpmWorkspace,
                    subprojects,
                    config: WorkspaceConfig {
                        patterns,
                        excludes: vec![],
                        dependencies: extract_dependencies(&package_json),
                        build_config: Some(package_json.clone()),
                    },
                    shared_configs: find_shared_configs(
                        root,
                        &["tsconfig.json", ".eslintrc", ".prettierrc"],
                    )
                    .await?,
                }));
            }
        }

        Ok(None)
    }

    fn workspace_type(&self) -> &'static str {
        "npm"
    }
}