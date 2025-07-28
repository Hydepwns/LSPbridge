use anyhow::Result;
use glob::{Pattern, PatternError};
use std::fs;
use std::path::{Path, PathBuf};

/// Workspace-aware filter that respects .gitignore and other ignore files
pub struct WorkspaceFilter {
    workspace_root: PathBuf,
    ignore_patterns: Vec<Pattern>,
    gitignore_patterns: Vec<Pattern>,
    respect_gitignore: bool,
}

impl WorkspaceFilter {
    /// Create a new workspace filter
    pub fn new(workspace_root: PathBuf) -> Self {
        let mut filter = Self {
            workspace_root,
            ignore_patterns: Vec::new(),
            gitignore_patterns: Vec::new(),
            respect_gitignore: true,
        };

        // Load .gitignore if it exists
        filter.load_gitignore();

        // Add common patterns that should always be ignored
        filter.add_default_patterns();

        filter
    }

    /// Load patterns from .gitignore file
    fn load_gitignore(&mut self) {
        let gitignore_path = self.workspace_root.join(".gitignore");
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            for line in content.lines() {
                let line = line.trim();

                // Skip comments and empty lines
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Convert gitignore pattern to glob pattern
                if let Ok(pattern) = self.gitignore_to_glob(line) {
                    if let Ok(glob_pattern) = Pattern::new(&pattern) {
                        self.gitignore_patterns.push(glob_pattern);
                    }
                }
            }
        }

        // Also check for other common ignore files
        self.load_ignore_file(".ignore");
        self.load_ignore_file(".fdignore");
        self.load_ignore_file(".rgignore");
    }

    /// Load patterns from a generic ignore file
    fn load_ignore_file(&mut self, filename: &str) {
        let ignore_path = self.workspace_root.join(filename);
        if let Ok(content) = fs::read_to_string(&ignore_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Ok(pattern) = self.gitignore_to_glob(line) {
                    if let Ok(glob_pattern) = Pattern::new(&pattern) {
                        self.ignore_patterns.push(glob_pattern);
                    }
                }
            }
        }
    }

    /// Convert gitignore pattern to glob pattern
    fn gitignore_to_glob(&self, gitignore_pattern: &str) -> Result<String, PatternError> {
        let mut pattern = gitignore_pattern.to_string();

        // Handle directory patterns
        if pattern.ends_with('/') {
            pattern.push_str("**");
        }

        // Handle patterns that should match anywhere
        if !pattern.starts_with('/') && !pattern.starts_with("**/") {
            pattern = format!("**/{}", pattern);
        } else if pattern.starts_with('/') {
            // Remove leading slash for absolute patterns
            pattern.remove(0);
        }

        // Convert gitignore wildcards to glob wildcards
        pattern = pattern.replace("**/**", "**");

        Ok(pattern)
    }

    /// Add default patterns that should always be ignored
    fn add_default_patterns(&mut self) {
        let default_patterns = vec![
            // Version control
            "**/.git/**",
            "**/.svn/**",
            "**/.hg/**",
            // Dependencies
            "**/node_modules/**",
            "**/vendor/**",
            "**/target/**",
            "**/.cargo/**",
            // Build artifacts
            "**/dist/**",
            "**/build/**",
            "**/out/**",
            "**/_build/**",
            // IDE and editor files
            "**/.idea/**",
            "**/.vscode/**",
            "**/Thumbs.db",
            "**/.DS_Store",
            // Temporary files
            "**/tmp/**",
            "**/temp/**",
            "**/*.tmp",
            "**/*.temp",
            "**/*.swp",
            "**/*.swo",
            "**/~*",
            // Cache directories
            "**/.cache/**",
            "**/__pycache__/**",
            "**/.pytest_cache/**",
            // Coverage reports
            "**/coverage/**",
            "**/.coverage",
            "**/htmlcov/**",
            // Environment files (security sensitive)
            "**/.env*",
            "**/secrets/**",
            "**/credentials/**",
        ];

        for pattern_str in default_patterns {
            if let Ok(pattern) = Pattern::new(pattern_str) {
                self.ignore_patterns.push(pattern);
            }
        }
    }

    /// Check if a file should be included based on all filters
    pub fn should_include_file(&self, file_path: &Path) -> bool {
        // Convert to relative path from workspace root
        let relative_path = match file_path.strip_prefix(&self.workspace_root) {
            Ok(p) => p,
            Err(_) => {
                // File is outside workspace, check absolute path
                file_path
            }
        };

        let path_str = relative_path.to_string_lossy();

        // Check gitignore patterns
        if self.respect_gitignore {
            for pattern in &self.gitignore_patterns {
                if pattern.matches(&path_str) {
                    return false;
                }
            }
        }

        // Check custom ignore patterns
        for pattern in &self.ignore_patterns {
            if pattern.matches(&path_str) {
                return false;
            }
        }

        // Check if file exists and is readable
        if !file_path.exists() || !file_path.is_file() {
            return false;
        }

        true
    }

    /// Add custom ignore pattern
    pub fn add_ignore_pattern(&mut self, pattern: &str) -> Result<()> {
        let glob_pattern = Pattern::new(pattern)?;
        self.ignore_patterns.push(glob_pattern);
        Ok(())
    }

    /// Set whether to respect .gitignore
    pub fn set_respect_gitignore(&mut self, respect: bool) {
        self.respect_gitignore = respect;
    }

    /// Get all active patterns for debugging
    pub fn get_active_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();

        if self.respect_gitignore {
            for pattern in &self.gitignore_patterns {
                patterns.push(format!("gitignore: {}", pattern.as_str()));
            }
        }

        for pattern in &self.ignore_patterns {
            patterns.push(format!("custom: {}", pattern.as_str()));
        }

        patterns
    }

    /// Check if a path is in a hidden directory
    pub fn is_hidden_path(path: &Path) -> bool {
        path.components()
            .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_filter_basic() {
        let temp_dir = TempDir::new().unwrap();
        let filter = WorkspaceFilter::new(temp_dir.path().to_path_buf());

        // Create test files
        let normal_file = temp_dir.path().join("src/main.rs");
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        File::create(&normal_file).unwrap();

        let node_modules_file = temp_dir.path().join("node_modules/package/index.js");
        fs::create_dir_all(temp_dir.path().join("node_modules/package")).unwrap();
        File::create(&node_modules_file).unwrap();

        assert!(filter.should_include_file(&normal_file));
        assert!(!filter.should_include_file(&node_modules_file));
    }

    #[test]
    fn test_gitignore_loading() {
        let temp_dir = TempDir::new().unwrap();

        // Create .gitignore
        let gitignore_content = r#"
# Comments should be ignored
target/
*.tmp
/build
node_modules/
.env*
"#;

        let mut gitignore_file = File::create(temp_dir.path().join(".gitignore")).unwrap();
        gitignore_file
            .write_all(gitignore_content.as_bytes())
            .unwrap();

        let filter = WorkspaceFilter::new(temp_dir.path().to_path_buf());

        // Create test files
        let target_file = temp_dir.path().join("target/debug/app");
        fs::create_dir_all(temp_dir.path().join("target/debug")).unwrap();
        File::create(&target_file).unwrap();

        let tmp_file = temp_dir.path().join("test.tmp");
        File::create(&tmp_file).unwrap();

        let env_file = temp_dir.path().join(".env.local");
        File::create(&env_file).unwrap();

        let src_file = temp_dir.path().join("src/main.rs");
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        File::create(&src_file).unwrap();

        assert!(!filter.should_include_file(&target_file));
        assert!(!filter.should_include_file(&tmp_file));
        assert!(!filter.should_include_file(&env_file));
        assert!(filter.should_include_file(&src_file));
    }

    #[test]
    fn test_custom_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut filter = WorkspaceFilter::new(temp_dir.path().to_path_buf());

        // Add custom pattern
        filter.add_ignore_pattern("**/test_*.rs").unwrap();

        let test_file = temp_dir.path().join("src/test_module.rs");
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        File::create(&test_file).unwrap();

        let normal_file = temp_dir.path().join("src/module.rs");
        File::create(&normal_file).unwrap();

        assert!(!filter.should_include_file(&test_file));
        assert!(filter.should_include_file(&normal_file));
    }

    #[test]
    fn test_hidden_paths() {
        let hidden_path = Path::new("/home/user/.config/app/config.toml");
        let normal_path = Path::new("/home/user/documents/file.txt");

        assert!(WorkspaceFilter::is_hidden_path(hidden_path));
        assert!(!WorkspaceFilter::is_hidden_path(normal_path));
    }
}
