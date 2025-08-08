//! Tree visualization and rendering

use crate::project::structure_analyzer::types::{DirectoryNode, ProjectStructure};

/// Renders project structures as tree visualizations
pub struct TreeRenderer;

impl TreeRenderer {
    /// Create a new tree renderer
    pub fn new() -> Self {
        Self
    }

    /// Get a formatted tree summary of the project structure
    pub fn get_file_tree_summary(&self, structure: &ProjectStructure, max_depth: usize) -> String {
        let mut result = String::new();
        self.format_tree_node(&structure.directory_tree, &mut result, 0, max_depth, "");
        result
    }

    fn format_tree_node(
        &self,
        node: &DirectoryNode,
        result: &mut String,
        depth: usize,
        max_depth: usize,
        prefix: &str,
    ) {
        if depth > max_depth {
            return;
        }

        let connector = if depth == 0 { "" } else { "├── " };
        let name = if node.is_directory {
            format!("{}/", node.name)
        } else {
            node.name.clone()
        };

        result.push_str(&format!("{prefix}{connector}{name}"));

        if node.is_directory && node.file_count > 0 {
            result.push_str(&format!(" ({} files)", node.file_count));
        }

        result.push('\n');

        if node.is_directory && depth < max_depth {
            let child_prefix = format!("{prefix}│   ");
            for (i, child) in node.children.iter().enumerate() {
                let is_last = i == node.children.len() - 1;
                let _child_connector = if is_last { "└── " } else { "├── " };
                let next_prefix = if is_last {
                    format!("{prefix}    ")
                } else {
                    child_prefix.clone()
                };

                self.format_tree_node(child, result, depth + 1, max_depth, &next_prefix);
            }
        }
    }
}

impl Default for TreeRenderer {
    fn default() -> Self {
        Self::new()
    }
}