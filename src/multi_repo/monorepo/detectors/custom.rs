//! Custom monorepo detector

use super::super::types::{WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::super::utils::find_subprojects;
use super::WorkspaceDetector;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// Detector for custom monorepo structures
pub struct CustomDetector;

#[async_trait]
impl WorkspaceDetector for CustomDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
        // Look for common patterns
        let common_patterns = vec!["packages/*", "apps/*", "services/*", "libs/*", "modules/*"];

        let mut found_projects = Vec::new();

        for pattern in &common_patterns {
            let projects = find_subprojects(root, &[pattern.to_string()], "package.json").await?;
            found_projects.extend(projects);
        }

        // Also check for Cargo.toml
        for pattern in &common_patterns {
            let projects = find_subprojects(root, &[pattern.to_string()], "Cargo.toml").await?;
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

    fn workspace_type(&self) -> &'static str {
        "custom"
    }
}