//! Rush monorepo detector

use super::super::types::{SubprojectInfo, WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::find_shared_configs;
use super::WorkspaceDetector;
use crate::core::constants::languages;
use crate::core::utils::FileUtils;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Detector for Rush monorepos
pub struct RushDetector;

#[async_trait]
impl WorkspaceDetector for RushDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
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
                shared_configs: find_shared_configs(root, &["common/config/rush/.npmrc"])
                    .await?,
            }));
        }

        Ok(None)
    }

    fn workspace_type(&self) -> &'static str {
        "rush"
    }
}