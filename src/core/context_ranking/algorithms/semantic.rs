use crate::core::types::Diagnostic;

pub struct SemanticScorer;

impl SemanticScorer {
    /// Score based on semantic similarity between names
    pub fn score_name_similarity(item_name: &str, diagnostic_text: &str) -> f32 {
        // Exact match
        if diagnostic_text.contains(item_name) {
            return 1.0;
        }

        // Case-insensitive match
        let lower_item = item_name.to_lowercase();
        let lower_diag = diagnostic_text.to_lowercase();
        if lower_diag.contains(&lower_item) {
            return 0.9;
        }

        // Partial match (item name is substring of word in diagnostic)
        let words_in_diagnostic: Vec<&str> = diagnostic_text.split_whitespace().collect();
        for word in words_in_diagnostic {
            if word.contains(item_name) || item_name.contains(word) {
                return 0.7;
            }
        }

        // Check for common prefixes/suffixes
        if Self::has_common_parts(item_name, diagnostic_text) {
            return 0.5;
        }

        0.0
    }

    /// Score based on diagnostic type and context type matching
    pub fn score_type_relevance(diagnostic: &Diagnostic) -> TypeRelevanceScores {
        let msg = &diagnostic.message.to_lowercase();

        TypeRelevanceScores {
            type_error: if msg.contains("type") || msg.contains("cannot assign") || msg.contains("incompatible") {
                1.0
            } else {
                0.5
            },
            
            import_error: if msg.contains("cannot find") || msg.contains("import") || msg.contains("module") {
                1.0
            } else {
                0.3
            },
            
            variable_error: if msg.contains("undefined") || msg.contains("not defined") || msg.contains("variable") {
                1.0
            } else {
                0.4
            },
            
            function_error: if msg.contains("function") || msg.contains("method") || msg.contains("call") {
                1.0
            } else {
                0.5
            },
            
            class_error: if msg.contains("class") || msg.contains("struct") || msg.contains("interface") {
                1.0
            } else {
                0.4
            },
        }
    }

    /// Check for common parts between names (prefixes, suffixes, substrings)
    fn has_common_parts(name1: &str, text: &str) -> bool {
        // Split camelCase and snake_case
        let parts1 = Self::split_identifier(name1);
        let text_lower = text.to_lowercase();

        // Check if any significant part appears in the text
        parts1.iter().any(|part| {
            part.len() > 3 && text_lower.contains(&part.to_lowercase())
        })
    }

    /// Split identifier into parts (handles camelCase, snake_case, etc.)
    fn split_identifier(identifier: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut current_start = 0;

        for (i, ch) in identifier.char_indices() {
            if i > 0 && (ch.is_uppercase() || ch == '_') {
                if current_start < i {
                    parts.push(&identifier[current_start..i]);
                }
                current_start = if ch == '_' { i + 1 } else { i };
            }
        }

        if current_start < identifier.len() {
            parts.push(&identifier[current_start..]);
        }

        parts.into_iter().filter(|s| !s.is_empty()).collect()
    }
}

pub struct TypeRelevanceScores {
    pub type_error: f32,
    pub import_error: f32,
    pub variable_error: f32,
    pub function_error: f32,
    pub class_error: f32,
}