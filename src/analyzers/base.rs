use super::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use std::borrow::Cow;

/// Common analyzer functionality to reduce duplication
pub trait AnalyzerBase {
    /// Create a standard diagnostic analysis with common fields
    fn create_analysis(
        &self,
        category: DiagnosticCategory,
        confidence: f32,
        complexity: u8,
        cause: String,
        symbols: Vec<String>,
    ) -> DiagnosticAnalysis {
        DiagnosticAnalysis {
            category,
            likely_cause: cause,
            confidence,
            related_symbols: symbols,
            is_cascading: false,
            fix_complexity: complexity,
            insights: Vec::new(),
        }
    }

    /// Add insight to an existing analysis
    fn add_insight(&self, analysis: &mut DiagnosticAnalysis, insight: &str) {
        analysis.insights.push(insight.to_string());
    }

    /// Mark analysis as potentially cascading
    fn mark_cascading(&self, analysis: &mut DiagnosticAnalysis) {
        analysis.is_cascading = true;
        analysis.confidence *= 0.9; // Slightly reduce confidence for cascading errors
    }

    /// Extract common quoted identifiers from diagnostic message
    fn extract_identifiers(&self, message: &str) -> Vec<String> {
        DiagnosticPatterns::extract_quoted_identifiers(message)
    }

    /// Determine if error is likely a type mismatch
    fn is_type_mismatch(&self, message: &str) -> bool {
        message.contains("expected") && message.contains("found")
            || message.contains("type mismatch")
            || message.contains("cannot be assigned to")
    }

    /// Determine if error is likely missing import/dependency
    fn is_missing_import(&self, message: &str) -> bool {
        message.contains("cannot find")
            || message.contains("not found")
            || message.contains("unresolved import")
            || message.contains("module not found")
    }

    /// Common syntax error patterns
    fn is_syntax_error(&self, message: &str) -> bool {
        message.contains("unexpected token")
            || message.contains("expected")
                && (message.contains("';'") || message.contains("'}'") || message.contains("')'"))
            || message.contains("unterminated")
    }

    /// Determine error category based on common patterns
    fn categorize_by_message(&self, message: &str) -> DiagnosticCategory {
        if self.is_syntax_error(message) {
            DiagnosticCategory::SyntaxError
        } else if self.is_type_mismatch(message) {
            DiagnosticCategory::TypeMismatch
        } else if self.is_missing_import(message) {
            DiagnosticCategory::MissingImport
        } else {
            DiagnosticCategory::UndefinedVariable
        }
    }

    /// Calculate confidence based on message specificity
    fn calculate_confidence(&self, message: &str, has_context: bool) -> f32 {
        let mut confidence: f32 = 0.7; // Base confidence

        // More specific messages get higher confidence
        if message.len() > 50 {
            confidence += 0.1;
        }

        // Context improves confidence
        if has_context {
            confidence += 0.1;
        }

        // Known patterns get higher confidence
        if self.is_syntax_error(message) || self.is_type_mismatch(message) {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    /// Normalize diagnostic message - returns Cow to avoid unnecessary allocations
    /// when the message doesn't need modification
    fn normalize_message<'a>(&self, message: &'a str) -> Cow<'a, str> {
        // Common patterns that need normalization
        if message.contains("  ") {
            // Collapse multiple spaces
            Cow::Owned(message.split_whitespace().collect::<Vec<_>>().join(" "))
        } else if message.ends_with(".") {
            // Remove trailing period for consistency
            Cow::Owned(message.trim_end_matches('.').to_string())
        } else if message.starts_with(char::is_lowercase) {
            // Capitalize first letter
            let mut chars = message.chars();
            match chars.next() {
                None => Cow::Borrowed(message),
                Some(first) => {
                    Cow::Owned(first.to_uppercase().collect::<String>() + chars.as_str())
                }
            }
        } else {
            // No normalization needed
            Cow::Borrowed(message)
        }
    }
}

/// Common diagnostic pattern extraction utilities
pub struct DiagnosticPatterns;

impl DiagnosticPatterns {
    /// Extract quoted identifiers from message (e.g., 'foo', "bar")
    pub fn extract_quoted_identifiers(message: &str) -> Vec<String> {
        let mut identifiers = Vec::new();

        // Extract single-quoted identifiers
        let mut chars = message.chars().peekable();
        let mut current_quote = None;
        let mut current_identifier = String::new();

        while let Some(ch) = chars.next() {
            match (ch, current_quote) {
                ('\'', None) | ('"', None) => {
                    current_quote = Some(ch);
                    current_identifier.clear();
                }
                (quote_char, Some(expected)) if quote_char == expected => {
                    if !current_identifier.is_empty() {
                        identifiers.push(current_identifier.clone());
                    }
                    current_quote = None;
                    current_identifier.clear();
                }
                (ch, Some(_)) => {
                    current_identifier.push(ch);
                }
                _ => {}
            }
        }

        identifiers
    }

    /// Extract type information from error messages
    pub fn extract_types(message: &str) -> Vec<String> {
        let mut types = Vec::new();

        // Common type patterns
        let type_patterns = [
            r"expected `([^`]+)`",
            r"found `([^`]+)`",
            r"type `([^`]+)`",
            r": ([A-Z][a-zA-Z0-9_]*)",
        ];

        for pattern in &type_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for cap in regex.captures_iter(message) {
                    if let Some(type_match) = cap.get(1) {
                        types.push(type_match.as_str().to_string());
                    }
                }
            }
        }

        types
    }

    /// Extract numeric values from messages (line numbers, column numbers, etc.)
    pub fn extract_numbers(message: &str) -> Vec<u32> {
        message
            .split_whitespace()
            .filter_map(|word| word.parse::<u32>().ok())
            .collect()
    }
}

/// Utility for consistent fix complexity scoring
pub struct ComplexityScorer;

impl ComplexityScorer {
    /// Score fix complexity from 1 (trivial) to 10 (major refactoring)
    pub fn score_fix_complexity(category: &DiagnosticCategory, message: &str) -> u8 {
        match category {
            DiagnosticCategory::SyntaxError => {
                if message.contains("missing") || message.contains("expected") {
                    1 // Usually just adding a character
                } else {
                    2
                }
            }
            DiagnosticCategory::MissingImport => 2, // Usually just adding an import
            DiagnosticCategory::TypeMismatch => {
                if message.contains("cannot be assigned") {
                    3 // Might need type conversion
                } else {
                    4 // Might need interface changes
                }
            }
            DiagnosticCategory::UndefinedVariable => 6, // Requires understanding business logic
            _ => 5,                                     // Default medium complexity
        }
    }

    /// Find similar name using simple string distance
    pub fn find_similar_name(target: &str, candidates: &[String]) -> Option<String> {
        let mut best_match = None;
        let mut best_distance = usize::MAX;

        for candidate in candidates {
            let distance = levenshtein_distance(target, candidate);
            if distance < best_distance && distance <= 2 {
                best_distance = distance;
                best_match = Some(candidate.clone());
            }
        }

        best_match
    }
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
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
                std::cmp::min(
                    matrix[i][j + 1] + 1, // deletion
                    matrix[i + 1][j] + 1, // insertion
                ),
                matrix[i][j] + cost, // substitution
            );
        }
    }

    matrix[len1][len2]
}
