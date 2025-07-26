use crate::core::{
    PrivacyFilter as PrivacyFilterTrait, PrivacyPolicy, Diagnostic, DiagnosticSeverity
};
use anyhow::Result;
use std::collections::HashMap;

pub struct PrivacyFilter {
    policy: PrivacyPolicy,
}

impl PrivacyFilter {
    pub fn new(policy: PrivacyPolicy) -> Self {
        Self { policy }
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

    pub fn update_policy(&mut self, policy: PrivacyPolicy) {
        self.policy = policy;
    }

    pub fn get_policy(&self) -> &PrivacyPolicy {
        &self.policy
    }

    fn sanitize_string_literals(&self, message: &str) -> String {
        // Remove quoted strings but preserve structure
        let mut result = message.to_string();
        
        // Replace double quotes
        result = regex::Regex::new(r#""[^"]*""#)
            .unwrap()
            .replace_all(&result, r#""[STRING]""#)
            .to_string();
        
        // Replace single quotes
        result = regex::Regex::new(r"'[^']*'")
            .unwrap()
            .replace_all(&result, "'[STRING]'")
            .to_string();
        
        // Replace template literals
        result = regex::Regex::new(r"`[^`]*`")
            .unwrap()
            .replace_all(&result, "`[STRING]`")
            .to_string();
        
        result
    }

    fn sanitize_comments(&self, message: &str) -> String {
        let mut result = message.to_string();
        
        // Remove line comments
        result = regex::Regex::new(r"//.*$")
            .unwrap()
            .replace_all(&result, "// [COMMENT]")
            .to_string();
        
        // Remove block comments
        result = regex::Regex::new(r"/\*[\s\S]*?\*/")
            .unwrap()
            .replace_all(&result, "/* [COMMENT] */")
            .to_string();
        
        // Remove hash comments
        result = regex::Regex::new(r"#.*$")
            .unwrap()
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

        format!("[DIR_{}]/{}", dir_hash, filename)
    }

    fn simple_hash(&self, input: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Convert to base36 and take first 6 chars
        radix_fmt::radix(hash, 36).to_string().chars().take(6).collect()
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
                .or_insert_with(Vec::new)
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
        // Check against exclusion patterns
        for pattern in &self.policy.exclude_patterns {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(&diagnostic.file))
                .unwrap_or(false)
            {
                return false;
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