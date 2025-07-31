//! Cross-repository type analysis

use crate::core::constants::languages;
use crate::core::utils::FileUtils;
use crate::multi_repo::cross_repo::types::{TypeDefinition, TypeReference, TypeUsage};
use crate::multi_repo::registry::RepositoryRegistry;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Lazy static initialization of type patterns
static TYPE_PATTERNS: Lazy<HashMap<String, Vec<Regex>>> = Lazy::new(|| {
    let mut patterns = HashMap::new();

    // TypeScript type definitions
    if let (Ok(r1), Ok(r2)) = (
        Regex::new(r"export\s+(?:declare\s+)?(?:interface|type|class|enum)\s+(\w+)"),
        Regex::new(r"(?:interface|type|class|enum)\s+(\w+)"),
    ) {
        patterns.insert(languages::TYPESCRIPT.to_string(), vec![r1, r2]);
    }

    // Rust type definitions
    if let (Ok(r1), Ok(r2)) = (
        Regex::new(r"pub\s+(?:struct|enum|trait|type)\s+(\w+)"),
        Regex::new(r"(?:struct|enum|trait|type)\s+(\w+)"),
    ) {
        patterns.insert(languages::RUST.to_string(), vec![r1, r2]);
    }

    // Python type definitions
    if let (Ok(r1), Ok(r2), Ok(r3)) = (
        Regex::new(r"class\s+(\w+)"),
        Regex::new(r"(\w+)\s*=\s*TypedDict"),
        Regex::new(r"(\w+)\s*=\s*NamedTuple"),
    ) {
        patterns.insert(languages::PYTHON.to_string(), vec![r1, r2, r3]);
    }

    patterns
});

/// Analyzes type definitions and usage across repositories
pub struct TypeAnalyzer {
    /// Type definition patterns
    type_patterns: HashMap<String, Vec<Regex>>,
}

impl TypeAnalyzer {
    /// Create a new type analyzer
    pub fn new() -> Self {
        Self {
            type_patterns: TYPE_PATTERNS.clone(),
        }
    }

    /// Analyze type references across repositories
    pub async fn analyze_type_references(
        &self,
        registry: &RepositoryRegistry,
    ) -> Result<Vec<TypeReference>> {
        let repos = registry.list_active().await?;
        let mut all_types = HashMap::new();
        let mut all_usages = HashMap::new();

        // First pass: collect all type definitions
        for repo in &repos {
            let types = self
                .find_type_definitions(&repo.path, &repo.id, &repo.primary_language)
                .await?;

            for (type_name, definition) in types {
                all_types
                    .entry(type_name.clone())
                    .or_insert_with(Vec::new)
                    .push(definition);
            }
        }

        // Second pass: find type usages
        for repo in &repos {
            let usages = self
                .find_type_usages(&repo.path, &repo.id, &repo.primary_language, &all_types)
                .await?;

            for (type_name, usage) in usages {
                all_usages
                    .entry(type_name)
                    .or_insert_with(Vec::new)
                    .push(usage);
            }
        }

        // Build type references
        let mut references = Vec::new();

        for (type_name, definitions) in all_types {
            for definition in definitions {
                let target_repos: Vec<TypeUsage> = all_usages
                    .get(&type_name)
                    .map(|usages| {
                        usages
                            .iter()
                            .filter(|u| u.repo_id != definition.repo_id)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                if !target_repos.is_empty() {
                    references.push(TypeReference {
                        type_name: type_name.clone(),
                        source_repo_id: definition.repo_id,
                        source_file: definition.file_path,
                        source_line: definition.line_number,
                        target_repos,
                    });
                }
            }
        }

        Ok(references)
    }

    /// Find type definitions in a repository
    async fn find_type_definitions(
        &self,
        repo_path: &Path,
        repo_id: &str,
        language: &Option<String>,
    ) -> Result<HashMap<String, TypeDefinition>> {
        let mut definitions = HashMap::new();

        let patterns = match language.as_deref() {
            Some("typescript") | Some("javascript") => self.type_patterns.get("typescript"),
            Some("rust") => self.type_patterns.get("rust"),
            Some("python") => self.type_patterns.get("python"),
            _ => None,
        };

        if let Some(patterns) = patterns {
            let extensions = get_file_extensions(language.as_deref());

            for entry in WalkDir::new(repo_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str());

                if let Some(ext) = ext {
                    if extensions.contains(&ext) {
                        if let Ok(content) =
                            FileUtils::read_with_context(path, "source file for type analysis")
                                .await
                        {
                            for (line_num, line) in content.lines().enumerate() {
                                for pattern in patterns {
                                    if let Some(captures) = pattern.captures(line) {
                                        if let Some(type_name) = captures.get(1) {
                                            definitions.insert(
                                                type_name.as_str().to_string(),
                                                TypeDefinition {
                                                    repo_id: repo_id.to_string(),
                                                    file_path: path.to_path_buf(),
                                                    line_number: line_num + 1,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(definitions)
    }

    /// Find type usages in a repository
    async fn find_type_usages(
        &self,
        repo_path: &Path,
        repo_id: &str,
        language: &Option<String>,
        known_types: &HashMap<String, Vec<TypeDefinition>>,
    ) -> Result<Vec<(String, TypeUsage)>> {
        let mut usages = Vec::new();
        let extensions = get_file_extensions(language.as_deref());

        for entry in WalkDir::new(repo_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());

            if let Some(ext) = ext {
                if extensions.contains(&ext) {
                    if let Ok(content) =
                        FileUtils::read_with_context(path, "source file for dependency analysis")
                            .await
                    {
                        for (line_num, line) in content.lines().enumerate() {
                            for type_name in known_types.keys() {
                                if line.contains(type_name) {
                                    usages.push((
                                        type_name.clone(),
                                        TypeUsage {
                                            repo_id: repo_id.to_string(),
                                            file_path: path.to_path_buf(),
                                            line_number: line_num + 1,
                                            usage_context: line.trim().to_string(),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(usages)
    }
}

/// Get file extensions for a language
fn get_file_extensions(language: Option<&str>) -> Vec<&'static str> {
    match language {
        Some("typescript") => vec!["ts", "tsx", "d.ts"],
        Some("javascript") => vec!["js", "jsx"],
        Some("rust") => vec!["rs"],
        Some("python") => vec!["py"],
        _ => vec![],
    }
}

impl Default for TypeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}