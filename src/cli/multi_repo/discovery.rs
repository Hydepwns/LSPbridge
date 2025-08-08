//! Repository discovery and management utilities
//!
//! This module provides utilities for discovering repositories, analyzing their
//! structure, and managing repository metadata and relationships.

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::multi_repo::RepositoryInfo;
use crate::project::BuildSystemDetector;

/// Repository discovery engine for finding and analyzing repositories
pub struct RepositoryDiscovery {
    /// Maximum depth to search for repositories
    max_depth: usize,
    
    /// Whether to follow symbolic links
    follow_links: bool,
    
    /// File patterns that indicate a repository root
    repository_indicators: Vec<String>,
}

impl RepositoryDiscovery {
    /// Create a new repository discovery engine
    pub fn new() -> Self {
        Self {
            max_depth: 5,
            follow_links: false,
            repository_indicators: vec![
                ".git".to_string(),
                "package.json".to_string(),
                "Cargo.toml".to_string(),
                "pyproject.toml".to_string(),
                "setup.py".to_string(),
                "go.mod".to_string(),
                "pom.xml".to_string(),
                "build.gradle".to_string(),
                "Makefile".to_string(),
                "CMakeLists.txt".to_string(),
            ],
        }
    }

    /// Configure maximum search depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Configure whether to follow symbolic links
    pub fn with_follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    /// Add custom repository indicators
    pub fn with_indicators(mut self, indicators: Vec<String>) -> Self {
        self.repository_indicators.extend(indicators);
        self
    }

    /// Discover repositories in the given path
    pub async fn discover_repositories(&self, root_path: &Path) -> Result<Vec<RepositoryCandidate>> {
        let mut candidates = Vec::new();
        let mut repository_roots = std::collections::HashSet::new();

        let walker = WalkDir::new(root_path)
            .max_depth(self.max_depth)
            .follow_links(self.follow_links);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            
            // Skip if this path is inside an already discovered repository
            let is_inside_repo = repository_roots.iter().any(|repo_root: &PathBuf| {
                path.starts_with(repo_root) && path != repo_root.as_path()
            });
            
            if is_inside_repo {
                continue;
            }

            if self.is_repository_root(path).await? {
                let candidate = self.analyze_repository_candidate(path).await?;
                repository_roots.insert(path.to_path_buf());
                candidates.push(candidate);
            }
        }

        Ok(candidates)
    }

    /// Check if a path is a repository root
    async fn is_repository_root(&self, path: &Path) -> Result<bool> {
        if !path.is_dir() {
            return Ok(false);
        }

        for indicator in &self.repository_indicators {
            let indicator_path = path.join(indicator);
            if indicator_path.exists() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Analyze a repository candidate and extract metadata
    async fn analyze_repository_candidate(&self, path: &Path) -> Result<RepositoryCandidate> {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Detect repository type
        let repo_type = self.detect_repository_type(path).await?;
        
        // Detect primary language
        let primary_language = self.detect_primary_language(path).await;
        
        // Detect build system
        let build_system = BuildSystemDetector::detect(path)
            .ok()
            .map(|config| format!("{:?}", config.system));

        // Check for monorepo indicators
        let is_monorepo = self.detect_monorepo_structure(path).await?;
        
        // Extract Git information if available
        let git_info = self.extract_git_info(path).await.ok();

        Ok(RepositoryCandidate {
            path: path.to_path_buf(),
            name,
            repo_type,
            primary_language,
            build_system,
            is_monorepo,
            git_info,
            subprojects: if is_monorepo {
                self.find_subprojects(path).await?
            } else {
                Vec::new()
            },
        })
    }

    /// Detect the type of repository
    async fn detect_repository_type(&self, path: &Path) -> Result<RepositoryType> {
        // Check for Git
        if path.join(".git").exists() {
            return Ok(RepositoryType::Git);
        }

        // Check for other VCS
        if path.join(".svn").exists() {
            return Ok(RepositoryType::Svn);
        }

        if path.join(".hg").exists() {
            return Ok(RepositoryType::Mercurial);
        }

        // Check for package managers that might indicate a project
        if path.join("package.json").exists() {
            return Ok(RepositoryType::Npm);
        }

        if path.join("Cargo.toml").exists() {
            return Ok(RepositoryType::Cargo);
        }

        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
            return Ok(RepositoryType::Python);
        }

        Ok(RepositoryType::Unknown)
    }

    /// Detect the primary programming language
    async fn detect_primary_language(&self, path: &Path) -> Option<String> {
        let mut language_counts: HashMap<String, usize> = HashMap::new();

        for entry in WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Some(extension) = entry.path().extension().and_then(|e| e.to_str()) {
                let language = match extension {
                    "rs" => "rust",
                    "ts" | "tsx" => "typescript", 
                    "js" | "jsx" => "javascript",
                    "py" => "python",
                    "go" => "go",
                    "java" => "java",
                    "cpp" | "cc" | "cxx" | "C" => "cpp",
                    "c" => "c",
                    "cs" => "csharp",
                    "rb" => "ruby",
                    "php" => "php",
                    "swift" => "swift",
                    "kt" => "kotlin",
                    "scala" => "scala",
                    "clj" | "cljs" => "clojure",
                    "hs" => "haskell",
                    "ml" => "ocaml",
                    "dart" => "dart",
                    _ => continue,
                };

                *language_counts.entry(language.to_string()).or_insert(0) += 1;
            }
        }

        language_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(language, _)| language)
    }

    /// Detect monorepo structure
    async fn detect_monorepo_structure(&self, path: &Path) -> Result<bool> {
        // Common monorepo indicators
        let monorepo_files = [
            "lerna.json",
            "nx.json",
            "rush.json",
            "workspace.json",
            "pnpm-workspace.yaml",
            "yarn.lock",
        ];

        for file in &monorepo_files {
            if path.join(file).exists() {
                return Ok(true);
            }
        }

        // Check for workspaces in package.json
        if let Ok(package_json) = std::fs::read_to_string(path.join("package.json")) {
            if package_json.contains("\"workspaces\"") {
                return Ok(true);
            }
        }

        // Check for Cargo workspace
        if let Ok(cargo_toml) = std::fs::read_to_string(path.join("Cargo.toml")) {
            if cargo_toml.contains("[workspace]") {
                return Ok(true);
            }
        }

        // Heuristic: multiple package.json or Cargo.toml files in subdirectories
        let mut project_files = 0;
        for entry in WalkDir::new(path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_name = entry.file_name().to_string_lossy();
            if file_name == "package.json" || file_name == "Cargo.toml" {
                project_files += 1;
                if project_files > 2 {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Find subprojects in a monorepo
    async fn find_subprojects(&self, path: &Path) -> Result<Vec<SubprojectInfo>> {
        let mut subprojects = Vec::new();

        for entry in WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
        {
            let dir_path = entry.path();
            
            // Skip the root directory
            if dir_path == path {
                continue;
            }

            // Check if this directory contains project files
            if self.is_repository_root(dir_path).await? {
                let name = dir_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let relative_path = dir_path.strip_prefix(path)
                    .unwrap_or(dir_path)
                    .to_path_buf();

                let language = self.detect_primary_language(dir_path).await;

                subprojects.push(SubprojectInfo {
                    name,
                    path: relative_path,
                    absolute_path: dir_path.to_path_buf(),
                    language,
                });
            }
        }

        Ok(subprojects)
    }

    /// Extract Git repository information
    async fn extract_git_info(&self, path: &Path) -> Result<GitInfo> {
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow::anyhow!("Not a Git repository"));
        }

        // Try to get remote URL
        let remote_url = self.get_git_remote_url(path).await.ok();
        
        // Try to get current branch
        let current_branch = self.get_git_current_branch(path).await.ok();

        Ok(GitInfo {
            remote_url,
            current_branch,
            has_uncommitted_changes: self.check_git_dirty(path).await.unwrap_or(false),
        })
    }

    /// Get Git remote URL
    async fn get_git_remote_url(&self, path: &Path) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["config", "--get", "remote.origin.url"])
            .current_dir(path)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get remote URL"))
        }
    }

    /// Get current Git branch
    async fn get_git_current_branch(&self, path: &Path) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get current branch"))
        }
    }

    /// Check if Git repository has uncommitted changes
    async fn check_git_dirty(&self, path: &Path) -> Result<bool> {
        let output = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(path)
            .output()
            .await?;

        if output.status.success() {
            Ok(!output.stdout.is_empty())
        } else {
            Err(anyhow::anyhow!("Failed to check Git status"))
        }
    }
}

impl Default for RepositoryDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// A repository candidate found during discovery
#[derive(Debug, Clone)]
pub struct RepositoryCandidate {
    pub path: PathBuf,
    pub name: String,
    pub repo_type: RepositoryType,
    pub primary_language: Option<String>,
    pub build_system: Option<String>,
    pub is_monorepo: bool,
    pub git_info: Option<GitInfo>,
    pub subprojects: Vec<SubprojectInfo>,
}

/// Repository type classification
#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryType {
    Git,
    Svn,
    Mercurial,
    Npm,
    Cargo,
    Python,
    Unknown,
}

/// Git repository information
#[derive(Debug, Clone)]
pub struct GitInfo {
    pub remote_url: Option<String>,
    pub current_branch: Option<String>,
    pub has_uncommitted_changes: bool,
}

/// Information about a subproject in a monorepo
#[derive(Debug, Clone)]
pub struct SubprojectInfo {
    pub name: String,
    pub path: PathBuf,
    pub absolute_path: PathBuf,
    pub language: Option<String>,
}

impl RepositoryCandidate {
    /// Convert to RepositoryInfo for registration
    pub fn to_repository_info(&self, id: String) -> RepositoryInfo {
        RepositoryInfo {
            id,
            name: self.name.clone(),
            path: self.path.clone(),
            remote_url: self.git_info.as_ref()
                .and_then(|git| git.remote_url.clone()),
            primary_language: self.primary_language.clone(),
            build_system: self.build_system.clone(),
            is_monorepo_member: false,
            monorepo_id: None,
            tags: Vec::new(),
            active: true,
            last_diagnostic_run: None,
            metadata: serde_json::json!({
                "repository_type": format!("{:?}", self.repo_type),
                "is_monorepo": self.is_monorepo,
                "subproject_count": self.subprojects.len(),
                "git_branch": self.git_info.as_ref()
                    .and_then(|git| git.current_branch.clone()),
                "has_uncommitted_changes": self.git_info.as_ref()
                    .map(|git| git.has_uncommitted_changes)
                    .unwrap_or(false)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_discover_git_repository() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        fs::create_dir_all(&repo_path).unwrap();

        // Create .git directory
        fs::create_dir(repo_path.join(".git")).unwrap();
        
        // Create some source files
        fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();

        let discovery = RepositoryDiscovery::new();
        let candidates = discovery.discover_repositories(temp_dir.path()).await.unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "test-repo");
        assert_eq!(candidates[0].repo_type, RepositoryType::Git);
        assert_eq!(candidates[0].primary_language, Some("rust".to_string()));
    }

    #[tokio::test]
    async fn test_detect_monorepo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("monorepo");
        fs::create_dir_all(&repo_path).unwrap();

        // Create package.json with workspaces
        let package_json = r#"
        {
            "name": "monorepo",
            "workspaces": ["packages/*"]
        }
        "#;
        fs::write(repo_path.join("package.json"), package_json).unwrap();

        // Create subproject
        let subproject_path = repo_path.join("packages/app");
        fs::create_dir_all(&subproject_path).unwrap();
        fs::write(subproject_path.join("package.json"), r#"{"name": "app"}"#).unwrap();

        let discovery = RepositoryDiscovery::new();
        let candidates = discovery.discover_repositories(temp_dir.path()).await.unwrap();

        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].is_monorepo);
        assert_eq!(candidates[0].subprojects.len(), 1);
        assert_eq!(candidates[0].subprojects[0].name, "app");
    }

    #[tokio::test]
    async fn test_detect_primary_language() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create more TypeScript files than Rust files
        fs::write(repo_path.join("index.ts"), "console.log('hello');").unwrap();
        fs::write(repo_path.join("types.ts"), "export interface User {}").unwrap();
        fs::write(repo_path.join("utils.ts"), "export const utils = {};").unwrap();
        fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();

        let discovery = RepositoryDiscovery::new();
        let language = discovery.detect_primary_language(repo_path).await;

        assert_eq!(language, Some("typescript".to_string()));
    }

    #[test]
    fn test_repository_candidate_conversion() {
        let candidate = RepositoryCandidate {
            path: PathBuf::from("/test/repo"),
            name: "test-repo".to_string(),
            repo_type: RepositoryType::Git,
            primary_language: Some("rust".to_string()),
            build_system: Some("Cargo".to_string()),
            is_monorepo: false,
            git_info: Some(GitInfo {
                remote_url: Some("https://github.com/user/repo.git".to_string()),
                current_branch: Some("main".to_string()),
                has_uncommitted_changes: false,
            }),
            subprojects: Vec::new(),
        };

        let repo_info = candidate.to_repository_info("test-id".to_string());

        assert_eq!(repo_info.id, "test-id");
        assert_eq!(repo_info.name, "test-repo");
        assert_eq!(repo_info.primary_language, Some("rust".to_string()));
        assert_eq!(repo_info.remote_url, Some("https://github.com/user/repo.git".to_string()));
        assert!(!repo_info.is_monorepo_member);
    }
}