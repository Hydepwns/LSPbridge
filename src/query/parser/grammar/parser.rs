//! Main parsing engine implementation

use super::types::{ParserState, ParsingContext, ProductionRule, ParseResult, ValueParser, DefaultValueParser};
use super::utilities::ParserUtilities;
use super::super::ast::*;
use super::super::lexer::{Token, TokenType};
use crate::core::errors::ParseError;
use crate::core::DiagnosticSeverity;
use chrono::{DateTime, Utc};
use std::str::FromStr;

/// Recursive descent parser for the query language
pub struct Parser {
    state: ParserState,
    context: ParsingContext,
    value_parser: Box<dyn ValueParser>,
    #[allow(dead_code)]
    utilities: ParserUtilities,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            state: ParserState::new(tokens),
            context: ParsingContext::new(),
            value_parser: Box::new(DefaultValueParser),
            utilities: ParserUtilities::new(),
        }
    }

    /// Create a parser with custom value parser
    pub fn with_value_parser(tokens: Vec<Token>, value_parser: Box<dyn ValueParser>) -> Self {
        Self {
            state: ParserState::new(tokens),
            context: ParsingContext::new(),
            value_parser,
            utilities: ParserUtilities::new(),
        }
    }

    /// Parse the tokens into a Query AST
    pub fn parse(&mut self) -> ParseResult<Query> {
        self.context.enter_rule(ProductionRule::Query);
        let result = self.parse_query();
        self.context.exit_rule();
        result
    }

    /// Parse a complete query
    fn parse_query(&mut self) -> ParseResult<Query> {
        let select = self.parse_select_clause()?;
        let from = self.parse_from_clause()?;
        
        let mut filters = Vec::new();
        let mut time_range = None;
        
        // Optional WHERE clause
        if self.state.match_token(&TokenType::Where) {
            let (parsed_filters, parsed_time_range) = self.parse_where_clause()?;
            filters = parsed_filters;
            time_range = parsed_time_range;
        }
        
        // Optional GROUP BY clause
        let group_by = if self.state.match_token(&TokenType::Group) {
            Some(self.parse_group_by_clause()?)
        } else {
            None
        };
        
        // Optional ORDER BY clause
        let order_by = if self.state.match_token(&TokenType::Order) {
            Some(self.parse_order_by_clause()?)
        } else {
            None
        };
        
        // Optional LIMIT clause
        let limit = if self.state.match_token(&TokenType::Limit) {
            Some(self.parse_limit_clause()?)
        } else {
            None
        };

        let query = Query {
            select,
            from,
            filters,
            time_range,
            group_by,
            order_by,
            limit,
        };

        // Validate the parsed query
        super::types::GrammarValidator::validate_query(&query)?;

        Ok(query)
    }

    /// Parse SELECT clause
    fn parse_select_clause(&mut self) -> ParseResult<SelectClause> {
        self.context.enter_rule(ProductionRule::SelectClause);
        self.context.expect_token(TokenType::Select);
        
        self.state.consume(TokenType::Select, "Expected 'SELECT'")?;
        
        let result = if self.state.match_token(&TokenType::Asterisk) {
            SelectClause::All
        } else if self.state.check(&TokenType::Count) {
            self.state.advance(); // consume COUNT
            self.state.consume(TokenType::LeftParen, "Expected '(' after COUNT")?;
            self.state.consume(TokenType::Asterisk, "Expected '*' in COUNT(*)")?;
            self.state.consume(TokenType::RightParen, "Expected ')' after COUNT(*)")?;
            SelectClause::Count
        } else if self.state.check_identifier() || 
                  self.state.check(&TokenType::Errors) ||
                  self.state.check(&TokenType::Warnings) ||
                  self.state.check(&TokenType::Files) ||
                  self.state.check(&TokenType::Diagnostics) ||
                  self.state.check(&TokenType::History) ||
                  self.state.check(&TokenType::Trends) {
            let fields = self.parse_field_list()?;
            SelectClause::Fields(fields)
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "*, COUNT(*), or field list".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            });
        };
        
        self.context.exit_rule();
        Ok(result)
    }

    /// Parse FROM clause
    fn parse_from_clause(&mut self) -> ParseResult<FromClause> {
        self.context.enter_rule(ProductionRule::FromClause);
        self.context.expect_token(TokenType::From);
        
        self.state.consume(TokenType::From, "Expected 'FROM'")?;
        
        // Check for table name - can be a keyword token or identifier
        let result = if self.state.check(&TokenType::Diagnostics) {
            self.state.advance();
            FromClause::Diagnostics
        } else if self.state.check(&TokenType::Files) {
            self.state.advance();
            FromClause::Files
        } else if self.state.check(&TokenType::History) {
            self.state.advance();
            FromClause::History
        } else if self.state.check(&TokenType::Trends) {
            self.state.advance();
            FromClause::Trends
        } else if self.state.check_identifier() {
            let token = self.state.advance();
            match token.lexeme.as_str() {
                "diagnostics" => FromClause::Diagnostics,
                "files" => FromClause::Files,
                "symbols" => FromClause::Symbols,
                "references" => FromClause::References,
                "projects" => FromClause::Projects,
                "history" => FromClause::History,
                "trends" => FromClause::Trends,
                _ => return Err(ParseError::UnknownTable {
                    table: token.lexeme.clone(),
                    line: token.line,
                    column: token.column,
                }),
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "table name".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            });
        };
        
        self.context.exit_rule();
        Ok(result)
    }

    /// Parse WHERE clause
    fn parse_where_clause(&mut self) -> ParseResult<(Vec<QueryFilter>, Option<TimeRange>)> {
        self.context.enter_rule(ProductionRule::WhereClause);
        
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
        
        self.context.exit_rule();
        Ok((filters, time_range))
    }

    /// Parse filter expression
    fn parse_filter_expression(&mut self) -> ParseResult<QueryFilter> {
        self.context.enter_rule(ProductionRule::FilterExpression);
        
        let result = if self.state.check(&TokenType::Last) {
            self.parse_relative_time_filter()
        } else if self.state.check_identifier() {
            let field = self.state.advance().lexeme.clone();
            
            match field.as_str() {
                "severity" => self.parse_severity_filter(),
                "file" => self.parse_file_filter(),
                "symbol" => self.parse_symbol_filter(),
                "since" | "before" | "after" => self.parse_time_filter(field),
                _ => self.parse_custom_filter(field),
            }
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "filter expression".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            })
        };
        
        self.context.exit_rule();
        result
    }

    /// Parse GROUP BY clause
    fn parse_group_by_clause(&mut self) -> ParseResult<GroupByClause> {
        self.context.enter_rule(ProductionRule::GroupByClause);
        
        // Consume "BY" (already consumed "GROUP")
        self.state.consume(TokenType::By, "Expected 'BY' after 'GROUP'")?;
        
        let fields = self.parse_field_list()?;
        
        self.context.exit_rule();
        Ok(GroupByClause { fields })
    }

    /// Parse ORDER BY clause
    fn parse_order_by_clause(&mut self) -> ParseResult<OrderByClause> {
        self.context.enter_rule(ProductionRule::OrderByClause);
        
        // Consume "BY" (already consumed "ORDER")
        self.state.consume(TokenType::By, "Expected 'BY' after 'ORDER'")?;
        
        // Parse field or aggregation function
        let field = if self.state.check(&TokenType::Count) || 
                       self.state.check(&TokenType::Sum) ||
                       self.state.check(&TokenType::Avg) ||
                       self.state.check(&TokenType::Min) ||
                       self.state.check(&TokenType::Max) {
            let func = self.state.advance().lexeme.clone();
            // Handle COUNT(*) and other aggregation functions
            if self.state.check(&TokenType::LeftParen) {
                self.state.advance();
                let arg = if self.state.check(&TokenType::Asterisk) {
                    self.state.advance();
                    "*".to_string()
                } else if self.state.check_identifier() {
                    self.state.advance().lexeme.clone()
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "field name or *".to_string(),
                        found: self.state.peek().lexeme.clone(),
                        line: self.state.peek().line,
                        column: self.state.peek().column,
                    });
                };
                self.state.consume(TokenType::RightParen, "Expected ')' after aggregation function")?;
                format!("{func}({arg})")
            } else {
                func
            }
        } else if self.state.check_identifier() || 
                  self.state.check(&TokenType::Errors) ||
                  self.state.check(&TokenType::Warnings) ||
                  self.state.check(&TokenType::Files) ||
                  self.state.check(&TokenType::Diagnostics) ||
                  self.state.check(&TokenType::History) ||
                  self.state.check(&TokenType::Trends) {
            self.state.advance().lexeme.clone()
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "field name".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            });
        };
        
        let direction = if self.state.match_token(&TokenType::Desc) {
            OrderDirection::Descending
        } else {
            self.state.match_token(&TokenType::Asc); // Optional ASC
            OrderDirection::Ascending
        };
        
        self.context.exit_rule();
        Ok(OrderByClause { field, direction })
    }

    /// Parse LIMIT clause
    fn parse_limit_clause(&mut self) -> ParseResult<u32> {
        self.context.enter_rule(ProductionRule::LimitClause);
        
        if !self.state.check_number() {
            return Err(ParseError::UnexpectedToken {
                expected: "number after LIMIT".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            });
        }
        let token = self.state.advance();
        let value = self.value_parser.parse_number_value(&token.lexeme)? as u32;
        
        // Validate limit value
        if value == 0 {
            return Err(ParseError::InvalidLimit {
                limit: value,
                reason: "LIMIT must be greater than 0".to_string(),
            });
        }
        
        self.context.exit_rule();
        Ok(value)
    }

    /// Parse field list
    fn parse_field_list(&mut self) -> ParseResult<Vec<String>> {
        let mut fields = Vec::new();
        
        loop {
            // Check for aggregation functions first
            let field = if self.state.check(&TokenType::Count) || 
                          self.state.check(&TokenType::Sum) ||
                          self.state.check(&TokenType::Avg) ||
                          self.state.check(&TokenType::Min) ||
                          self.state.check(&TokenType::Max) {
                let func = self.state.advance().lexeme.clone();
                // Handle COUNT(*) and other aggregation functions
                if self.state.check(&TokenType::LeftParen) {
                    self.state.advance();
                    let arg = if self.state.check(&TokenType::Asterisk) {
                        self.state.advance();
                        "*".to_string()
                    } else if self.state.check_identifier() {
                        self.state.advance().lexeme.clone()
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: "field name or *".to_string(),
                            found: self.state.peek().lexeme.clone(),
                            line: self.state.peek().line,
                            column: self.state.peek().column,
                        });
                    };
                    self.state.consume(TokenType::RightParen, "Expected ')' after aggregation function")?;
                    format!("{func}({arg})")
                } else {
                    func
                }
            } else if self.state.check_identifier() {
                self.state.advance().lexeme.clone()
            } else if self.state.check(&TokenType::Errors) ||
                      self.state.check(&TokenType::Warnings) ||
                      self.state.check(&TokenType::Files) ||
                      self.state.check(&TokenType::Diagnostics) ||
                      self.state.check(&TokenType::History) ||
                      self.state.check(&TokenType::Trends) {
                self.state.advance().lexeme.clone()
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "field name".to_string(),
                    found: self.state.peek().lexeme.clone(),
                    line: self.state.peek().line,
                    column: self.state.peek().column,
                });
            };
            fields.push(field);
            
            if self.state.match_token(&TokenType::Comma) {
                continue;
            } else {
                break;
            }
        }
        
        Ok(fields)
    }

    /// Parse severity filter
    fn parse_severity_filter(&mut self) -> ParseResult<QueryFilter> {
        let comparison = self.parse_comparison_operator()?;
        let value = self.parse_string_or_identifier()?;
        
        let severity = match value.to_lowercase().as_str() {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "info" => DiagnosticSeverity::Information,
            "hint" => DiagnosticSeverity::Hint,
            _ => return Err(ParseError::InvalidSeverity {
                severity: value,
                line: self.state.previous().line,
                column: self.state.previous().column,
            }),
        };
        
        Ok(QueryFilter::Severity(SeverityFilter { severity, comparison }))
    }

    /// Parse file filter
    fn parse_file_filter(&mut self) -> ParseResult<QueryFilter> {
        let _comparison = self.parse_comparison_operator()?; // File filter doesn't use comparison
        let pattern = self.parse_string_or_identifier()?;
        Ok(QueryFilter::File(FileFilter { pattern }))
    }

    /// Parse symbol filter
    fn parse_symbol_filter(&mut self) -> ParseResult<QueryFilter> {
        let _comparison = self.parse_comparison_operator()?; // Symbol filter doesn't use comparison
        let pattern = self.parse_string_or_identifier()?;
        Ok(QueryFilter::Symbol(SymbolFilter { pattern }))
    }

    /// Parse time filter
    fn parse_time_filter(&mut self, field: String) -> ParseResult<QueryFilter> {
        let _comparison = self.parse_comparison_operator()?; // Time filter doesn't use comparison
        let value = self.parse_string_or_identifier()?;
        
        // Parse the datetime
        let datetime = DateTime::<Utc>::from_str(&value)
            .map_err(|_| ParseError::InvalidDateTime {
                value: value.clone(),
                line: self.state.previous().line,
                column: self.state.previous().column,
            })?;
        
        let time_range = match field.as_str() {
            "since" => TimeRange::since(datetime),
            "before" => TimeRange::before(datetime),
            "after" => TimeRange::after(datetime),
            _ => unreachable!(),
        };
        
        Ok(QueryFilter::TimeRange(time_range))
    }

    /// Parse custom filter
    fn parse_custom_filter(&mut self, field: String) -> ParseResult<QueryFilter> {
        let _comparison = self.parse_comparison_operator()?; // Custom filter doesn't use comparison
        let value = self.parse_string_or_identifier()?;
        Ok(QueryFilter::Custom(field, value))
    }

    /// Parse relative time filter
    fn parse_relative_time_filter(&mut self) -> ParseResult<QueryFilter> {
        self.state.consume(TokenType::Last, "Expected 'LAST'")?;
        let value = self.parse_number_value()? as u32;
        
        let relative_time = match &self.state.peek().token_type {
            TokenType::Hours => {
                self.state.advance();
                RelativeTime::LastHours(value)
            }
            TokenType::Days => {
                self.state.advance();
                RelativeTime::LastDays(value)
            }
            TokenType::Weeks => {
                self.state.advance();
                RelativeTime::LastWeeks(value)
            }
            _ => return Err(ParseError::UnexpectedToken {
                expected: "time unit (hours, days, weeks)".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            }),
        };

        Ok(QueryFilter::TimeRange(TimeRange::relative(relative_time)))
    }

    /// Parse comparison operator
    fn parse_comparison_operator(&mut self) -> ParseResult<Comparison> {
        self.context.enter_rule(ProductionRule::ComparisonOperator);
        
        let comparison = if self.state.match_token(&TokenType::Equal) {
            Comparison::Equal
        } else if self.state.match_token(&TokenType::NotEqual) {
            Comparison::NotEqual
        } else if self.state.match_token(&TokenType::Like) {
            Comparison::Equal // Like is treated as Equal for severity
        } else if self.state.match_token(&TokenType::GreaterThan) {
            Comparison::GreaterThan
        } else if self.state.match_token(&TokenType::LessThan) {
            Comparison::LessThan
        } else if self.state.match_token(&TokenType::GreaterThanOrEqual) {
            Comparison::GreaterThanOrEqual
        } else if self.state.match_token(&TokenType::LessThanOrEqual) {
            Comparison::LessThanOrEqual
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "comparison operator".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            });
        };
        
        self.context.exit_rule();
        Ok(comparison)
    }

    /// Parse string or identifier value
    fn parse_string_or_identifier(&mut self) -> ParseResult<String> {
        if self.state.check_string() {
            let token = self.state.advance();
            Ok(self.value_parser.parse_string_value(&token.lexeme))
        } else if self.state.check_identifier() {
            let token = self.state.advance();
            Ok(token.lexeme.clone())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "string or identifier".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            })
        }
    }

    /// Parse number value
    fn parse_number_value(&mut self) -> ParseResult<f64> {
        if !self.state.check_number() {
            return Err(ParseError::UnexpectedToken {
                expected: "number".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            });
        }
        let token = self.state.advance();
        self.value_parser.parse_number_value(&token.lexeme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::lexer::Lexer;

    fn parse_query(input: &str) -> ParseResult<Query> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_simple_select_all() {
        let query = parse_query("SELECT * FROM diagnostics").unwrap();
        assert_eq!(query.select, SelectClause::All);
        assert_eq!(query.from, FromClause::Diagnostics);
        assert!(query.filters.is_empty());
    }

    #[test]
    fn test_select_count() {
        let query = parse_query("SELECT COUNT(*) FROM files").unwrap();
        assert_eq!(query.select, SelectClause::Count);
        assert_eq!(query.from, FromClause::Files);
    }

    #[test]
    fn test_select_with_filter() {
        let query = parse_query("SELECT * FROM diagnostics WHERE severity = 'error'").unwrap();
        assert_eq!(query.select, SelectClause::All);
        assert_eq!(query.from, FromClause::Diagnostics);
        assert_eq!(query.filters.len(), 1);
        
        if let QueryFilter::Severity(filter) = &query.filters[0] {
            assert_eq!(filter.severity, DiagnosticSeverity::Error);
        } else {
            panic!("Expected severity filter");
        }
    }

    #[test]
    fn test_relative_time_filter() {
        let query = parse_query("SELECT * FROM diagnostics WHERE LAST 7 DAYS").unwrap();
        assert!(query.time_range.is_some());
        
        if let Some(TimeRange { relative: Some(RelativeTime::LastDays(7)), .. }) = query.time_range {
            // Success
        } else {
            panic!("Expected relative time range");
        }
    }

    #[test]
    fn test_order_by_and_limit() {
        let query = parse_query("SELECT * FROM diagnostics ORDER BY severity DESC LIMIT 10").unwrap();
        assert!(query.order_by.is_some());
        assert_eq!(query.limit, Some(10));
        
        if let Some(order_by) = query.order_by {
            assert_eq!(order_by.field, "severity");
            assert_eq!(order_by.direction, OrderDirection::Descending);
        }
    }

    #[test]
    fn test_field_list_parsing() {
        let query = parse_query("SELECT file, line, message FROM diagnostics").unwrap();
        
        if let SelectClause::Fields(fields) = query.select {
            assert_eq!(fields, vec!["file", "line", "message"]);
        } else {
            panic!("Expected field list");
        }
    }

    #[test]
    fn test_error_handling() {
        assert!(parse_query("SELECT").is_err());
        assert!(parse_query("SELECT * FROM").is_err());
        assert!(parse_query("SELECT * FROM unknown_table").is_err());
    }
}