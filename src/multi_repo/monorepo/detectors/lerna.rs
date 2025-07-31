//! Lerna monorepo detector

use super::super::types::{WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::{find_shared_configs, find_subprojects};
use super::WorkspaceDetector;
use crate::core::utils::FileUtils;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// Detector for Lerna monorepos
pub struct LernaDetector;

#[async_trait]
impl WorkspaceDetector for LernaDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
        let lerna_json_path = root.join("lerna.json");
        if !lerna_json_path.exists() {
            return Ok(None);
        }

        let content = FileUtils::read_with_context(&lerna_json_path, "lerna.json file").await?;
        let lerna_config: serde_json::Value = serde_json::from_str(&content)?;

        let patterns = if let Some(packages) = lerna_config.get("packages").and_then(|v| v.as_array()) {
            packages
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            vec!["packages/*".to_string()]
        };

        let subprojects = find_subprojects(root, &patterns, "package.json").await?;

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
            shared_configs: find_shared_configs(
                root,
                &["tsconfig.json", ".eslintrc", ".prettierrc"],
            )
            .await?,
        }))
    }

    fn workspace_type(&self) -> &'static str {
        "lerna"
    }
}