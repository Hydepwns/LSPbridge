//! Core structure analysis functionality

use crate::project::structure_analyzer::types::{DirectoryNode, ProjectStructure};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Tree building result
pub type TreeBuildResult = (
    DirectoryNode,
    Vec<PathBuf>,           // source_dirs
    Vec<PathBuf>,           // test_dirs
    Vec<PathBuf>,           // config_files
    Vec<PathBuf>,           // doc_files
    usize,                  // total_files
    HashMap<String, usize>, // file_types
    Vec<PathBuf>,           // subprojects
);

/// Core analyzer for building project structure
pub struct CoreAnalyzer {
    ignore_patterns: HashSet<String>,
    source_indicators: HashSet<String>,
    test_indicators: HashSet<String>,
    config_extensions: HashSet<String>,
    doc_extensions: HashSet<String>,
}

impl CoreAnalyzer {
    pub fn new(
        ignore_patterns: HashSet<String>,
        source_indicators: HashSet<String>,
        test_indicators: HashSet<String>,
        config_extensions: HashSet<String>,
        doc_extensions: HashSet<String>,
    ) -> Self {
        Self {
            ignore_patterns,
            source_indicators,
            test_indicators,
            config_extensions,
            doc_extensions,
        }
    }

    /// Analyze the project structure
    pub fn analyze(&self, project_root: &Path) -> Result<ProjectStructure> {
        let mut structure = ProjectStructure {
            root: project_root.to_path_buf(),
            source_dirs: Vec::new(),
            test_dirs: Vec::new(),
            config_files: Vec::new(),
            documentation_files: Vec::new(),
            total_files: 0,
            file_types: HashMap::new(),
            directory_tree: DirectoryNode {
                name: project_root
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new(""))
                    .to_string_lossy()
                    .into_owned(),
                path: project_root.to_path_buf(),
                is_directory: true,
                children: Vec::new(),
                file_count: 0,
                total_size: 0,
            },
            is_monorepo: false,
            subprojects: Vec::new(),
        };

        // Build directory tree and collect information
        let (
            tree,
            source_dirs,
            test_dirs,
            config_files,
            doc_files,
            total_files,
            file_types,
            subprojects,
        ) = self.build_tree(project_root)?;

        structure.directory_tree = tree;
        structure.source_dirs = source_dirs;
        structure.test_dirs = test_dirs;
        structure.config_files = config_files;
        structure.documentation_files = doc_files;
        structure.total_files = total_files;
        structure.file_types = file_types;
        structure.subprojects = subprojects;

        Ok(structure)
    }

    /// Build the directory tree
    pub fn build_tree(&self, path: &Path) -> Result<TreeBuildResult> {
        let mut source_dirs = Vec::new();
        let mut test_dirs = Vec::new();
        let mut config_files = Vec::new();
        let mut doc_files = Vec::new();
        let mut total_files = 0;
        let mut file_types = HashMap::new();
        let mut subprojects = Vec::new();

        let node = self.build_tree_recursive(
            path,
            path,
            &mut source_dirs,
            &mut test_dirs,
            &mut config_files,
            &mut doc_files,
            &mut total_files,
            &mut file_types,
            &mut subprojects,
        )?;

        Ok((
            node,
            source_dirs,
            test_dirs,
            config_files,
            doc_files,
            total_files,
            file_types,
            subprojects,
        ))
    }

    fn build_tree_recursive(
        &self,
        path: &Path,
        root_path: &Path,
        source_dirs: &mut Vec<PathBuf>,
        test_dirs: &mut Vec<PathBuf>,
        config_files: &mut Vec<PathBuf>,
        doc_files: &mut Vec<PathBuf>,
        total_files: &mut usize,
        file_types: &mut HashMap<String, usize>,
        subprojects: &mut Vec<PathBuf>,
    ) -> Result<DirectoryNode> {
        let mut entries = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().into_owned();

            // Skip ignored patterns
            if self.should_ignore(&file_name) {
                continue;
            }

            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                let mut child_node = DirectoryNode {
                    name: file_name.clone(),
                    path: path.clone(),
                    is_directory: true,
                    children: Vec::new(),
                    file_count: 0,
                    total_size: 0,
                };

                let child = self.build_tree_recursive(
                    &path,
                    root_path,
                    source_dirs,
                    test_dirs,
                    config_files,
                    doc_files,
                    total_files,
                    file_types,
                    subprojects,
                )?;
                child_node.file_count = child.file_count;
                child_node.total_size = child.total_size;
                child_node.children = child.children;

                // Classify directory - test dirs take priority
                if self.is_test_dir(&file_name, &path) {
                    test_dirs.push(path.clone());
                } else if self.is_source_dir(&file_name, &path) {
                    source_dirs.push(path.clone());
                }

                // Check for subprojects (package.json, Cargo.toml, etc. in subdirectories)
                if path != root_path {
                    if path.join("package.json").exists()
                        || path.join("Cargo.toml").exists()
                        || path.join("pyproject.toml").exists()
                        || path.join("go.mod").exists()
                    {
                        subprojects.push(path.clone());
                    }
                }

                entries.push(child_node);
            } else {
                *total_files += 1;

                // Classify file
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();

                    *file_types.entry(ext_str.clone()).or_insert(0) += 1;

                    if self.config_extensions.contains(&ext_str) {
                        config_files.push(path.clone());
                    } else if self.doc_extensions.contains(&ext_str) {
                        doc_files.push(path.clone());
                    }
                }

                // Also check for specific config files without extensions
                let file_name_lower = file_name.to_lowercase();
                if file_name_lower == "dockerfile"
                    || file_name_lower == "makefile"
                    || file_name_lower == "rakefile"
                    || file_name_lower == "procfile"
                    || file_name_lower.starts_with(".env")
                    || file_name_lower.starts_with(".gitignore")
                    || file_name_lower.starts_with(".dockerignore")
                {
                    config_files.push(path.clone());
                }

                entries.push(DirectoryNode {
                    name: file_name,
                    path: path.clone(),
                    is_directory: false,
                    children: Vec::new(),
                    file_count: 1,
                    total_size: metadata.len(),
                });
            }
        }

        // Sort entries: directories first, then files, alphabetically
        entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        // Calculate totals
        let mut node_file_count = 0;
        let mut node_total_size = 0;

        for entry in &entries {
            node_file_count += entry.file_count;
            node_total_size += entry.total_size;
        }

        let node = DirectoryNode {
            name: path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_string_lossy()
                .into_owned(),
            path: path.to_path_buf(),
            is_directory: true,
            children: entries,
            file_count: node_file_count,
            total_size: node_total_size,
        };

        Ok(node)
    }

    pub fn should_ignore(&self, name: &str) -> bool {
        name.starts_with('.') && name != "." && name != ".." || self.ignore_patterns.contains(name)
    }

    fn is_source_dir(&self, name: &str, path: &Path) -> bool {
        let name_lower = name.to_lowercase();

        // Direct match
        if self.source_indicators.contains(&name_lower) {
            return true;
        }

        // Check if it contains source files
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if matches!(
                        ext_str.as_str(),
                        "rs" | "js"
                            | "ts"
                            | "jsx"
                            | "tsx"
                            | "py"
                            | "java"
                            | "go"
                            | "cpp"
                            | "c"
                            | "h"
                            | "hpp"
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn is_test_dir(&self, name: &str, path: &Path) -> bool {
        let name_lower = name.to_lowercase();

        // Direct match
        for indicator in &self.test_indicators {
            if name_lower.contains(indicator) {
                return true;
            }
        }

        // Check if it contains test files
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                if file_name.contains("test") || file_name.contains("spec") {
                    return true;
                }
            }
        }

        false
    }
}