//! Filter-specific grammar rules

use super::super::types::{ParserState, ParsingContext, ParseResult, ValueParser, DefaultValueParser};
use super::super::super::ast::*;
use super::super::super::lexer::TokenType;
use crate::core::errors::ParseError;
use crate::core::DiagnosticSeverity;
use chrono::{DateTime, Utc};
use std::str::FromStr;

/// Trait for parsing filter expressions
pub trait FilterRules {
    /// Parse any filter expression
    fn parse_filter_expression(&mut self) -> ParseResult<QueryFilter>;
    
    /// Parse severity filter
    fn parse_severity_filter(&mut self) -> ParseResult<QueryFilter>;
    
    /// Parse file filter  
    fn parse_file_filter(&mut self) -> ParseResult<QueryFilter>;
    
    /// Parse symbol filter
    fn parse_symbol_filter(&mut self) -> ParseResult<QueryFilter>;
    
    /// Parse time filter
    fn parse_time_filter(&mut self, field: String) -> ParseResult<QueryFilter>;
    
    /// Parse relative time filter
    fn parse_relative_time_filter(&mut self) -> ParseResult<QueryFilter>;
    
    /// Parse custom filter
    fn parse_custom_filter(&mut self, field: String) -> ParseResult<QueryFilter>;
}

/// Implementation of filter parsing rules
pub struct FilterRuleParser<'a> {
    pub state: &'a mut ParserState,
    pub context: &'a mut ParsingContext,
    pub value_parser: Box<dyn ValueParser>,
}

impl<'a> FilterRuleParser<'a> {
    /// Create a new filter rule parser
    pub fn new(state: &'a mut ParserState, context: &'a mut ParsingContext) -> Self {
        Self { 
            state, 
            context, 
            value_parser: Box::new(DefaultValueParser)
        }
    }
    
    /// Create a filter rule parser with custom value parser
    pub fn with_value_parser(
        state: &'a mut ParserState, 
        context: &'a mut ParsingContext,
        value_parser: Box<dyn ValueParser>
    ) -> Self {
        Self { state, context, value_parser }
    }
}

impl<'a> FilterRules for FilterRuleParser<'a> {
    /// Parse any filter expression
    /// filter_expression = relative_time_filter | field_filter
    fn parse_filter_expression(&mut self) -> ParseResult<QueryFilter> {
        if self.state.check(&TokenType::Last) {
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
        }
    }
    
    /// Parse severity filter
    /// severity_filter = severity comparison_operator severity_value
    fn parse_severity_filter(&mut self) -> ParseResult<QueryFilter> {
        self.parse_comparison_operator()?;
        let value = self.parse_string_or_identifier()?;
        
        let severity = match value.to_lowercase().as_str() {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "info" | "information" => DiagnosticSeverity::Information,
            "hint" => DiagnosticSeverity::Hint,
            _ => return Err(ParseError::InvalidSeverity {
                severity: value,
                line: self.state.previous().line,
                column: self.state.previous().column,
            }),
        };
        
        Ok(QueryFilter::Severity(SeverityFilter { severity }))
    }
    
    /// Parse file filter
    /// file_filter = file comparison_operator string_pattern
    fn parse_file_filter(&mut self) -> ParseResult<QueryFilter> {
        self.parse_comparison_operator()?;
        let pattern = self.parse_string_or_identifier()?;
        
        if pattern.is_empty() {
            return Err(ParseError::EmptyPattern {
                filter_type: "file".to_string(),
                line: self.state.previous().line,
                column: self.state.previous().column,
            });
        }
        
        Ok(QueryFilter::File(FileFilter { pattern }))
    }
    
    /// Parse symbol filter
    /// symbol_filter = symbol comparison_operator string_pattern
    fn parse_symbol_filter(&mut self) -> ParseResult<QueryFilter> {
        self.parse_comparison_operator()?;
        let pattern = self.parse_string_or_identifier()?;
        
        if pattern.is_empty() {
            return Err(ParseError::EmptyPattern {
                filter_type: "symbol".to_string(),
                line: self.state.previous().line,
                column: self.state.previous().column,
            });
        }
        
        Ok(QueryFilter::Symbol(SymbolFilter { pattern }))
    }
    
    /// Parse time filter
    /// time_filter = (since|before|after) comparison_operator datetime_value
    fn parse_time_filter(&mut self, field: String) -> ParseResult<QueryFilter> {
        self.parse_comparison_operator()?;
        let value = self.parse_string_or_identifier()?;
        
        // Parse the datetime - support various formats
        let datetime = self.parse_datetime(&value)?;
        
        let time_range = match field.as_str() {
            "since" => TimeRange::since(datetime),
            "before" => TimeRange::before(datetime),
            "after" => TimeRange::after(datetime),
            _ => unreachable!("Invalid time filter field"),
        };
        
        Ok(QueryFilter::TimeRange(time_range))
    }
    
    /// Parse relative time filter
    /// relative_time_filter = LAST number time_unit
    fn parse_relative_time_filter(&mut self) -> ParseResult<QueryFilter> {
        self.state.consume(TokenType::Last, "Expected 'LAST'")?;
        
        let value_token = self.state.consume(TokenType::Number, "Expected number after LAST")?;
        let value = self.value_parser.parse_number_value(&value_token.lexeme)? as u32;
        
        if value == 0 {
            return Err(ParseError::InvalidRelativeTime {
                value,
                unit: "time".to_string(),
                reason: "Time value must be greater than 0".to_string(),
            });
        }
        
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
    
    /// Parse custom filter
    /// custom_filter = field comparison_operator value
    fn parse_custom_filter(&mut self, field: String) -> ParseResult<QueryFilter> {
        self.parse_comparison_operator()?;
        let value = self.parse_string_or_identifier()?;
        
        if field.is_empty() {
            return Err(ParseError::EmptyFieldName {
                line: self.state.previous().line,
                column: self.state.previous().column,
            });
        }
        
        Ok(QueryFilter::Custom(field, value))
    }
}

impl<'a> FilterRuleParser<'a> {
    /// Parse comparison operator
    fn parse_comparison_operator(&mut self) -> ParseResult<()> {
        if self.state.match_token(&TokenType::Equal) ||
           self.state.match_token(&TokenType::NotEqual) ||
           self.state.match_token(&TokenType::Like) ||
           self.state.match_token(&TokenType::Greater) ||
           self.state.match_token(&TokenType::Less) ||
           self.state.match_token(&TokenType::GreaterEqual) ||
           self.state.match_token(&TokenType::LessEqual) {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "comparison operator (=, !=, LIKE, >, <, >=, <=)".to_string(),
                found: self.state.peek().lexeme.clone(),
                line: self.state.peek().line,
                column: self.state.peek().column,
            })
        }
    }
    
    /// Parse string or identifier value
    fn parse_string_or_identifier(&mut self) -> ParseResult<String> {
        if self.state.check(&TokenType::String) {
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
    
    /// Parse datetime with flexible format support
    fn parse_datetime(&self, value: &str) -> ParseResult<DateTime<Utc>> {
        // Try common datetime formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.3fZ",
            "%Y-%m-%d",
        ];
        
        for format in &formats {
            if let Ok(dt) = chrono::DateTime::parse_from_str(value, format) {
                return Ok(dt.with_timezone(&Utc));
            }
        }
        
        // Try RFC3339 parsing
        if let Ok(dt) = DateTime::<Utc>::from_str(value) {
            return Ok(dt);
        }
        
        Err(ParseError::InvalidDateTime {
            value: value.to_string(),
            line: self.state.previous().line,
            column: self.state.previous().column,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::ParserState;
    use super::super::super::super::lexer::Lexer;

    fn create_filter_parser(input: &str) -> (ParserState, super::super::super::types::ParsingContext) {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let state = ParserState::new(tokens);
        let context = super::super::super::types::ParsingContext::new();
        (state, context)
    }

    #[test]
    fn test_severity_filter() {
        let (mut state, mut context) = create_filter_parser("severity = 'error'");
        state.advance(); // consume 'severity'
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        let filter = parser.parse_severity_filter().unwrap();
        if let QueryFilter::Severity(severity_filter) = filter {
            assert_eq!(severity_filter.severity, DiagnosticSeverity::Error);
        } else {
            panic!("Expected severity filter");
        }
    }

    #[test]
    fn test_file_filter() {
        let (mut state, mut context) = create_filter_parser("file LIKE '*.rs'");
        state.advance(); // consume 'file'
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        let filter = parser.parse_file_filter().unwrap();
        if let QueryFilter::File(file_filter) = filter {
            assert_eq!(file_filter.pattern, "*.rs");
        } else {
            panic!("Expected file filter");
        }
    }

    #[test]
    fn test_relative_time_filter() {
        let (mut state, mut context) = create_filter_parser("LAST 7 DAYS");
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        let filter = parser.parse_relative_time_filter().unwrap();
        if let QueryFilter::TimeRange(time_range) = filter {
            if let Some(RelativeTime::LastDays(7)) = time_range.relative {
                // Success
            } else {
                panic!("Expected 7 days relative time");
            }
        } else {
            panic!("Expected time range filter");
        }
    }

    #[test]
    fn test_custom_filter() {
        let (mut state, mut context) = create_filter_parser("custom_field = 'value'");
        state.advance(); // consume 'custom_field'
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        let filter = parser.parse_custom_filter("custom_field".to_string()).unwrap();
        if let QueryFilter::Custom(field, value) = filter {
            assert_eq!(field, "custom_field");
            assert_eq!(value, "value");
        } else {
            panic!("Expected custom filter");
        }
    }

    #[test]
    fn test_invalid_severity() {
        let (mut state, mut context) = create_filter_parser("severity = 'invalid'");
        state.advance(); // consume 'severity'
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        assert!(parser.parse_severity_filter().is_err());
    }

    #[test]
    fn test_empty_pattern() {
        let (mut state, mut context) = create_filter_parser("file = ''");
        state.advance(); // consume 'file'
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        assert!(parser.parse_file_filter().is_err());
    }

    #[test]
    fn test_invalid_relative_time_value() {
        let (mut state, mut context) = create_filter_parser("LAST 0 DAYS");
        let mut parser = FilterRuleParser::new(&mut state, &mut context);
        
        assert!(parser.parse_relative_time_filter().is_err());
    }
}