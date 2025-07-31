//! Grammar module for the query parser
//! 
//! This module provides a comprehensive grammar framework for parsing
//! query language expressions with proper error handling, recovery,
//! and validation.

pub mod types;
pub mod parser;
pub mod rules;
pub mod utilities;

// Re-export main types and functionality
pub use types::{
    ParserState, ParsingContext, ProductionRule, ParseResult, 
    ValueParser, DefaultValueParser, GrammarValidator
};
pub use parser::Parser;
pub use rules::{QueryRules, ClauseRules, FilterRules, ExpressionRules};
pub use utilities::{ParserUtilities, RecoveryStrategy, ParserMetrics};

// Re-export AST types from parent module for convenience
pub use super::ast::*;

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::lexer::Lexer;

    fn parse_query_with_grammar(input: &str) -> ParseResult<Query> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_comprehensive_query_parsing() {
        let queries = [
            "SELECT * FROM diagnostics",
            "SELECT COUNT(*) FROM files",
            "SELECT file, line, message FROM diagnostics WHERE severity = 'error'",
            "SELECT * FROM diagnostics WHERE LAST 7 DAYS",
            "SELECT * FROM diagnostics WHERE severity = 'error' AND file LIKE '*.rs'",
            "SELECT file, severity FROM diagnostics GROUP BY file ORDER BY severity DESC LIMIT 10",
        ];

        for query in &queries {
            let result = parse_query_with_grammar(query);
            assert!(result.is_ok(), "Failed to parse query: {}", query);
        }
    }

    #[test]
    fn test_error_handling_and_recovery() {
        let invalid_queries = [
            "SELECT", // Missing FROM
            "SELECT * FROM", // Missing table name
            "SELECT * FROM unknown_table", // Invalid table
            "SELECT * FROM diagnostics WHERE", // Missing condition
            "SELECT * FROM diagnostics LIMIT 0", // Invalid limit
            "SELECT * FROM diagnostics WHERE severity = 'invalid'", // Invalid severity
        ];

        for query in &invalid_queries {
            let result = parse_query_with_grammar(query);
            assert!(result.is_err(), "Should have failed to parse: {}", query);
        }
    }

    #[test]
    fn test_parser_state_management() {
        let mut lexer = Lexer::new("SELECT * FROM diagnostics");
        let tokens = lexer.tokenize().unwrap();
        let mut state = ParserState::new(tokens);

        // Test initial state
        assert!(!state.is_at_end());
        assert_eq!(state.peek().token_type, super::super::lexer::TokenType::Select);

        // Test advancement
        state.advance();
        assert_eq!(state.peek().token_type, super::super::lexer::TokenType::Asterisk);

        // Test token matching
        assert!(state.match_token(&super::super::lexer::TokenType::Asterisk));
        assert_eq!(state.peek().token_type, super::super::lexer::TokenType::From);
    }

    #[test]
    fn test_parsing_context() {
        let mut context = ParsingContext::new();

        // Test rule management
        context.enter_rule(ProductionRule::Query);
        assert_eq!(context.current_rule, Some(ProductionRule::Query));

        context.expect_token(super::super::lexer::TokenType::Select);
        assert!(context.expected_tokens.contains(&super::super::lexer::TokenType::Select));

        context.exit_rule();
        assert_eq!(context.current_rule, None);
        assert!(context.expected_tokens.is_empty());
    }

    #[test]
    fn test_value_parsing() {
        let parser = DefaultValueParser;

        // Test string parsing with quotes
        assert_eq!(parser.parse_string_value("\"hello world\""), "hello world");
        assert_eq!(parser.parse_string_value("'test'"), "test");
        assert_eq!(parser.parse_string_value("unquoted"), "unquoted");

        // Test number parsing
        assert_eq!(parser.parse_number_value("42").unwrap(), 42.0);
        assert_eq!(parser.parse_number_value("3.14159").unwrap(), 3.14159);
        assert!(parser.parse_number_value("not_a_number").is_err());

        // Test boolean parsing
        assert_eq!(parser.parse_boolean_value("true").unwrap(), true);
        assert_eq!(parser.parse_boolean_value("FALSE").unwrap(), false);
        assert!(parser.parse_boolean_value("maybe").is_err());
    }

    #[test]
    fn test_grammar_validation() {
        // Test valid query
        let valid_query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            time_range: None,
            group_by: None,
            order_by: None,
            limit: None,
        };
        assert!(GrammarValidator::validate_query(&valid_query).is_ok());

        // Test invalid query with empty GROUP BY
        let invalid_query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            time_range: None,
            group_by: Some(GroupByClause { fields: Vec::new() }),
            order_by: None,
            limit: None,
        };
        assert!(GrammarValidator::validate_query(&invalid_query).is_err());
    }

    #[test]
    fn test_parser_utilities() {
        let utils = ParserUtilities::new();

        // Test token sequence validation
        let valid_sequence = vec![
            super::super::lexer::TokenType::Select,
            super::super::lexer::TokenType::Asterisk,
            super::super::lexer::TokenType::From,
            super::super::lexer::TokenType::Identifier,
        ];
        assert!(utils.validate_token_sequence(&valid_sequence));

        // Test error suggestions
        let error = crate::core::errors::ParseError::UnknownTable {
            table: "diagnostic".to_string(),
            line: 1,
            column: 15,
        };
        let suggestion = utils.suggest_correction(&error);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("diagnostics"));
    }

    #[test]
    fn test_parser_metrics() {
        let mut metrics = ParserMetrics::default();
        
        // Simulate parsing activity
        metrics.tokens_parsed = 100;
        metrics.errors_encountered = 2;
        metrics.recoveries_attempted = 2;
        metrics.recoveries_successful = 1;
        metrics.cache_hits = 75;
        metrics.cache_misses = 25;

        // Test metric calculations
        assert_eq!(metrics.success_rate(), 0.98);
        assert_eq!(metrics.recovery_rate(), 0.5);
        assert_eq!(metrics.cache_hit_rate(), 0.75);
    }

    #[test]
    fn test_complex_query_structures() {
        // Test query with multiple filters
        let complex_query = "SELECT file, severity, message FROM diagnostics WHERE severity = 'error' AND file LIKE '*.rs' AND LAST 30 DAYS GROUP BY file ORDER BY severity DESC LIMIT 50";
        let result = parse_query_with_grammar(complex_query);
        assert!(result.is_ok());

        let query = result.unwrap();
        assert!(matches!(query.select, SelectClause::Fields(_)));
        assert_eq!(query.from, FromClause::Diagnostics);
        assert!(!query.filters.is_empty());
        assert!(query.time_range.is_some());
        assert!(query.group_by.is_some());
        assert!(query.order_by.is_some());
        assert_eq!(query.limit, Some(50));
    }

    #[test]
    fn test_edge_cases() {
        // Test empty input
        let result = parse_query_with_grammar("");
        assert!(result.is_err());

        // Test whitespace-only input
        let result = parse_query_with_grammar("   \n\t  ");
        assert!(result.is_err());

        // Test case sensitivity
        let result = parse_query_with_grammar("select * from diagnostics");
        assert!(result.is_ok()); // Should be case-insensitive

        // Test SQL injection attempts (should be handled safely)
        let result = parse_query_with_grammar("SELECT * FROM diagnostics; DROP TABLE users; --");
        // The parser should handle this gracefully (either parse or error safely)
        // We don't expect it to execute any dangerous operations
    }
}