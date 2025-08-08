//! Grammar-related types and structures

use super::super::ast::*;
use super::super::lexer::{Token, TokenType};
use crate::core::errors::ParseError;

/// Parser state for tracking position and context
#[derive(Debug, Clone)]
pub struct ParserState {
    pub tokens: Vec<Token>,
    pub current: usize,
}

impl ParserState {
    /// Create a new parser state
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Check if we're at the end of tokens
    pub fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Get the current token without advancing
    pub fn peek(&self) -> &Token {
        if self.is_at_end() {
            // Return a dummy EOF token if we're at the end
            static EOF_TOKEN: Token = Token {
                token_type: TokenType::Eof,
                lexeme: String::new(),
                line: 0,
                column: 0,
            };
            &EOF_TOKEN
        } else {
            &self.tokens[self.current]
        }
    }

    /// Get the previous token
    pub fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    /// Advance to the next token and return the current one
    pub fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    /// Check if the current token matches the given type
    pub fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().token_type == token_type
        }
    }

    /// Check if the current token is a string literal
    pub fn check_string(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(self.peek().token_type, TokenType::String(_))
        }
    }

    /// Check if the current token is a number literal
    pub fn check_number(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(self.peek().token_type, TokenType::Number(_))
        }
    }

    /// Check if the current token is an identifier
    pub fn check_identifier(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(self.peek().token_type, TokenType::Identifier(_))
        }
    }

    /// Consume a token if it matches the given type
    pub fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume a token and verify it matches the expected type
    pub fn consume(&mut self, expected: TokenType, message: &str) -> Result<&Token, ParseError> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            let current = self.peek();
            Err(ParseError::UnexpectedToken {
                expected: message.to_string(),
                found: current.lexeme.clone(),
                line: current.line,
                column: current.column,
            })
        }
    }
}

/// Grammar production rules enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ProductionRule {
    Query,
    SelectClause,
    FromClause,
    WhereClause,
    GroupByClause,
    OrderByClause,
    LimitClause,
    FilterExpression,
    ComparisonOperator,
    LogicalOperator,
    TimeRange,
    RelativeTime,
    FieldList,
    StringLiteral,
    NumberLiteral,
    Identifier,
}

/// Parsing context for better error messages
#[derive(Debug, Clone)]
pub struct ParsingContext {
    pub current_rule: Option<ProductionRule>,
    pub expected_tokens: Vec<TokenType>,
    pub error_recovery_points: Vec<usize>,
}

impl Default for ParsingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ParsingContext {
    /// Create a new parsing context
    pub fn new() -> Self {
        Self {
            current_rule: None,
            expected_tokens: Vec::new(),
            error_recovery_points: Vec::new(),
        }
    }

    /// Enter a new production rule
    pub fn enter_rule(&mut self, rule: ProductionRule) {
        self.current_rule = Some(rule);
        self.expected_tokens.clear();
    }

    /// Exit the current production rule
    pub fn exit_rule(&mut self) {
        self.current_rule = None;
        self.expected_tokens.clear();
    }

    /// Add an expected token for better error messages
    pub fn expect_token(&mut self, token_type: TokenType) {
        if !self.expected_tokens.contains(&token_type) {
            self.expected_tokens.push(token_type);
        }
    }

    /// Add an error recovery point
    pub fn add_recovery_point(&mut self, position: usize) {
        self.error_recovery_points.push(position);
    }
}

/// Parse result wrapper for better error handling
pub type ParseResult<T> = Result<T, ParseError>;

/// Helper trait for value parsing
pub trait ValueParser {
    fn parse_string_value(&self, lexeme: &str) -> String;
    fn parse_number_value(&self, lexeme: &str) -> Result<f64, ParseError>;
    fn parse_boolean_value(&self, lexeme: &str) -> Result<bool, ParseError>;
}

/// Default implementation of ValueParser
pub struct DefaultValueParser;

impl ValueParser for DefaultValueParser {
    fn parse_string_value(&self, lexeme: &str) -> String {
        // Remove quotes if present
        if lexeme.starts_with('"') && lexeme.ends_with('"') {
            lexeme[1..lexeme.len()-1].to_string()
        } else if lexeme.starts_with('\'') && lexeme.ends_with('\'') {
            lexeme[1..lexeme.len()-1].to_string()
        } else {
            lexeme.to_string()
        }
    }

    fn parse_number_value(&self, lexeme: &str) -> Result<f64, ParseError> {
        lexeme.parse::<f64>().map_err(|_| ParseError::InvalidNumber {
            value: lexeme.to_string(),
            line: 0, // Line info would need to be passed in
            column: 0,
        })
    }

    fn parse_boolean_value(&self, lexeme: &str) -> Result<bool, ParseError> {
        match lexeme.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ParseError::InvalidBoolean {
                value: lexeme.to_string(),
                line: 0,
                column: 0,
            })
        }
    }
}

/// Grammar rule validation
pub struct GrammarValidator;

impl GrammarValidator {
    /// Validate a query structure
    pub fn validate_query(query: &Query) -> Result<(), ParseError> {
        // Validate that required clauses are present
        Self::validate_select_clause(&query.select)?;
        Self::validate_from_clause(&query.from)?;
        
        // Validate optional clauses if present
        if let Some(ref group_by) = query.group_by {
            Self::validate_group_by_clause(group_by)?;
        }
        
        if let Some(ref order_by) = query.order_by {
            Self::validate_order_by_clause(order_by)?;
        }
        
        Ok(())
    }

    /// Validate select clause
    fn validate_select_clause(_select: &SelectClause) -> Result<(), ParseError> {
        // All select clauses are valid in our current grammar
        Ok(())
    }

    /// Validate from clause
    fn validate_from_clause(_from: &FromClause) -> Result<(), ParseError> {
        // All from clauses are valid in our current grammar
        Ok(())
    }

    /// Validate group by clause
    fn validate_group_by_clause(group_by: &GroupByClause) -> Result<(), ParseError> {
        if group_by.fields.is_empty() {
            return Err(ParseError::EmptyGroupBy);
        }
        Ok(())
    }

    /// Validate order by clause
    fn validate_order_by_clause(order_by: &OrderByClause) -> Result<(), ParseError> {
        if order_by.field.is_empty() {
            return Err(ParseError::EmptyOrderBy);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::lexer::Lexer;

    #[test]
    fn test_parser_state() {
        let mut lexer = Lexer::new("SELECT * FROM test");
        let tokens = lexer.tokenize().unwrap();
        let mut state = ParserState::new(tokens);
        
        // Test initial state
        assert!(!state.is_at_end());
        assert_eq!(state.peek().token_type, TokenType::Select);
        
        // Test advance
        state.advance();
        assert_eq!(state.peek().token_type, TokenType::Asterisk);
        
        // Test match_token
        assert!(state.match_token(&TokenType::Asterisk));
        assert_eq!(state.peek().token_type, TokenType::From);
    }

    #[test]
    fn test_parsing_context() {
        let mut context = ParsingContext::new();
        
        context.enter_rule(ProductionRule::Query);
        assert_eq!(context.current_rule, Some(ProductionRule::Query));
        
        context.expect_token(TokenType::Select);
        assert!(context.expected_tokens.contains(&TokenType::Select));
        
        context.exit_rule();
        assert_eq!(context.current_rule, None);
        assert!(context.expected_tokens.is_empty());
    }

    #[test]
    fn test_value_parser() {
        let parser = DefaultValueParser;
        
        // Test string parsing
        assert_eq!(parser.parse_string_value("\"hello\""), "hello");
        assert_eq!(parser.parse_string_value("'world'"), "world");
        assert_eq!(parser.parse_string_value("test"), "test");
        
        // Test number parsing
        assert_eq!(parser.parse_number_value("42").unwrap(), 42.0);
        assert_eq!(parser.parse_number_value("3.14").unwrap(), 3.14);
        assert!(parser.parse_number_value("not_a_number").is_err());
        
        // Test boolean parsing
        assert_eq!(parser.parse_boolean_value("true").unwrap(), true);
        assert_eq!(parser.parse_boolean_value("FALSE").unwrap(), false);
        assert!(parser.parse_boolean_value("maybe").is_err());
    }

    #[test]
    fn test_grammar_validator() {
        use super::super::super::ast::*;
        
        let query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            time_range: None,
            group_by: None,
            order_by: None,
            limit: None,
        };
        
        assert!(GrammarValidator::validate_query(&query).is_ok());
        
        // Test invalid group by
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
}