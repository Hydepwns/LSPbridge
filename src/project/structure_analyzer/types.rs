//! Project structure types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Complete project structure representation
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

/// Directory tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub children: Vec<DirectoryNode>,
    pub file_count: usize,
    pub total_size: u64,
}

impl ProjectStructure {
    /// Get the main programming language of the project
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

    /// Get a summary of the project structure
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
            summary.push_str(&format!("Main language: {lang}\n"));
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
            summary.push_str(&format!("  .{ext}: {count}\n"));
        }

        summary
    }
}