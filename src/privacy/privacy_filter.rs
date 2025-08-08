use super::workspace_filter::WorkspaceFilter;
use crate::core::{
    Diagnostic, DiagnosticSeverity, PrivacyFilter as PrivacyFilterTrait, PrivacyPolicy,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Pre-compiled regex patterns for security and performance
static DOUBLE_QUOTE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""[^"]*""#).expect("Failed to compile double quote regex")
});
static SINGLE_QUOTE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"'[^']*'").expect("Failed to compile single quote regex")
});
static TEMPLATE_LITERAL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`[^`]*`").expect("Failed to compile template literal regex")
});
static LINE_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"//.*$").expect("Failed to compile line comment regex")
});
static BLOCK_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/\*[\s\S]*?\*/").expect("Failed to compile block comment regex")
});
static HASH_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"#.*$").expect("Failed to compile hash comment regex")
});

pub struct PrivacyFilter {
    policy: PrivacyPolicy,
    workspace_filter: Option<WorkspaceFilter>,
}

impl PrivacyFilter {
    pub fn new(policy: PrivacyPolicy) -> Self {
        Self {
            policy,
            workspace_filter: None,
        }
    }

    pub fn with_default_policy() -> Self {
        Self::new(PrivacyPolicy::default())
    }

    pub fn with_strict_policy() -> Self {
        Self::new(PrivacyPolicy::strict())
    }

    pub fn with_permissive_policy() -> Self {
        Self::new(PrivacyPolicy::permissive())
    }

    pub fn with_workspace(mut self, workspace_root: PathBuf) -> Self {
        self.workspace_filter = Some(WorkspaceFilter::new(workspace_root));
        self
    }

    pub fn update_policy(&mut self, policy: PrivacyPolicy) {
        self.policy = policy;
    }

    pub fn get_policy(&self) -> &PrivacyPolicy {
        &self.policy
    }

    pub fn set_workspace_filter(&mut self, workspace_root: PathBuf) {
        self.workspace_filter = Some(WorkspaceFilter::new(workspace_root));
    }

    /// Sanitize string literals in diagnostic messages to prevent information leakage.
    /// 
    /// This function replaces string literals with placeholder text while preserving
    /// the overall structure of the message for debugging purposes.
    fn sanitize_string_literals(&self, message: &str) -> String {
        if message.is_empty() {
            return message.to_string();
        }

        // Limit input size to prevent DoS attacks
        let message = if message.len() > 8192 {
            &message[..8192]
        } else {
            message
        };

        let mut result = message.to_string();

        // Replace double quotes using pre-compiled regex
        result = DOUBLE_QUOTE_REGEX
            .replace_all(&result, r#""[STRING]""#)
            .to_string();

        // Replace single quotes using pre-compiled regex
        result = SINGLE_QUOTE_REGEX
            .replace_all(&result, "'[STRING]'")
            .to_string();

        // Replace template literals using pre-compiled regex
        result = TEMPLATE_LITERAL_REGEX
            .replace_all(&result, "`[STRING]`")
            .to_string();

        result
    }

    /// Sanitize comments in diagnostic messages to prevent information leakage.
    /// 
    /// This function replaces comments with placeholder text while preserving
    /// the comment structure for context.
    fn sanitize_comments(&self, message: &str) -> String {
        if message.is_empty() {
            return message.to_string();
        }

        // Limit input size to prevent DoS attacks
        let message = if message.len() > 8192 {
            &message[..8192]
        } else {
            message
        };

        let mut result = message.to_string();

        // Remove line comments using pre-compiled regex
        result = LINE_COMMENT_REGEX
            .replace_all(&result, "// [COMMENT]")
            .to_string();

        // Remove block comments using pre-compiled regex
        result = BLOCK_COMMENT_REGEX
            .replace_all(&result, "/* [COMMENT] */")
            .to_string();

        // Remove hash comments using pre-compiled regex
        result = HASH_COMMENT_REGEX
            .replace_all(&result, "# [COMMENT]")
            .to_string();

        result
    }

    fn anonymize_file_path(&self, file_path: &str) -> String {
        if file_path.is_empty() {
            return file_path.to_string();
        }

        let parts: Vec<&str> = file_path.split('/').collect();
        if parts.is_empty() {
            return file_path.to_string();
        }

        let filename = parts.last().unwrap_or(&"");
        let dir_path = parts[..parts.len().saturating_sub(1)].join("/");
        let dir_hash = self.simple_hash(&dir_path);

        format!("[DIR_{dir_hash}]/{filename}")
    }

    fn simple_hash(&self, input: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();

        // Convert to base36 and take first 6 chars
        radix_fmt::radix(hash, 36)
            .to_string()
            .chars()
            .take(6)
            .collect()
    }

    /// Validate that a glob pattern is safe to use.
    /// 
    /// This prevents injection attacks through malicious glob patterns.
    fn is_safe_glob_pattern(&self, pattern: &str) -> bool {
        // Basic safety checks
        if pattern.is_empty() || pattern.len() > 256 {
            return false;
        }

        // Check for potentially dangerous patterns
        let dangerous_patterns = [
            "../",      // Path traversal
            "..\\",     // Windows path traversal  
            "/etc/",    // System directories
            "/proc/",   // Process information
            "/sys/",    // System information
            "C:\\",     // Windows system root
            "\\\\",     // UNC paths
        ];

        for dangerous in &dangerous_patterns {
            if pattern.contains(dangerous) {
                return false;
            }
        }

        // Only allow alphanumeric, common path chars, and glob wildcards
        pattern.chars().all(|c| {
            c.is_alphanumeric() 
                || c == '/' 
                || c == '\\' 
                || c == '.' 
                || c == '_' 
                || c == '-' 
                || c == '*' 
                || c == '?' 
                || c == '[' 
                || c == ']' 
                || c == '{' 
                || c == '}' 
                || c == ','
        })
    }

    fn limit_diagnostics_per_file(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        if self.policy.max_diagnostics_per_file == 0 {
            return diagnostics;
        }

        let mut file_groups: HashMap<String, Vec<Diagnostic>> = HashMap::new();

        // Group by file
        for diagnostic in diagnostics {
            file_groups
                .entry(diagnostic.file.clone())
                .or_default()
                .push(diagnostic);
        }

        // Limit each group and prioritize by severity
        let mut limited = Vec::new();
        for (_, mut file_diagnostics) in file_groups {
            // Sort by severity (errors first)
            file_diagnostics.sort_by_key(|d| d.severity as u8);

            // Take only the allowed number
            file_diagnostics.truncate(self.policy.max_diagnostics_per_file);
            limited.extend(file_diagnostics);
        }

        limited
    }
}

impl PrivacyFilterTrait for PrivacyFilter {
    fn apply(&self, diagnostics: Vec<Diagnostic>) -> Result<Vec<Diagnostic>> {
        let mut filtered: Vec<Diagnostic> = diagnostics
            .into_iter()
            .filter(|d| self.should_include_diagnostic(d))
            .map(|d| self.sanitize_diagnostic(d))
            .collect();

        // Apply per-file limits
        if self.policy.max_diagnostics_per_file > 0 {
            filtered = self.limit_diagnostics_per_file(filtered);
        }

        Ok(filtered)
    }

    fn should_include_diagnostic(&self, diagnostic: &Diagnostic) -> bool {
        // First check workspace filter if available
        if let Some(ref workspace_filter) = self.workspace_filter {
            let file_path = Path::new(&diagnostic.file);
            if !workspace_filter.should_include_file(file_path) {
                return false;
            }
        }

        // Check against exclusion patterns with proper validation
        for pattern in &self.policy.exclude_patterns {
            // Validate pattern before using it to prevent regex injection
            if self.is_safe_glob_pattern(pattern) {
                match glob::Pattern::new(pattern) {
                    Ok(p) => {
                        if p.matches(&diagnostic.file) {
                            return false;
                        }
                    }
                    Err(_) => {
                        // Invalid pattern - log warning but continue processing
                        eprintln!("Warning: Invalid glob pattern ignored: {pattern}");
                        continue;
                    }
                }
            } else {
                // Unsafe pattern - log warning and skip
                eprintln!("Warning: Potentially unsafe glob pattern ignored: {pattern}");
                continue;
            }
        }

        // Check severity filters
        if self.policy.include_only_errors && diagnostic.severity != DiagnosticSeverity::Error {
            return false;
        }

        true
    }

    fn sanitize_diagnostic(&self, mut diagnostic: Diagnostic) -> Diagnostic {
        // Sanitize message content
        if self.policy.sanitize_strings {
            diagnostic.message = self.sanitize_string_literals(&diagnostic.message);
        }

        if self.policy.sanitize_comments {
            diagnostic.message = self.sanitize_comments(&diagnostic.message);
        }

        // Anonymize file paths if requested
        if self.policy.anonymize_file_paths {
            diagnostic.file = self.anonymize_file_path(&diagnostic.file);
        }

        // Sanitize related information
        if let Some(related_info) = &mut diagnostic.related_information {
            for info in related_info.iter_mut() {
                if self.policy.sanitize_strings {
                    info.message = self.sanitize_string_literals(&info.message);
                }

                if self.policy.anonymize_file_paths {
                    info.location.uri = self.anonymize_file_path(&info.location.uri);
                }
            }
        }

        diagnostic
    }
}

// Add regex to dependencies
// In Cargo.toml, add: regex = "1.0"
