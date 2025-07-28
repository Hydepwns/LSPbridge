use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub root: PathBuf,
    pub source_dirs: Vec<PathBuf>,
    pub test_dirs: Vec<PathBuf>,
    pub config_files: Vec<PathBuf>,
    pub documentation_files: Vec<PathBuf>,
    pub total_files: usize,
    pub file_types: HashMap<String, usize>,
    pub directory_tree: DirectoryNode,
    pub is_monorepo: bool,
    pub subprojects: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub children: Vec<DirectoryNode>,
    pub file_count: usize,
    pub total_size: u64,
}

pub struct StructureAnalyzer {
    ignore_patterns: HashSet<String>,
    source_indicators: HashSet<String>,
    test_indicators: HashSet<String>,
    config_extensions: HashSet<String>,
    doc_extensions: HashSet<String>,
}

impl Default for StructureAnalyzer {
    fn default() -> Self {
        let mut ignore_patterns = HashSet::new();
        ignore_patterns.insert("node_modules".to_string());
        ignore_patterns.insert("target".to_string());
        ignore_patterns.insert("dist".to_string());
        ignore_patterns.insert("build".to_string());
        ignore_patterns.insert(".git".to_string());
        ignore_patterns.insert(".svn".to_string());
        ignore_patterns.insert(".hg".to_string());
        ignore_patterns.insert("__pycache__".to_string());
        ignore_patterns.insert(".pytest_cache".to_string());
        ignore_patterns.insert(".tox".to_string());
        ignore_patterns.insert("venv".to_string());
        ignore_patterns.insert(".venv".to_string());
        ignore_patterns.insert("env".to_string());
        ignore_patterns.insert(".env".to_string());

        let mut source_indicators = HashSet::new();
        source_indicators.insert("src".to_string());
        source_indicators.insert("lib".to_string());
        source_indicators.insert("app".to_string());
        source_indicators.insert("source".to_string());
        source_indicators.insert("sources".to_string());
        source_indicators.insert("pkg".to_string());
        source_indicators.insert("cmd".to_string());
        source_indicators.insert("internal".to_string());

        let mut test_indicators = HashSet::new();
        test_indicators.insert("test".to_string());
        test_indicators.insert("tests".to_string());
        test_indicators.insert("spec".to_string());
        test_indicators.insert("specs".to_string());
        test_indicators.insert("__tests__".to_string());
        test_indicators.insert("test_".to_string());
        test_indicators.insert("_test".to_string());
        test_indicators.insert("testing".to_string());

        let mut config_extensions = HashSet::new();
        config_extensions.insert("toml".to_string());
        config_extensions.insert("yaml".to_string());
        config_extensions.insert("yml".to_string());
        config_extensions.insert("json".to_string());
        config_extensions.insert("ini".to_string());
        config_extensions.insert("conf".to_string());
        config_extensions.insert("config".to_string());
        config_extensions.insert("cfg".to_string());
        config_extensions.insert("xml".to_string());

        let mut doc_extensions = HashSet::new();
        doc_extensions.insert("md".to_string());
        doc_extensions.insert("rst".to_string());
        doc_extensions.insert("txt".to_string());
        doc_extensions.insert("adoc".to_string());
        doc_extensions.insert("textile".to_string());

        Self {
            ignore_patterns,
            source_indicators,
            test_indicators,
            config_extensions,
            doc_extensions,
        }
    }
}

impl StructureAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

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

        // Detect if this is a monorepo
        structure.is_monorepo = self.detect_monorepo(project_root, &structure.subprojects);

        Ok(structure)
    }

    fn build_tree(
        &self,
        path: &Path,
    ) -> Result<(
        DirectoryNode,
        Vec<PathBuf>,           // source_dirs
        Vec<PathBuf>,           // test_dirs
        Vec<PathBuf>,           // config_files
        Vec<PathBuf>,           // doc_files
        usize,                  // total_files
        HashMap<String, usize>, // file_types
        Vec<PathBuf>,           // subprojects
    )> {
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

    fn should_ignore(&self, name: &str) -> bool {
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

    fn detect_monorepo(&self, root: &Path, subprojects: &[PathBuf]) -> bool {
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

        result.push_str(&format!("{}{}{}", prefix, connector, name));

        if node.is_directory && node.file_count > 0 {
            result.push_str(&format!(" ({} files)", node.file_count));
        }

        result.push('\n');

        if node.is_directory && depth < max_depth {
            let child_prefix = format!("{}│   ", prefix);
            for (i, child) in node.children.iter().enumerate() {
                let is_last = i == node.children.len() - 1;
                let child_connector = if is_last { "└── " } else { "├── " };
                let next_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    child_prefix.clone()
                };

                self.format_tree_node(child, result, depth + 1, max_depth, &next_prefix);
            }
        }
    }
}

impl ProjectStructure {
    pub fn get_main_language(&self) -> Option<String> {
        let mut language_scores: HashMap<&str, usize> = HashMap::new();

        for (ext, count) in &self.file_types {
            let language = match ext.as_str() {
                "rs" => "Rust",
                "js" | "jsx" => "JavaScript",
                "ts" | "tsx" => "TypeScript",
                "py" => "Python",
                "java" => "Java",
                "go" => "Go",
                "rb" => "Ruby",
                "php" => "PHP",
                "cpp" | "cc" | "cxx" => "C++",
                "c" => "C",
                "cs" => "C#",
                "swift" => "Swift",
                "kt" | "kts" => "Kotlin",
                "scala" => "Scala",
                "ex" | "exs" => "Elixir",
                "erl" | "hrl" => "Erlang",
                _ => continue,
            };

            *language_scores.entry(language).or_insert(0) += count;
        }

        language_scores
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.to_string())
    }

    pub fn summary(&self) -> String {
        let mut summary = format!(
            "Project Structure Summary:\n\
             Root: {}\n\
             Total files: {}\n",
            self.root.display(),
            self.total_files
        );

        if self.is_monorepo {
            summary.push_str(&format!(
                "Type: Monorepo with {} subprojects\n",
                self.subprojects.len()
            ));
        }

        if let Some(lang) = self.get_main_language() {
            summary.push_str(&format!("Main language: {}\n", lang));
        }

        if !self.source_dirs.is_empty() {
            summary.push_str(&format!("Source directories: {}\n", self.source_dirs.len()));
        }

        if !self.test_dirs.is_empty() {
            summary.push_str(&format!("Test directories: {}\n", self.test_dirs.len()));
        }

        summary.push_str("\nFile type distribution:\n");
        let mut sorted_types: Vec<_> = self.file_types.iter().collect();
        sorted_types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

        for (ext, count) in sorted_types.iter().take(10) {
            summary.push_str(&format!("  .{}: {}\n", ext, count));
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_basic_structure_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap(); // Should be ignored

        // Create files
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src/lib.rs"), "pub mod utils;").unwrap();
        fs::write(root.join("tests/test_main.rs"), "#[test] fn test() {}").unwrap();
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();
        fs::write(root.join("README.md"), "# Project").unwrap();
        fs::write(root.join("docs/guide.md"), "# Guide").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();

        assert_eq!(structure.total_files, 6); // node_modules should be ignored
        assert_eq!(structure.source_dirs.len(), 1);
        assert_eq!(structure.test_dirs.len(), 1);
        assert!(structure
            .config_files
            .iter()
            .any(|p| p.ends_with("Cargo.toml")));
        assert_eq!(structure.documentation_files.len(), 2);
        assert_eq!(structure.file_types.get("rs"), Some(&3));
        assert_eq!(structure.file_types.get("md"), Some(&2));
    }

    #[test]
    fn test_monorepo_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a yarn workspaces monorepo
        let package_json = r#"{
            "name": "monorepo",
            "workspaces": ["packages/*"]
        }"#;
        fs::write(root.join("package.json"), package_json).unwrap();

        fs::create_dir_all(root.join("packages/app")).unwrap();
        fs::create_dir_all(root.join("packages/lib")).unwrap();
        fs::write(root.join("packages/app/package.json"), "{}").unwrap();
        fs::write(root.join("packages/lib/package.json"), "{}").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();

        assert!(structure.is_monorepo);
        assert_eq!(structure.subprojects.len(), 2);
    }

    #[test]
    fn test_language_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create TypeScript project
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/index.ts"), "export {}").unwrap();
        fs::write(root.join("src/utils.ts"), "export {}").unwrap();
        fs::write(root.join("src/types.ts"), "export {}").unwrap();
        fs::write(root.join("test.js"), "test()").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();

        assert_eq!(
            structure.get_main_language(),
            Some("TypeScript".to_string())
        );
    }

    #[test]
    fn test_file_tree_summary() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src/utils")).unwrap();
        fs::write(root.join("src/main.rs"), "").unwrap();
        fs::write(root.join("src/utils/helpers.rs"), "").unwrap();

        let analyzer = StructureAnalyzer::new();
        let structure = analyzer.analyze(root).unwrap();
        let tree = analyzer.get_file_tree_summary(&structure, 3);

        assert!(tree.contains("src/"));
        assert!(tree.contains("utils/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("helpers.rs"));
    }
}
