//! PNPM workspace detector

use super::super::types::{WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::{find_shared_configs, find_subprojects};
use super::WorkspaceDetector;
use crate::core::utils::FileUtils;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// Detector for PNPM workspaces
pub struct PnpmDetector;

#[async_trait]
impl WorkspaceDetector for PnpmDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
        let workspace_yaml_path = root.join("pnpm-workspace.yaml");
        if !workspace_yaml_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&workspace_yaml_path, "workspace.yaml file").await?;
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

            let subprojects = find_subprojects(root, &patterns, "package.json").await?;

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
                shared_configs: find_shared_configs(
                    root,
                    &["tsconfig.json", ".eslintrc", ".prettierrc"],
                )
                .await?,
            }));
        }

        Ok(None)
    }

    fn workspace_type(&self) -> &'static str {
        "pnpm"
    }
}