//! Cargo workspace detector

use super::super::types::{WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::{extract_cargo_dependencies, find_shared_configs, find_subprojects};
use super::WorkspaceDetector;
use crate::core::utils::FileUtils;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Detector for Cargo workspaces
pub struct CargoDetector;

#[async_trait]
impl WorkspaceDetector for CargoDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
        let cargo_toml_path = root.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&cargo_toml_path, "Cargo.toml file").await?;
        let cargo_toml: toml::Value = toml::from_str(&content)?;

        if let Some(workspace) = cargo_toml.get("workspace") {
            let patterns = if let Some(members) = workspace.get("members").and_then(|v| v.as_array()) {
                members
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                vec![]
            };

            if !patterns.is_empty() {
                let subprojects = find_subprojects(root, &patterns, "Cargo.toml").await?;

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
                        dependencies: extract_cargo_dependencies(&cargo_toml),
                        build_config: Some(serde_json::to_value(&cargo_toml)?),
                    },
                    shared_configs: find_shared_configs(
                        root,
                        &["rustfmt.toml", ".cargo/config.toml"],
                    )
                    .await?,
                }));
            }
        }

        Ok(None)
    }

    fn workspace_type(&self) -> &'static str {
        "cargo"
    }
}