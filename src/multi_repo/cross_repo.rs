//! Cross-repository analysis for shared types and dependencies

use crate::core::constants::languages;
use crate::core::utils::FileUtils;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::registry::RepositoryRegistry;

/// Cross-repository type reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeReference {
    /// Type name
    pub type_name: String,

    /// Source repository where type is defined
    pub source_repo_id: String,

    /// Source file path
    pub source_file: PathBuf,

    /// Source line number
    pub source_line: usize,

    /// Target repositories using this type
    pub target_repos: Vec<TypeUsage>,
}

/// Type usage in a target repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeUsage {
    pub repo_id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub usage_context: String,
}

/// Import relationship between files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRelation {
    /// Source repository
    pub source_repo_id: String,

    /// Source file
    pub source_file: PathBuf,

    /// Imported module/package
    pub import_path: String,

    /// Target repository (if external)
    pub target_repo_id: Option<String>,

    /// Resolved target file
    pub target_file: Option<PathBuf>,

    /// Import type
    pub import_type: ImportType,
}

/// Types of imports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportType {
    /// Local file import
    Local,

    /// Package import
    Package,

    /// Relative import
    Relative,

    /// Workspace import (monorepo)
    Workspace,

    /// External repository import
    External,
}

/// Analyzes cross-repository dependencies and type usage
pub struct CrossRepoAnalyzer {
    /// Whether to analyze type references
    analyze_types: bool,

    /// Import patterns for different languages
    import_patterns: HashMap<String, Vec<Regex>>,

    /// Type definition patterns
    type_patterns: HashMap<String, Vec<Regex>>,
}

impl CrossRepoAnalyzer {
    /// Create a new cross-repository analyzer
    pub fn new(analyze_types: bool) -> Self {
        let mut import_patterns = HashMap::new();
        let mut type_patterns = HashMap::new();

        // TypeScript/JavaScript import patterns
        import_patterns.insert(languages::TYPESCRIPT.to_string(), vec![
            Regex::new(r#"import\s+(?:type\s+)?(?:\{[^}]+\}|\*\s+as\s+\w+|\w+)\s+from\s+['"]([@\w\-/\.]+)['"]"#).unwrap(),
            Regex::new(r#"require\s*\(\s*['"]([@\w\-/\.]+)['"]\s*\)"#).unwrap(),
            Regex::new(r#"import\s*\(\s*['"]([@\w\-/\.]+)['"]\s*\)"#).unwrap(),
        ]);

        // Rust import patterns
        import_patterns.insert(
            languages::RUST.to_string(),
            vec![
                Regex::new(r"use\s+((?:\w+::)*\w+)").unwrap(),
                Regex::new(r#"extern\s+crate\s+(\w+)"#).unwrap(),
            ],
        );

        // Python import patterns
        import_patterns.insert(
            languages::PYTHON.to_string(),
            vec![
                Regex::new(r"from\s+([\w\.]+)\s+import").unwrap(),
                Regex::new(r"import\s+([\w\.]+)").unwrap(),
            ],
        );

        // TypeScript type definitions
        type_patterns.insert(
            languages::TYPESCRIPT.to_string(),
            vec![
                Regex::new(r"export\s+(?:declare\s+)?(?:interface|type|class|enum)\s+(\w+)")
                    .unwrap(),
                Regex::new(r"(?:interface|type|class|enum)\s+(\w+)").unwrap(),
            ],
        );

        // Rust type definitions
        type_patterns.insert(
            languages::RUST.to_string(),
            vec![
                Regex::new(r"pub\s+(?:struct|enum|trait|type)\s+(\w+)").unwrap(),
                Regex::new(r"(?:struct|enum|trait|type)\s+(\w+)").unwrap(),
            ],
        );

        // Python type definitions
        type_patterns.insert(
            languages::PYTHON.to_string(),
            vec![
                Regex::new(r"class\s+(\w+)").unwrap(),
                Regex::new(r"(\w+)\s*=\s*TypedDict").unwrap(),
                Regex::new(r"(\w+)\s*=\s*NamedTuple").unwrap(),
            ],
        );

        Self {
            analyze_types,
            import_patterns,
            type_patterns,
        }
    }

    /// Analyze type references across repositories
    pub async fn analyze_type_references(
        &self,
        registry: &RepositoryRegistry,
    ) -> Result<Vec<TypeReference>> {
        if !self.analyze_types {
            return Ok(Vec::new());
        }

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
                            .filter(|u| u.repo_id != definition.0)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                if !target_repos.is_empty() {
                    references.push(TypeReference {
                        type_name: type_name.clone(),
                        source_repo_id: definition.0,
                        source_file: definition.1,
                        source_line: definition.2,
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
    ) -> Result<HashMap<String, (String, PathBuf, usize)>> {
        let mut definitions = HashMap::new();

        let patterns = match language.as_deref() {
            Some("typescript") | Some("javascript") => self.type_patterns.get("typescript"),
            Some("rust") => self.type_patterns.get("rust"),
            Some("python") => self.type_patterns.get("python"),
            _ => None,
        };

        if let Some(patterns) = patterns {
            let extensions = match language.as_deref() {
                Some("typescript") => vec!["ts", "tsx", "d.ts"],
                Some("javascript") => vec!["js", "jsx"],
                Some("rust") => vec!["rs"],
                Some("python") => vec!["py"],
                _ => vec![],
            };

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
                                                (
                                                    repo_id.to_string(),
                                                    path.to_path_buf(),
                                                    line_num + 1,
                                                ),
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
        known_types: &HashMap<String, Vec<(String, PathBuf, usize)>>,
    ) -> Result<Vec<(String, TypeUsage)>> {
        let mut usages = Vec::new();

        let extensions = match language.as_deref() {
            Some("typescript") => vec!["ts", "tsx"],
            Some("javascript") => vec!["js", "jsx"],
            Some("rust") => vec!["rs"],
            Some("python") => vec!["py"],
            _ => return Ok(usages),
        };

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

    /// Resolve import paths across repositories
    pub async fn resolve_imports(
        &self,
        registry: &RepositoryRegistry,
    ) -> Result<Vec<ImportRelation>> {
        let repos = registry.list_active().await?;
        let mut import_relations = Vec::new();

        for repo in repos {
            let imports = self
                .find_imports(&repo.path, &repo.id, &repo.primary_language)
                .await?;

            for mut import in imports {
                // Try to resolve the import
                if let Some(resolved) = self
                    .resolve_import_target(&import.import_path, &repo, registry)
                    .await?
                {
                    import.target_repo_id = Some(resolved.0);
                    import.target_file = Some(resolved.1);
                    import.import_type = resolved.2;
                }

                import_relations.push(import);
            }
        }

        Ok(import_relations)
    }

    /// Find imports in a repository
    async fn find_imports(
        &self,
        repo_path: &Path,
        repo_id: &str,
        language: &Option<String>,
    ) -> Result<Vec<ImportRelation>> {
        let mut imports = Vec::new();

        let patterns = match language.as_deref() {
            Some("typescript") | Some("javascript") => self.import_patterns.get("typescript"),
            Some("rust") => self.import_patterns.get("rust"),
            Some("python") => self.import_patterns.get("python"),
            _ => None,
        };

        if let Some(patterns) = patterns {
            let extensions = match language.as_deref() {
                Some("typescript") => vec!["ts", "tsx"],
                Some("javascript") => vec!["js", "jsx"],
                Some("rust") => vec!["rs"],
                Some("python") => vec!["py"],
                _ => vec![],
            };

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
                            for line in content.lines() {
                                for pattern in patterns {
                                    if let Some(captures) = pattern.captures(line) {
                                        if let Some(import_path) = captures.get(1) {
                                            imports.push(ImportRelation {
                                                source_repo_id: repo_id.to_string(),
                                                source_file: path.to_path_buf(),
                                                import_path: import_path.as_str().to_string(),
                                                target_repo_id: None,
                                                target_file: None,
                                                import_type: ImportType::Local, // Will be determined
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(imports)
    }

    /// Resolve import target
    async fn resolve_import_target(
        &self,
        import_path: &str,
        source_repo: &super::registry::RepositoryInfo,
        registry: &RepositoryRegistry,
    ) -> Result<Option<(String, PathBuf, ImportType)>> {
        // Check if it's a local import
        if import_path.starts_with('.') || import_path.starts_with('/') {
            return Ok(Some((
                source_repo.id.clone(),
                source_repo.path.join(import_path),
                ImportType::Local,
            )));
        }

        // Check if it's a workspace import (monorepo)
        if source_repo.is_monorepo_member {
            if let Some(monorepo_id) = &source_repo.monorepo_id {
                // Find sibling packages in the monorepo
                let siblings = registry
                    .list_active()
                    .await?
                    .into_iter()
                    .filter(|r| r.monorepo_id.as_ref() == Some(monorepo_id))
                    .collect::<Vec<_>>();

                for sibling in siblings {
                    if import_path.contains(&sibling.name) {
                        return Ok(Some((
                            sibling.id,
                            sibling.path.clone(),
                            ImportType::Workspace,
                        )));
                    }
                }
            }
        }

        // Check if it's an external package that we're tracking
        let all_repos = registry.list_active().await?;
        for repo in all_repos {
            if let Some(remote_url) = &repo.remote_url {
                if import_path.contains(&repo.name) || remote_url.contains(import_path) {
                    return Ok(Some((repo.id, repo.path.clone(), ImportType::External)));
                }
            }
        }

        // Package import (not tracked)
        Ok(None)
    }
}
