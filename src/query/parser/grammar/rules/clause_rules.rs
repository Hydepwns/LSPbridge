//! Clause-level grammar rules (SELECT, FROM, WHERE, etc.)

use super::super::types::{ParserState, ParsingContext, ParseResult};
use super::super::super::ast::*;
use super::super::super::lexer::TokenType;
use crate::core::errors::ParseError;

/// Trait for parsing clause-level grammar rules
pub trait ClauseRules {
    /// Parse SELECT clause
    fn parse_select_clause(&mut self) -> ParseResult<SelectClause>;
    
    /// Parse FROM clause  
    fn parse_from_clause(&mut self) -> ParseResult<FromClause>;
    
    /// Parse WHERE clause
    fn parse_where_clause(&mut self) -> ParseResult<(Vec<QueryFilter>, Option<TimeRange>)>;
    
    /// Parse GROUP BY clause
    fn parse_group_by_clause(&mut self) -> ParseResult<GroupByClause>;
    
    /// Parse ORDER BY clause
    fn parse_order_by_clause(&mut self) -> ParseResult<OrderByClause>;
    
    /// Parse LIMIT clause
    fn parse_limit_clause(&mut self) -> ParseResult<u32>;
}

/// Implementation of clause-level parsing rules
pub struct ClauseRuleParser<'a> {
    pub state: &'a mut ParserState,
    pub context: &'a mut ParsingContext,
}

impl<'a> ClauseRuleParser<'a> {
    /// Create a new clause rule parser
    pub fn new(state: &'a mut ParserState, context: &'a mut ParsingContext) -> Self {
        Self { state, context }
    }
}

impl<'a> ClauseRules for ClauseRuleParser<'a> {
    /// Parse SELECT clause
    /// select_clause = SELECT (ASTERISK | COUNT LPAREN ASTERISK RPAREN | field_list)
    fn parse_select_clause(&mut self) -> ParseResult<SelectClause> {
        self.state.consume(TokenType::Select, "Expected 'SELECT'")?;
        
        if self.state.match_token(&TokenType::Asterisk) {
            Ok(SelectClause::All)
        } else if self.state.check(&TokenType::Count) {
            self.state.advance(); // consume COUNT
            self.state.consume(TokenType::LeftParen, "Expected '(' after COUNT")?;
            self.state.consume(TokenType::Asterisk, "Expected '*' in COUNT(*)")?;
            self.state.consume(TokenType::RightParen, "Expected ')' after COUNT(*)")?;
            Ok(SelectClause::Count)
        } else if self.state.check_identifier() {
            let fields = self.parse_field_list()?;
            Ok(SelectClause::Fields(fields))
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "*, COUNT(*), or field list".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            })
        }
    }
    
    /// Parse FROM clause
    /// from_clause = FROM table_name
    fn parse_from_clause(&mut self) -> ParseResult<FromClause> {
        self.state.consume(TokenType::From, "Expected 'FROM'")?;
        
        let token = self.state.consume(TokenType::Identifier, "Expected table name")?;
        match token.lexeme.as_str() {
            "diagnostics" => Ok(FromClause::Diagnostics),
            "files" => Ok(FromClause::Files),
            "symbols" => Ok(FromClause::Symbols),
            "references" => Ok(FromClause::References),
            "projects" => Ok(FromClause::Projects),
            _ => Err(ParseError::UnknownTable {
                table: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            }),
        }
    }
    
    /// Parse WHERE clause
    /// where_clause = WHERE filter_expression (logical_operator filter_expression)*
    fn parse_where_clause(&mut self) -> ParseResult<(Vec<QueryFilter>, Option<TimeRange>)> {
        // WHERE token already consumed by caller
        
        let mut filters = Vec::new();
        let mut time_range = None;
        
        // Parse filters with AND/OR operators
        loop {
            let filter = self.parse_filter_expression()?;
            
            // Check if this is a time range filter
            if let QueryFilter::TimeRange(ref tr) = filter {
                time_range = Some(tr.clone());
            } else {
                filters.push(filter);
            }
            
            // Check for logical operators
            if self.state.match_token(&TokenType::And) || self.state.match_token(&TokenType::Or) {
                continue;
            } else {
                break;
            }
        }
        
        Ok((filters, time_range))
    }
    
    /// Parse GROUP BY clause
    /// group_by_clause = GROUP BY field_list
    fn parse_group_by_clause(&mut self) -> ParseResult<GroupByClause> {
        // GROUP token already consumed by caller
        // Skip "BY" if present
        if self.state.check(&TokenType::GroupBy) {
            self.state.advance();
        }
        
        let fields = self.parse_field_list()?;
        
        if fields.is_empty() {
            return Err(ParseError::EmptyGroupBy);
        }
        
        Ok(GroupByClause { fields })
    }
    
    /// Parse ORDER BY clause
    /// order_by_clause = ORDER BY field (ASC | DESC)?
    fn parse_order_by_clause(&mut self) -> ParseResult<OrderByClause> {
        // ORDER token already consumed by caller
        // Skip "BY" if present
        if self.state.check(&TokenType::OrderBy) {
            self.state.advance();
        }
        
        let field = self.state.consume(TokenType::Identifier, "Expected field name")?.lexeme.clone();
        
        if field.is_empty() {
            return Err(ParseError::EmptyOrderBy);
        }
        
        let direction = if self.state.match_token(&TokenType::Desc) {
            OrderDirection::Descending
        } else {
            self.state.match_token(&TokenType::Asc); // Optional ASC
            OrderDirection::Ascending
        };
        
        Ok(OrderByClause { field, direction })
    }
    
    /// Parse LIMIT clause
    /// limit_clause = LIMIT number
    fn parse_limit_clause(&mut self) -> ParseResult<u32> {
        // LIMIT token already consumed by caller
        
        let token = self.state.consume(TokenType::Number, "Expected number after LIMIT")?;
        let value = token.lexeme.parse::<u32>()
            .map_err(|_| ParseError::InvalidNumber {
                value: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            })?;
        
        if value == 0 {
            return Err(ParseError::InvalidLimit {
                value,
                reason: "LIMIT must be greater than 0".to_string(),
            });
        }
        
        Ok(value)
    }
}

impl<'a> ClauseRuleParser<'a> {
    /// Parse a comma-separated list of field names
    fn parse_field_list(&mut self) -> ParseResult<Vec<String>> {
        let mut fields = Vec::new();
        
        loop {
            let field = self.state.consume(TokenType::Identifier, "Expected field name")?.lexeme.clone();
            fields.push(field);
            
            if self.state.match_token(&TokenType::Comma) {
                continue;
            } else {
                break;
            }
        }
        
        Ok(fields)
    }
    
    /// Parse a single filter expression (delegates to FilterRules)
    fn parse_filter_expression(&mut self) -> ParseResult<QueryFilter> {
        // This would be implemented by delegating to FilterRules
        // For now, we'll provide a minimal implementation
        if self.state.check_identifier() {
            let field = self.state.advance().lexeme.clone();
            
            // Consume comparison operator
            if !self.state.match_token(&TokenType::Equal) {
                return Err(ParseError::UnexpectedToken {
                    expected: "comparison operator".to_string(),
                    found: self.state.peek().lexeme.clone(),
                    line: self.state.peek().line,
                    column: self.state.peek().column,
                });
            }
            
            // Parse value
            let value = if self.state.check_string() {
                let token = self.state.advance();
                // Remove quotes
                let mut value = token.lexeme.clone();
                if value.starts_with('"') && value.ends_with('"') {
                    value = value[1..value.len()-1].to_string();
                }
                value
            } else if self.state.check_identifier() {
                self.state.advance().lexeme.clone()
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "string or identifier".to_string(),
                    found: self.state.peek().lexeme.clone(),
                    line: self.state.peek().line,
                    column: self.state.peek().column,
                });
            };
            
            Ok(QueryFilter::Custom(field, value))
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "filter expression".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::ParserState;
    use super::super::super::super::lexer::Lexer;

    fn create_parser_with_input(input: &str) -> (ParserState, super::super::super::types::ParsingContext) {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let state = ParserState::new(tokens);
        let context = super::super::super::types::ParsingContext::new();
        (state, context)
    }

    #[test]
    fn test_select_all_clause() {
        let (mut state, mut context) = create_parser_with_input("SELECT *");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let select = parser.parse_select_clause().unwrap();
        assert_eq!(select, SelectClause::All);
    }

    #[test]
    fn test_select_count_clause() {
        let (mut state, mut context) = create_parser_with_input("SELECT COUNT(*)");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let select = parser.parse_select_clause().unwrap();
        assert_eq!(select, SelectClause::Count);
    }

    #[test]
    fn test_select_fields_clause() {
        let (mut state, mut context) = create_parser_with_input("SELECT file, line, message");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let select = parser.parse_select_clause().unwrap();
        if let SelectClause::Fields(fields) = select {
            assert_eq!(fields, vec!["file", "line", "message"]);
        } else {
            panic!("Expected field list");
        }
    }

    #[test]
    fn test_from_clause() {
        let (mut state, mut context) = create_parser_with_input("FROM diagnostics");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let from = parser.parse_from_clause().unwrap();
        assert_eq!(from, FromClause::Diagnostics);
    }

    #[test]
    fn test_unknown_table() {
        let (mut state, mut context) = create_parser_with_input("FROM unknown_table");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        assert!(parser.parse_from_clause().is_err());
    }

    #[test]
    fn test_group_by_clause() {
        let (mut state, mut context) = create_parser_with_input("BY file, severity");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let group_by = parser.parse_group_by_clause().unwrap();
        assert_eq!(group_by.fields, vec!["file", "severity"]);
    }

    #[test]
    fn test_order_by_clause() {
        let (mut state, mut context) = create_parser_with_input("BY severity DESC");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let order_by = parser.parse_order_by_clause().unwrap();
        assert_eq!(order_by.field, "severity");
        assert_eq!(order_by.direction, OrderDirection::Descending);
    }

    #[test]
    fn test_limit_clause() {
        let (mut state, mut context) = create_parser_with_input("10");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        let limit = parser.parse_limit_clause().unwrap();
        assert_eq!(limit, 10);
    }

    #[test]
    fn test_invalid_limit() {
        let (mut state, mut context) = create_parser_with_input("0");
        let mut parser = ClauseRuleParser::new(&mut state, &mut context);
        
        assert!(parser.parse_limit_clause().is_err());
    }
}