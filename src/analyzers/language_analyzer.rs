use crate::core::{Diagnostic, SemanticContext};

/// Trait for language-specific diagnostic analysis
pub trait LanguageAnalyzer: Send + Sync {
    /// Analyze a diagnostic and provide categorization and insights
    fn analyze_diagnostic(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis;

    /// Suggest potential fixes for the diagnostic
    fn suggest_fix(
        &self,
        diagnostic: &Diagnostic,
        context: Option<&SemanticContext>,
    ) -> Vec<FixSuggestion>;

    /// Determine what additional context would be helpful
    fn extract_context_requirements(&self, diagnostic: &Diagnostic) -> ContextRequirements;

    /// Get the language this analyzer handles
    fn language(&self) -> &str;

    /// Check if this analyzer can handle a specific diagnostic
    fn can_analyze(&self, diagnostic: &Diagnostic) -> bool {
        diagnostic.source.to_lowercase().contains(self.language())
    }
}

/// Analysis result for a diagnostic
#[derive(Debug, Clone)]
pub struct DiagnosticAnalysis {
    /// Category of the diagnostic
    pub category: DiagnosticCategory,
    /// Likely root cause
    pub likely_cause: String,
    /// Confidence in the analysis (0.0 - 1.0)
    pub confidence: f32,
    /// Related types or symbols mentioned in the error
    pub related_symbols: Vec<String>,
    /// Whether this is likely a cascading error
    pub is_cascading: bool,
    /// Estimated complexity to fix (1-5, where 1 is trivial)
    pub fix_complexity: u8,
    /// Additional insights
    pub insights: Vec<String>,
}

/// Categories of diagnostics
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticCategory {
    // Type-related
    TypeMismatch,
    MissingProperty,
    UndefinedType,
    GenericTypeError,

    // Variable/Symbol related
    UndefinedVariable,
    UnusedVariable,
    UninitializedVariable,

    // Import/Module related
    MissingImport,
    CircularDependency,
    ModuleResolution,

    // Syntax
    SyntaxError,
    ParseError,

    // Memory/Ownership (Rust-specific)
    BorrowChecker,
    LifetimeError,
    MoveError,

    // Async/Concurrency
    AsyncError,
    RaceCondition,

    // Best practices
    CodeQuality,
    Performance,
    Security,

    // Other
    Unknown,
}

/// Suggested fix for a diagnostic
#[derive(Debug, Clone)]
pub struct FixSuggestion {
    /// Description of the fix
    pub description: String,
    /// Code snippet showing the fix
    pub code_snippet: Option<String>,
    /// Confidence in this fix (0.0 - 1.0)
    pub confidence: f32,
    /// Whether this fix can be applied automatically
    pub is_automatic: bool,
    /// Prerequisites for this fix
    pub prerequisites: Vec<String>,
}

/// Requirements for additional context
#[derive(Debug, Clone)]
pub struct ContextRequirements {
    /// Files that should be examined
    pub required_files: Vec<String>,
    /// Symbols that need to be looked up
    pub required_symbols: Vec<String>,
    /// Type definitions needed
    pub required_types: Vec<String>,
    /// Configuration files to check
    pub config_files: Vec<String>,
    /// Dependencies to verify
    pub dependencies: Vec<String>,
}

/// Common patterns for diagnostic analysis
pub struct DiagnosticPatterns;

impl DiagnosticPatterns {
    /// Extract quoted identifiers from error messages
    pub fn extract_quoted_identifiers(message: &str) -> Vec<String> {
        let mut identifiers = Vec::new();

        // Single quotes
        let re = regex::Regex::new(r"'([a-zA-Z_][a-zA-Z0-9_]*)'").unwrap();
        for cap in re.captures_iter(message) {
            if let Some(ident) = cap.get(1) {
                identifiers.push(ident.as_str().to_string());
            }
        }

        // Backticks
        let re = regex::Regex::new(r"`([a-zA-Z_][a-zA-Z0-9_]*)`").unwrap();
        for cap in re.captures_iter(message) {
            if let Some(ident) = cap.get(1) {
                identifiers.push(ident.as_str().to_string());
            }
        }

        // Double quotes (for types)
        let re = regex::Regex::new(r#""([a-zA-Z_][a-zA-Z0-9_<>,\s]*?)""#).unwrap();
        for cap in re.captures_iter(message) {
            if let Some(ident) = cap.get(1) {
                identifiers.push(ident.as_str().to_string());
            }
        }

        identifiers
    }

    /// Extract type names from error messages
    pub fn extract_types(message: &str) -> Vec<String> {
        let mut types = Vec::new();

        // Common patterns for types
        let patterns = vec![
            r"type '([^']+)'",
            r"Type '([^']+)'",
            r"of type '([^']+)'",
            r"to type '([^']+)'",
            r"expected '([^']+)'",
            r"found '([^']+)'",
        ];

        for pattern in patterns {
            let re = regex::Regex::new(pattern).unwrap();
            for cap in re.captures_iter(message) {
                if let Some(type_name) = cap.get(1) {
                    types.push(type_name.as_str().to_string());
                }
            }
        }

        types
    }

    /// Check if an error message indicates a missing import
    pub fn is_missing_import(message: &str) -> bool {
        let patterns = vec![
            "cannot find",
            "Cannot find",
            "not found",
            "does not exist",
            "undefined",
            "is not defined",
            "unresolved import",
        ];

        patterns.iter().any(|pattern| message.contains(pattern))
    }

    /// Check if an error is likely a typo
    pub fn find_similar_name(error_name: &str, available_names: &[String]) -> Option<String> {
        // Simple Levenshtein distance check
        let mut best_match = None;
        let mut best_distance = usize::MAX;

        for name in available_names {
            let distance = levenshtein_distance(error_name, name);
            if distance < best_distance && distance <= 3 {
                // Max 3 character difference
                best_distance = distance;
                best_match = Some(name.clone());
            }
        }

        best_match
    }
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                matrix[i][j] + cost,
                std::cmp::min(matrix[i + 1][j] + 1, matrix[i][j + 1] + 1),
            );
        }
    }

    matrix[len1][len2]
}

impl Default for DiagnosticAnalysis {
    fn default() -> Self {
        Self {
            category: DiagnosticCategory::Unknown,
            likely_cause: "Unknown cause".to_string(),
            confidence: 0.5,
            related_symbols: Vec::new(),
            is_cascading: false,
            fix_complexity: 3,
            insights: Vec::new(),
        }
    }
}

impl Default for ContextRequirements {
    fn default() -> Self {
        Self {
            required_files: Vec::new(),
            required_symbols: Vec::new(),
            required_types: Vec::new(),
            config_files: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}
