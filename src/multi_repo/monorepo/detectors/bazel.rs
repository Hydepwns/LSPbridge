//! Bazel workspace detector

use super::super::types::{SubprojectInfo, WorkspaceConfig, WorkspaceLayout, WorkspaceType};
use super::WorkspaceDetector;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Detector for Bazel workspaces
pub struct BazelDetector;

#[async_trait]
impl WorkspaceDetector for BazelDetector {
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
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

    fn workspace_type(&self) -> &'static str {
        "bazel"
    }
}