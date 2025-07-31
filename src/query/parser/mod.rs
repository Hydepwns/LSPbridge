//! Query language parser for LSP Bridge diagnostics
//!
//! This module provides a complete query language implementation for searching and
//! filtering diagnostic data. The language supports SQL-like syntax with extensions
//! for time-based queries and diagnostic-specific operations.
//!
//! # Query Language Features
//!
//! - **SELECT clauses**: `*`, `COUNT(*)`, field lists, aggregation functions
//! - **FROM clauses**: `diagnostics`, `files`, `history`, `trends`
//! - **WHERE clauses**: Field filters, time ranges, severity filters
//! - **GROUP BY**: Grouping by multiple fields
//! - **ORDER BY**: Sorting with ASC/DESC
//! - **LIMIT**: Result set limiting  
//! - **Time ranges**: Relative (`LAST 7 DAYS`) and absolute timestamps
//!
//! # Example Usage
//!
//! ```rust
//! use lsp_bridge::query::parser::{QueryParser, QueryValidator};
//!
//! // Parse a query
//! let parser = QueryParser::new();
//! let query = parser.parse("SELECT * FROM diagnostics WHERE severity = 'error' AND LAST 24 HOURS")?;
//!
//! // Validate the query
//! let validator = QueryValidator::new();
//! validator.validate(&query)?;
//!
//! println!("Parsed query: {:?}", query);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Query Examples
//!
//! ```sql
//! -- Select all recent errors
//! SELECT * FROM diagnostics WHERE severity = 'error' AND LAST 7 DAYS
//!
//! -- Count warnings by category
//! SELECT COUNT(*) FROM diagnostics WHERE severity = 'warning' GROUP BY category
//!
//! -- Find files with many diagnostics
//! SELECT path, COUNT(*) FROM diagnostics GROUP BY path ORDER BY COUNT(*) DESC LIMIT 10
//!
//! -- Get trending diagnostic categories
//! SELECT category, COUNT(*) FROM trends WHERE LAST 30 DAYS GROUP BY category
//! ```

pub mod ast;
pub mod errors;
pub mod grammar;
pub mod lexer;

// Re-export main types for convenience
pub use ast::{
    Comparison, ComparisonFilter, FromClause, GroupByClause, MessageFilter, OrderByClause,
    OrderDirection, PathFilter, Query, QueryAggregation, QueryFilter, RelativeTime, SelectClause,
    SeverityFilter, TimeRange,
};
pub use errors::{
    OptimizationSuggestion, QueryOptimizer, QueryValidator, SuggestionSeverity, SuggestionType,
};
pub use grammar::Parser;
pub use lexer::{Lexer, Token, TokenType};

use crate::core::errors::ParseError;

/// Main query parser providing a simple interface for parsing query strings
///
/// This is the primary entry point for parsing query language strings into
/// structured Query ASTs. It combines lexical analysis, parsing, and validation
/// into a single convenient interface.
///
/// # Example
///
/// ```rust
/// use lsp_bridge::query::parser::QueryParser;
///
/// let parser = QueryParser::new();
/// let query = parser.parse("SELECT COUNT(*) FROM diagnostics WHERE severity = 'error'")?;
/// println!("Parsed {} filters", query.filters.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct QueryParser {
    validator: QueryValidator,
}

impl QueryParser {
    /// Create a new query parser with default validation rules
    pub fn new() -> Self {
        Self {
            validator: QueryValidator::new(),
        }
    }

    /// Create a query parser with a custom validator
    pub fn with_validator(validator: QueryValidator) -> Self {
        Self { validator }
    }

    /// Parse a query string into a Query AST
    ///
    /// This method performs the complete parsing pipeline:
    /// 1. Lexical analysis (tokenization)
    /// 2. Syntax analysis (parsing)
    /// 3. Semantic analysis (validation)
    ///
    /// # Arguments
    ///
    /// * `input` - The query string to parse
    ///
    /// # Returns
    ///
    /// * `Ok(Query)` - Successfully parsed and validated query
    /// * `Err(ParseError)` - Parsing or validation error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lsp_bridge::query::parser::QueryParser;
    ///
    /// let parser = QueryParser::new();
    ///
    /// // Simple query
    /// let query = parser.parse("SELECT * FROM diagnostics")?;
    /// assert_eq!(query.filters.len(), 0);
    ///
    /// // Query with filters
    /// let query = parser.parse("SELECT * FROM diagnostics WHERE severity = 'error'")?;
    /// assert_eq!(query.filters.len(), 1);
    ///
    /// // Query with time range
    /// let query = parser.parse("SELECT COUNT(*) FROM diagnostics WHERE LAST 7 DAYS")?;
    /// assert!(query.time_range.is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(&self, input: &str) -> Result<Query, ParseError> {
        // Step 1: Tokenize the input
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;

        // Step 2: Parse tokens into AST
        let mut parser = Parser::new(tokens);
        let query = parser.parse()?;

        // Step 3: Validate the query
        if let Err(errors) = self.validator.validate(&query) {
            // Return the first validation error
            // In a real implementation, you might want to collect all errors
            return Err(errors.into_iter().next().unwrap());
        }

        Ok(query)
    }

    /// Parse a query string without validation
    ///
    /// This method skips semantic validation and returns the raw parsed AST.
    /// Useful for parsing potentially invalid queries for analysis or debugging.
    ///
    /// # Arguments
    ///
    /// * `input` - The query string to parse
    ///
    /// # Returns
    ///
    /// * `Ok(Query)` - Successfully parsed query (may be semantically invalid)
    /// * `Err(ParseError)` - Syntax error in parsing
    pub fn parse_unchecked(&self, input: &str) -> Result<Query, ParseError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    /// Get optimization suggestions for a query
    ///
    /// Analyzes a parsed query and returns suggestions for improving
    /// performance, correctness, or style.
    ///
    /// # Arguments  
    ///
    /// * `query` - The query to analyze
    ///
    /// # Returns
    ///
    /// Vector of optimization suggestions
    pub fn get_optimization_suggestions(&self, query: &Query) -> Vec<OptimizationSuggestion> {
        QueryOptimizer::analyze(query)
    }

    /// Validate a query and return detailed validation results
    ///
    /// Performs comprehensive validation and returns all validation errors
    /// found, rather than just the first one.
    ///
    /// # Arguments
    ///
    /// * `query` - The query to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Query is valid
    /// * `Err(Vec<ParseError>)` - List of validation errors
    pub fn validate_query(&self, query: &Query) -> Result<(), Vec<ParseError>> {
        self.validator.validate(query)
    }

    /// Get the validator used by this parser
    ///
    /// Returns a reference to the internal validator, allowing access
    /// to valid field names and other validation metadata.
    pub fn validator(&self) -> &QueryValidator {
        &self.validator
    }

    /// Get a mutable reference to the validator
    ///
    /// Allows customization of validation rules after parser creation.
    pub fn validator_mut(&mut self) -> &mut QueryValidator {
        &mut self.validator
    }
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for parsing a query string
///
/// This is a shorthand for creating a parser and parsing a query.
/// Useful for one-off parsing operations.
///
/// # Arguments
///
/// * `input` - The query string to parse
///
/// # Returns
///
/// * `Ok(Query)` - Successfully parsed and validated query
/// * `Err(ParseError)` - Parsing or validation error
///
/// # Example
///
/// ```rust
/// use lsp_bridge::query::parser::parse_query;
///
/// let query = parse_query("SELECT * FROM diagnostics WHERE severity = 'error'")?;
/// println!("Found {} filters", query.filters.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_query(input: &str) -> Result<Query, ParseError> {
    let parser = QueryParser::new();
    parser.parse(input)
}

/// Convenience function for parsing a query without validation
///
/// # Arguments
///
/// * `input` - The query string to parse
///
/// # Returns
///
/// * `Ok(Query)` - Successfully parsed query (may be semantically invalid)
/// * `Err(ParseError)` - Syntax error in parsing
pub fn parse_query_unchecked(input: &str) -> Result<Query, ParseError> {
    let parser = QueryParser::new();
    parser.parse_unchecked(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_integration_simple() -> Result<(), ParseError> {
        let parser = QueryParser::new();
        
        let query = parser.parse("SELECT * FROM diagnostics")?;
        assert_eq!(query.select, SelectClause::All);
        assert_eq!(query.from, FromClause::Diagnostics);
        assert!(query.filters.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_parser_integration_with_filters() -> Result<(), ParseError> {
        let parser = QueryParser::new();
        
        let query = parser.parse("SELECT * FROM diagnostics WHERE severity = 'error' AND path LIKE '*.rs'")?;
        assert_eq!(query.filters.len(), 2);
        
        Ok(())
    }

    #[test]
    fn test_parser_integration_aggregation() -> Result<(), ParseError> {
        let parser = QueryParser::new();
        
        let query = parser.parse("SELECT COUNT(*) FROM diagnostics GROUP BY severity")?;
        assert_eq!(query.select, SelectClause::Count);
        assert!(query.group_by.is_some());
        
        Ok(())
    }

    #[test]
    fn test_parser_integration_time_range() -> Result<(), ParseError> {
        let parser = QueryParser::new();
        
        let query = parser.parse("SELECT * FROM diagnostics WHERE LAST 7 DAYS")?;
        assert!(query.time_range.is_some());
        
        if let Some(TimeRange { relative: Some(RelativeTime::LastDays(7)), .. }) = query.time_range {
            // Success
        } else {
            panic!("Expected 7 day time range");
        }
        
        Ok(())
    }

    #[test]
    fn test_parser_integration_complex_query() -> Result<(), ParseError> {
        let parser = QueryParser::new();
        
        let query = parser.parse(
            "SELECT path, COUNT(*) FROM diagnostics WHERE severity = 'error' AND LAST 24 HOURS GROUP BY path ORDER BY COUNT(*) DESC LIMIT 10"
        )?;
        
        assert!(matches!(query.select, SelectClause::Fields(_)));
        assert_eq!(query.filters.len(), 1);
        assert!(query.time_range.is_some());
        assert!(query.group_by.is_some());
        assert!(query.order_by.is_some());
        assert_eq!(query.limit, Some(10));
        
        Ok(())
    }

    #[test]
    fn test_convenience_functions() -> Result<(), ParseError> {
        let query1 = parse_query("SELECT * FROM diagnostics")?;
        let query2 = parse_query_unchecked("SELECT * FROM diagnostics")?;
        
        assert_eq!(query1.select, query2.select);
        assert_eq!(query1.from, query2.from);
        
        Ok(())
    }

    #[test]
    fn test_optimization_suggestions() -> Result<(), ParseError> {
        let parser = QueryParser::new();
        let query = parser.parse("SELECT * FROM history")?;
        
        let suggestions = parser.get_optimization_suggestions(&query);
        assert!(!suggestions.is_empty());
        
        // Should suggest adding LIMIT and time range for history queries
        assert!(suggestions.iter().any(|s| s.message.contains("LIMIT")));
        assert!(suggestions.iter().any(|s| s.message.contains("time range")));
        
        Ok(())
    }

    #[test]
    fn test_validation_integration() {
        let parser = QueryParser::new();
        
        // Valid query should parse successfully
        assert!(parser.parse("SELECT path FROM diagnostics").is_ok());
        
        // Invalid field should fail validation
        assert!(parser.parse("SELECT invalid_field FROM diagnostics").is_err());
        
        // Invalid limit should fail validation
        assert!(parser.parse("SELECT * FROM diagnostics LIMIT 0").is_err());
    }

    #[test]
    fn test_custom_validator() {
        let mut validator = QueryValidator::new();
        validator.add_valid_field("custom_field".to_string());
        
        let parser = QueryParser::with_validator(validator);
        
        // Should now accept the custom field
        assert!(parser.parse("SELECT custom_field FROM diagnostics").is_ok());
    }
}