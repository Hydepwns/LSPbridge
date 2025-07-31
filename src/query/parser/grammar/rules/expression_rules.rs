//! Expression-level grammar rules

use super::super::types::{ParserState, ParsingContext, ParseResult, ValueParser, DefaultValueParser};
use super::super::super::ast::*;
use super::super::super::lexer::TokenType;
use crate::core::errors::ParseError;

/// Trait for parsing expression-level grammar rules
pub trait ExpressionRules {
    /// Parse logical expression (AND/OR combinations)
    fn parse_logical_expression(&mut self) -> ParseResult<LogicalExpression>;
    
    /// Parse comparison expression
    fn parse_comparison_expression(&mut self) -> ParseResult<ComparisonExpression>;
    
    /// Parse literal value
    fn parse_literal_value(&mut self) -> ParseResult<LiteralValue>;
    
    /// Parse field reference
    fn parse_field_reference(&mut self) -> ParseResult<FieldReference>;
}

/// Logical expression combining multiple filters
#[derive(Debug, Clone, PartialEq)]
pub struct LogicalExpression {
    pub left: Box<FilterExpression>,
    pub operator: LogicalOperator,
    pub right: Box<FilterExpression>,
}

/// Comparison expression for field-value comparisons
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonExpression {
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: LiteralValue,
}

/// Filter expression (either logical or comparison)
#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpression {
    Logical(LogicalExpression),
    Comparison(ComparisonExpression),
    Field(FieldReference),
}

/// Literal value types
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Boolean(bool),
    DateTime(chrono::DateTime<chrono::Utc>),
}

/// Field reference
#[derive(Debug, Clone, PartialEq)]
pub struct FieldReference {
    pub name: String,
    pub qualified: bool, // true if table.field format
}

/// Logical operators
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Like,
    NotLike,
    In,
    NotIn,
}

/// Implementation of expression parsing rules
pub struct ExpressionRuleParser<'a> {
    pub state: &'a mut ParserState,
    pub context: &'a mut ParsingContext,
    pub value_parser: Box<dyn ValueParser>,
}

impl<'a> ExpressionRuleParser<'a> {
    /// Create a new expression rule parser
    pub fn new(state: &'a mut ParserState, context: &'a mut ParsingContext) -> Self {
        Self { 
            state, 
            context, 
            value_parser: Box::new(DefaultValueParser)
        }
    }
    
    /// Create an expression rule parser with custom value parser
    pub fn with_value_parser(
        state: &'a mut ParserState, 
        context: &'a mut ParsingContext,
        value_parser: Box<dyn ValueParser>
    ) -> Self {
        Self { state, context, value_parser }
    }
}

impl<'a> ExpressionRules for ExpressionRuleParser<'a> {
    /// Parse logical expression with precedence
    /// logical_expression = comparison_expression (logical_operator comparison_expression)*
    fn parse_logical_expression(&mut self) -> ParseResult<LogicalExpression> {
        let mut left = self.parse_comparison_as_filter_expression()?;
        
        while self.is_logical_operator() {
            let operator = self.parse_logical_operator()?;
            let right = self.parse_comparison_as_filter_expression()?;
            
            left = FilterExpression::Logical(LogicalExpression {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            });
        }
        
        // Extract the logical expression if that's what we have
        if let FilterExpression::Logical(logical) = left {
            Ok(logical)
        } else {
            // Convert single comparison to logical expression
            Ok(LogicalExpression {
                left: Box::new(left.clone()),
                operator: LogicalOperator::And, // Default operator
                right: Box::new(left),
            })
        }
    }
    
    /// Parse comparison expression
    /// comparison_expression = field_reference comparison_operator literal_value
    fn parse_comparison_expression(&mut self) -> ParseResult<ComparisonExpression> {
        let field = self.parse_field_reference()?;
        let operator = self.parse_comparison_operator()?;
        let value = self.parse_literal_value()?;
        
        Ok(ComparisonExpression {
            field: field.name,
            operator,
            value,
        })
    }
    
    /// Parse literal value
    /// literal_value = string | number | boolean | datetime
    fn parse_literal_value(&mut self) -> ParseResult<LiteralValue> {
        let current = self.state.peek();
        
        match &current.token_type {
            TokenType::String(_) => {
                let token = self.state.advance();
                let value = self.value_parser.parse_string_value(&token.lexeme);
                Ok(LiteralValue::String(value))
            }
            TokenType::Number(_) => {
                let token = self.state.advance();
                let value = self.value_parser.parse_number_value(&token.lexeme)?;
                Ok(LiteralValue::Number(value))
            }
            TokenType::Identifier(_) => {
                let token = self.state.advance();
                match token.lexeme.to_lowercase().as_str() {
                    "true" => Ok(LiteralValue::Boolean(true)),
                    "false" => Ok(LiteralValue::Boolean(false)),
                    _ => {
                        // Try to parse as datetime
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&token.lexeme) {
                            Ok(LiteralValue::DateTime(dt.with_timezone(&chrono::Utc)))
                        } else {
                            // Treat as string
                            Ok(LiteralValue::String(token.lexeme.clone()))
                        }
                    }
                }
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "literal value (string, number, boolean, or datetime)".to_string(),
                found: current.lexeme.clone(),
                line: current.line,
                column: current.column,
            }),
        }
    }
    
    /// Parse field reference
    /// field_reference = identifier | identifier DOT identifier
    fn parse_field_reference(&mut self) -> ParseResult<FieldReference> {
        let first_token = self.state.consume(TokenType::Identifier, "Expected field name")?;
        let mut name = first_token.lexeme.clone();
        let mut qualified = false;
        
        // Check for qualified field (table.field)
        if self.state.match_token(&TokenType::Dot) {
            qualified = true;
            let field_token = self.state.consume(TokenType::Identifier, "Expected field name after '.'")?;
            name = format!("{}.{}", name, field_token.lexeme);
        }
        
        Ok(FieldReference { name, qualified })
    }
}

impl<'a> ExpressionRuleParser<'a> {
    /// Check if current token is a logical operator
    fn is_logical_operator(&self) -> bool {
        matches!(self.state.peek().token_type, TokenType::And | TokenType::Or)
    }
    
    /// Parse logical operator
    fn parse_logical_operator(&mut self) -> ParseResult<LogicalOperator> {
        match &self.state.peek().token_type {
            TokenType::And => {
                self.state.advance();
                Ok(LogicalOperator::And)
            }
            TokenType::Or => {
                self.state.advance();
                Ok(LogicalOperator::Or)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "logical operator (AND, OR)".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            }),
        }
    }
    
    /// Parse comparison operator
    fn parse_comparison_operator(&mut self) -> ParseResult<ComparisonOperator> {
        let current = self.state.peek();
        
        let operator = match &current.token_type {
            TokenType::Equal => ComparisonOperator::Equal,
            TokenType::NotEqual => ComparisonOperator::NotEqual,
            TokenType::Greater => ComparisonOperator::Greater,
            TokenType::GreaterEqual => ComparisonOperator::GreaterEqual,
            TokenType::Less => ComparisonOperator::Less,
            TokenType::LessEqual => ComparisonOperator::LessEqual,
            TokenType::Like => ComparisonOperator::Like,
            _ => return Err(ParseError::UnexpectedToken {
                expected: "comparison operator (=, !=, >, >=, <, <=, LIKE)".to_string(),
                found: current.lexeme.clone(),
                line: current.line,
                column: current.column,
            }),
        };
        
        self.state.advance();
        Ok(operator)
    }
    
    /// Parse comparison expression as filter expression
    fn parse_comparison_as_filter_expression(&mut self) -> ParseResult<FilterExpression> {
        let comparison = self.parse_comparison_expression()?;
        Ok(FilterExpression::Comparison(comparison))
    }
}

/// Utility functions for expression evaluation
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate a literal value to string for comparison
    pub fn literal_to_string(literal: &LiteralValue) -> String {
        match literal {
            LiteralValue::String(s) => s.clone(),
            LiteralValue::Number(n) => n.to_string(),
            LiteralValue::Boolean(b) => b.to_string(),
            LiteralValue::DateTime(dt) => dt.to_rfc3339(),
        }
    }
    
    /// Check if two literal values are equal
    pub fn literals_equal(left: &LiteralValue, right: &LiteralValue) -> bool {
        match (left, right) {
            (LiteralValue::String(a), LiteralValue::String(b)) => a == b,
            (LiteralValue::Number(a), LiteralValue::Number(b)) => (a - b).abs() < f64::EPSILON,
            (LiteralValue::Boolean(a), LiteralValue::Boolean(b)) => a == b,
            (LiteralValue::DateTime(a), LiteralValue::DateTime(b)) => a == b,
            _ => false,
        }
    }
    
    /// Compare two literal values
    pub fn compare_literals(left: &LiteralValue, right: &LiteralValue) -> Option<std::cmp::Ordering> {
        match (left, right) {
            (LiteralValue::Number(a), LiteralValue::Number(b)) => a.partial_cmp(b),
            (LiteralValue::String(a), LiteralValue::String(b)) => Some(a.cmp(b)),
            (LiteralValue::DateTime(a), LiteralValue::DateTime(b)) => Some(a.cmp(b)),
            (LiteralValue::Boolean(a), LiteralValue::Boolean(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::ParserState;
    use super::super::super::super::lexer::Lexer;

    fn create_expression_parser(input: &str) -> (ParserState, super::super::super::types::ParsingContext) {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let state = ParserState::new(tokens);
        let context = super::super::super::types::ParsingContext::new();
        (state, context)
    }

    #[test]
    fn test_field_reference() {
        let (mut state, mut context) = create_expression_parser("severity");
        let mut parser = ExpressionRuleParser::new(&mut state, &mut context);
        
        let field = parser.parse_field_reference().unwrap();
        assert_eq!(field.name, "severity");
        assert!(!field.qualified);
    }

    #[test]
    fn test_qualified_field_reference() {
        let (mut state, mut context) = create_expression_parser("diagnostics.severity");
        let mut parser = ExpressionRuleParser::new(&mut state, &mut context);
        
        let field = parser.parse_field_reference().unwrap();
        assert_eq!(field.name, "diagnostics.severity");
        assert!(field.qualified);
    }

    #[test]
    fn test_string_literal() {
        let (mut state, mut context) = create_expression_parser("'error'");
        let mut parser = ExpressionRuleParser::new(&mut state, &mut context);
        
        let literal = parser.parse_literal_value().unwrap();
        if let LiteralValue::String(s) = literal {
            assert_eq!(s, "error");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_number_literal() {
        let (mut state, mut context) = create_expression_parser("42.5");
        let mut parser = ExpressionRuleParser::new(&mut state, &mut context);
        
        let literal = parser.parse_literal_value().unwrap();
        if let LiteralValue::Number(n) = literal {
            assert_eq!(n, 42.5);
        } else {
            panic!("Expected number literal");
        }
    }

    #[test]
    fn test_boolean_literal() {
        let (mut state, mut context) = create_expression_parser("true");
        let mut parser = ExpressionRuleParser::new(&mut state, &mut context);
        
        let literal = parser.parse_literal_value().unwrap();
        if let LiteralValue::Boolean(b) = literal {
            assert!(b);
        } else {
            panic!("Expected boolean literal");
        }
    }

    #[test]
    fn test_comparison_expression() {
        let (mut state, mut context) = create_expression_parser("severity = 'error'");
        let mut parser = ExpressionRuleParser::new(&mut state, &mut context);
        
        let comparison = parser.parse_comparison_expression().unwrap();
        assert_eq!(comparison.field, "severity");
        assert_eq!(comparison.operator, ComparisonOperator::Equal);
        
        if let LiteralValue::String(s) = comparison.value {
            assert_eq!(s, "error");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_expression_evaluator() {
        let str_lit = LiteralValue::String("test".to_string());
        let num_lit = LiteralValue::Number(42.0);
        
        assert_eq!(ExpressionEvaluator::literal_to_string(&str_lit), "test");
        assert_eq!(ExpressionEvaluator::literal_to_string(&num_lit), "42");
        
        assert!(ExpressionEvaluator::literals_equal(&str_lit, &LiteralValue::String("test".to_string())));
        assert!(!ExpressionEvaluator::literals_equal(&str_lit, &num_lit));
    }
}