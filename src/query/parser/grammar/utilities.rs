//! Parser utility functions and helpers

use super::types::{ParserState, ParsingContext, ParseResult};
use super::super::lexer::{Token, TokenType};
use crate::core::errors::ParseError;
use std::collections::HashMap;

/// Parser utilities for common operations
pub struct ParserUtilities {
    /// Cache for parsed tokens to avoid re-parsing
    token_cache: HashMap<String, Token>,
    
    /// Error recovery strategies
    recovery_strategies: Vec<RecoveryStrategy>,
}

impl ParserUtilities {
    /// Create new parser utilities
    pub fn new() -> Self {
        Self {
            token_cache: HashMap::new(),
            recovery_strategies: vec![
                RecoveryStrategy::SkipToNextStatement,
                RecoveryStrategy::SkipToNextClause,
                RecoveryStrategy::InsertMissingToken,
            ],
        }
    }

    /// Synchronize parser state after error for recovery
    pub fn synchronize(&self, state: &mut ParserState) {
        while !state.is_at_end() {
            if matches!(state.previous().token_type, TokenType::Semicolon) {
                return;
            }

            match state.peek().token_type {
                TokenType::Select |
                TokenType::From |
                TokenType::Where |
                TokenType::GroupBy |
                TokenType::OrderBy |
                TokenType::Limit => return,
                _ => {}
            }

            state.advance();
        }
    }

    /// Check if token sequence is valid
    pub fn validate_token_sequence(&self, tokens: &[TokenType]) -> bool {
        // Basic validation rules for token sequences
        for window in tokens.windows(2) {
            if !self.is_valid_token_pair(&window[0], &window[1]) {
                return false;
            }
        }
        true
    }

    /// Check if two consecutive tokens form a valid pair
    fn is_valid_token_pair(&self, first: &TokenType, second: &TokenType) -> bool {
        match (first, second) {
            // Valid pairs
            (TokenType::Select, TokenType::Asterisk) => true,
            (TokenType::Select, TokenType::Count) => true,
            (TokenType::Select, TokenType::Identifier(_)) => true,
            (TokenType::From, TokenType::Identifier(_)) => true,
            (TokenType::Where, TokenType::Identifier(_)) => true,
            (TokenType::Where, TokenType::Last) => true,
            (TokenType::Identifier(_), TokenType::Equal) => true,
            (TokenType::Identifier(_), TokenType::NotEqual) => true,
            (TokenType::Identifier(_), TokenType::Like) => true,
            (TokenType::Equal, TokenType::String(_)) => true,
            (TokenType::Equal, TokenType::Identifier(_)) => true,
            (TokenType::Equal, TokenType::Number(_)) => true,
            (TokenType::And, TokenType::Identifier(_)) => true,
            (TokenType::Or, TokenType::Identifier(_)) => true,
            (TokenType::GroupBy, TokenType::Identifier(_)) => true,
            (TokenType::OrderBy, TokenType::Identifier(_)) => true,
            (TokenType::Limit, TokenType::Number(_)) => true,
            
            // Invalid pairs that commonly occur due to syntax errors
            (TokenType::Select, TokenType::From) => false, // Missing selection
            (TokenType::From, TokenType::Where) => false,  // Missing table name
            (TokenType::Where, TokenType::GroupBy) => false, // Missing condition
            (TokenType::Equal, TokenType::Equal) => false, // Double equals
            
            // Default: allow most combinations
            _ => true,
        }
    }

    /// Suggest corrections for common syntax errors
    pub fn suggest_correction(&self, error: &ParseError) -> Option<String> {
        match error {
            ParseError::UnexpectedToken { expected, found, .. } => {
                self.suggest_token_correction(expected, found)
            }
            ParseError::UnknownTable { table, .. } => {
                self.suggest_table_correction(table)
            }
            ParseError::InvalidSeverity { severity, .. } => {
                self.suggest_severity_correction(severity)
            }
            _ => None,
        }
    }

    /// Suggest token corrections
    fn suggest_token_correction(&self, expected: &str, found: &str) -> Option<String> {
        let corrections = [
            ("SELECT", vec!["SELCT", "SLECT", "ELECT"]),
            ("FROM", vec!["FORM", "FRM", "FRON"]),
            ("WHERE", vec!["WERE", "WHRE", "WHER"]),
            ("AND", vec!["AN", "AD"]),
            ("OR", vec!["ORR"]),
            ("ORDER BY", vec!["ORDER", "ORDERBY"]),
            ("GROUP BY", vec!["GROUP", "GROUPBY"]),
            ("LIMIT", vec!["LIMTI", "LIMT"]),
        ];

        for (correct, typos) in &corrections {
            if expected.contains(correct) && typos.contains(&found) {
                return Some(format!("Did you mean '{}'?", correct));
            }
        }

        None
    }

    /// Suggest table name corrections
    fn suggest_table_correction(&self, table: &str) -> Option<String> {
        let valid_tables = ["diagnostics", "files", "symbols", "references", "projects"];
        
        // Find closest match using edit distance
        let mut best_match = None;
        let mut best_distance = usize::MAX;
        
        for valid_table in &valid_tables {
            let distance = self.edit_distance(table, valid_table);
            if distance < best_distance && distance <= 2 {
                best_distance = distance;
                best_match = Some(*valid_table);
            }
        }
        
        best_match.map(|table| format!("Did you mean '{}'?", table))
    }

    /// Suggest severity corrections
    fn suggest_severity_correction(&self, severity: &str) -> Option<String> {
        let valid_severities = ["error", "warning", "info", "information", "hint"];
        
        let mut best_match = None;
        let mut best_distance = usize::MAX;
        
        for valid_severity in &valid_severities {
            let distance = self.edit_distance(&severity.to_lowercase(), valid_severity);
            if distance < best_distance && distance <= 2 {
                best_distance = distance;
                best_match = Some(*valid_severity);
            }
        }
        
        best_match.map(|severity| format!("Did you mean '{}'?", severity))
    }

    /// Calculate edit distance between two strings
    fn edit_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();
        
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        
        // Initialize first row and column
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i-1] == s2_chars[j-1] { 0 } else { 1 };
                
                matrix[i][j] = std::cmp::min(
                    std::cmp::min(
                        matrix[i-1][j] + 1,     // deletion
                        matrix[i][j-1] + 1      // insertion
                    ),
                    matrix[i-1][j-1] + cost     // substitution
                );
            }
        }
        
        matrix[len1][len2]
    }

    /// Format error message with context
    pub fn format_error_with_context(&self, error: &ParseError, input: &str) -> String {
        let mut message = format!("Parse error: {}", error);
        
        // Add line context if available
        if let Some((line_num, column)) = self.get_error_position(error) {
            if let Some(line_content) = self.get_line_content(input, line_num) {
                message.push_str(&format!("\n  --> Line {}, Column {}", line_num, column));
                message.push_str(&format!("\n     | {}", line_content));
                
                // Add pointer to error location
                let pointer = " ".repeat(column.saturating_sub(1)) + "^";
                message.push_str(&format!("\n     | {}", pointer));
            }
        }
        
        // Add suggestion if available
        if let Some(suggestion) = self.suggest_correction(error) {
            message.push_str(&format!("\n  Help: {}", suggestion));
        }
        
        message
    }

    /// Extract error position from parse error
    fn get_error_position(&self, error: &ParseError) -> Option<(usize, usize)> {
        match error {
            ParseError::UnexpectedToken { line, column, .. } => Some((*line, *column)),
            ParseError::UnknownTable { line, column, .. } => Some((*line, *column)),
            ParseError::InvalidSeverity { line, column, .. } => Some((*line, *column)),
            ParseError::InvalidDateTime { line, column, .. } => Some((*line, *column)),
            ParseError::InvalidNumber { line, column, .. } => Some((*line, *column)),
            _ => None,
        }
    }

    /// Get content of specific line from input
    fn get_line_content(&self, input: &str, line_num: usize) -> Option<String> {
        input.lines().nth(line_num.saturating_sub(1)).map(|s| s.to_string())
    }

    /// Cache frequently used tokens
    pub fn cache_token(&mut self, key: String, token: Token) {
        self.token_cache.insert(key, token);
    }

    /// Retrieve cached token
    pub fn get_cached_token(&self, key: &str) -> Option<&Token> {
        self.token_cache.get(key)
    }

    /// Clear token cache
    pub fn clear_cache(&mut self) {
        self.token_cache.clear();
    }
}

/// Error recovery strategies
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// Skip tokens until next statement
    SkipToNextStatement,
    /// Skip tokens until next clause (SELECT, FROM, etc.)
    SkipToNextClause,
    /// Insert a missing token
    InsertMissingToken,
    /// Replace current token with expected one
    ReplaceToken,
}

/// Recovery context for error handling
pub struct RecoveryContext {
    pub strategy: RecoveryStrategy,
    pub tokens_skipped: usize,
    pub recovery_successful: bool,
}

impl RecoveryContext {
    /// Create new recovery context
    pub fn new(strategy: RecoveryStrategy) -> Self {
        Self {
            strategy,
            tokens_skipped: 0,
            recovery_successful: false,
        }
    }
}

/// Parser performance metrics
#[derive(Debug, Clone, Default)]
pub struct ParserMetrics {
    pub tokens_parsed: usize,
    pub errors_encountered: usize,
    pub recoveries_attempted: usize,
    pub recoveries_successful: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl ParserMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.tokens_parsed == 0 {
            0.0
        } else {
            1.0 - (self.errors_encountered as f64 / self.tokens_parsed as f64)
        }
    }

    /// Calculate recovery rate
    pub fn recovery_rate(&self) -> f64 {
        if self.recoveries_attempted == 0 {
            0.0
        } else {
            self.recoveries_successful as f64 / self.recoveries_attempted as f64
        }
    }

    /// Calculate cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total_accesses = self.cache_hits + self.cache_misses;
        if total_accesses == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total_accesses as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_sequence_validation() {
        let utils = ParserUtilities::new();
        
        // Valid sequence
        let valid_tokens = vec![TokenType::Select, TokenType::Asterisk, TokenType::From, TokenType::Identifier];
        assert!(utils.validate_token_sequence(&valid_tokens));
        
        // Invalid sequence
        let invalid_tokens = vec![TokenType::Select, TokenType::From]; // Missing selection
        assert!(!utils.validate_token_sequence(&invalid_tokens));
    }

    #[test]
    fn test_edit_distance() {
        let utils = ParserUtilities::new();
        
        assert_eq!(utils.edit_distance("kitten", "sitting"), 3);
        assert_eq!(utils.edit_distance("SELECT", "SELCT"), 1);
        assert_eq!(utils.edit_distance("diagnostics", "diagnostic"), 1);
    }

    #[test]
    fn test_table_suggestion() {
        let utils = ParserUtilities::new();
        
        let suggestion = utils.suggest_table_correction("diagnostic");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("diagnostics"));
        
        let no_suggestion = utils.suggest_table_correction("completely_different");
        assert!(no_suggestion.is_none());
    }

    #[test]
    fn test_severity_suggestion() {
        let utils = ParserUtilities::new();
        
        let suggestion = utils.suggest_severity_correction("eror");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("error"));
        
        let suggestion2 = utils.suggest_severity_correction("warn");
        assert!(suggestion2.is_some());
        assert!(suggestion2.unwrap().contains("warning"));
    }

    #[test]
    fn test_parser_metrics() {
        let mut metrics = ParserMetrics::default();
        metrics.tokens_parsed = 100;
        metrics.errors_encountered = 5;
        metrics.recoveries_attempted = 3;
        metrics.recoveries_successful = 2;
        metrics.cache_hits = 80;
        metrics.cache_misses = 20;
        
        assert_eq!(metrics.success_rate(), 0.95);
        assert_eq!(metrics.recovery_rate(), 2.0 / 3.0);
        assert_eq!(metrics.cache_hit_rate(), 0.8);
    }

    #[test]
    fn test_token_caching() {
        let mut utils = ParserUtilities::new();
        
        let token = Token {
            token_type: TokenType::Select,
            lexeme: "SELECT".to_string(),
            line: 1,
            column: 1,
        };
        
        utils.cache_token("select_token".to_string(), token.clone());
        
        let cached = utils.get_cached_token("select_token");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().lexeme, "SELECT");
        
        utils.clear_cache();
        assert!(utils.get_cached_token("select_token").is_none());
    }
}