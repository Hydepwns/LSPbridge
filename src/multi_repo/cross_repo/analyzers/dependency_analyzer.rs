//! Cross-repository dependency analysis

use crate::core::constants::languages;
use crate::core::utils::FileUtils;
use crate::multi_repo::cross_repo::types::{ImportRelation, ImportType};
use crate::multi_repo::registry::{RepositoryInfo, RepositoryRegistry};
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Lazy static initialization of import patterns
static IMPORT_PATTERNS: Lazy<HashMap<String, Vec<Regex>>> = Lazy::new(|| {
    let mut patterns = HashMap::new();

    // TypeScript/JavaScript import patterns
    if let (Ok(r1), Ok(r2), Ok(r3)) = (
        Regex::new(r#"import\s+(?:type\s+)?(?:\{[^}]+\}|\*\s+as\s+\w+|\w+)\s+from\s+['"]([@\w\-/\.]+)['"]"#),
        Regex::new(r#"require\s*\(\s*['"]([@\w\-/\.]+)['"]\s*\)"#),
        Regex::new(r#"import\s*\(\s*['"]([@\w\-/\.]+)['"]\s*\)"#),
    ) {
        patterns.insert(languages::TYPESCRIPT.to_string(), vec![r1, r2, r3]);
    }

    // Rust import patterns
    if let (Ok(r1), Ok(r2)) = (
        Regex::new(r"use\s+((?:\w+::)*\w+)"),
        Regex::new(r#"extern\s+crate\s+(\w+)"#),
    ) {
        patterns.insert(languages::RUST.to_string(), vec![r1, r2]);
    }

    // Python import patterns
    if let (Ok(r1), Ok(r2)) = (
        Regex::new(r"from\s+([\w\.]+)\s+import"),
        Regex::new(r"import\s+([\w\.]+)"),
    ) {
        patterns.insert(languages::PYTHON.to_string(), vec![r1, r2]);
    }

    patterns
});

/// Analyzes dependencies across repositories
pub struct DependencyAnalyzer {
    /// Import patterns for different languages
    import_patterns: HashMap<String, Vec<Regex>>,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer
    pub fn new() -> Self {
        Self {
            import_patterns: IMPORT_PATTERNS.clone(),
        }
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
        source_repo: &RepositoryInfo,
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

/// Get file extensions for a language
fn get_file_extensions(language: Option<&str>) -> Vec<&'static str> {
    match language {
        Some("typescript") => vec!["ts", "tsx"],
        Some("javascript") => vec!["js", "jsx"],
        Some("rust") => vec!["rs"],
        Some("python") => vec!["py"],
        _ => vec![],
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}