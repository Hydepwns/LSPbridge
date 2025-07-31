//! Monorepo detection logic

use std::fs;
use std::path::{Path, PathBuf};

/// Detector for monorepo structures
pub struct MonorepoDetector;

impl MonorepoDetector {
    /// Create a new monorepo detector
    pub fn new() -> Self {
        Self
    }

    /// Detect if a project is a monorepo
    pub fn detect(&self, root: &Path, subprojects: &[PathBuf]) -> bool {
        // Common monorepo indicators
        if root.join("lerna.json").exists() {
            return true;
        }

        if root.join("pnpm-workspace.yaml").exists() {
            return true;
        }

        if root.join("rush.json").exists() {
            return true;
        }

        // Check for yarn workspaces
        if let Ok(content) = fs::read_to_string(root.join("package.json")) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if json.get("workspaces").is_some() {
                    return true;
                }
            }
        }

        // Check for nx
        if root.join("nx.json").exists() {
            return true;
        }

        // Multiple subprojects might indicate a monorepo
        subprojects.len() > 2
    }
}

impl Default for MonorepoDetector {
    fn default() -> Self {
        Self::new()
    }
}