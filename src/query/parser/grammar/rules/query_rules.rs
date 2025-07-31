//! High-level query grammar rules

use super::super::types::{ParserState, ParsingContext, ParseResult};
use super::super::super::ast::*;
use crate::core::errors::ParseError;

/// Trait for parsing query-level grammar rules
pub trait QueryRules {
    /// Parse a complete query
    fn parse_query(&mut self) -> ParseResult<Query>;
    
    /// Validate query structure
    fn validate_query_structure(&self, query: &Query) -> ParseResult<()>;
}

/// Implementation of query-level parsing rules
pub struct QueryRuleParser<'a> {
    pub state: &'a mut ParserState,
    pub context: &'a mut ParsingContext,
}

impl<'a> QueryRuleParser<'a> {
    /// Create a new query rule parser
    pub fn new(state: &'a mut ParserState, context: &'a mut ParsingContext) -> Self {
        Self { state, context }
    }
}

impl<'a> QueryRules for QueryRuleParser<'a> {
    /// Parse a complete query following the grammar:
    /// query = select_clause from_clause where_clause? group_by_clause? order_by_clause? limit_clause?
    fn parse_query(&mut self) -> ParseResult<Query> {
        // This would typically delegate to the main parser
        // For now, we define the structure that the main parser should follow
        unimplemented!("This trait is used for structure definition, actual parsing is in parser.rs")
    }
    
    /// Validate that a parsed query conforms to our grammar rules
    fn validate_query_structure(&self, query: &Query) -> ParseResult<()> {
        // Ensure required clauses are present
        self.validate_required_clauses(query)?;
        
        // Validate clause order and compatibility
        self.validate_clause_compatibility(query)?;
        
        // Validate semantic constraints
        self.validate_semantic_constraints(query)?;
        
        Ok(())
    }
}

impl<'a> QueryRuleParser<'a> {
    /// Validate that required clauses are present
    fn validate_required_clauses(&self, query: &Query) -> ParseResult<()> {
        // SELECT and FROM are required
        match query.select {
            SelectClause::All | SelectClause::Count | SelectClause::Fields(_) => {}
        }
        
        match query.from {
            FromClause::Diagnostics | FromClause::Files | FromClause::Symbols | 
            FromClause::References | FromClause::Projects => {}
        }
        
        Ok(())
    }
    
    /// Validate clause compatibility
    fn validate_clause_compatibility(&self, query: &Query) -> ParseResult<()> {
        // GROUP BY requires SELECT fields or COUNT
        if query.group_by.is_some() {
            match &query.select {
                SelectClause::All => {
                    return Err(ParseError::IncompatibleClauses {
                        clause1: "SELECT *".to_string(),
                        clause2: "GROUP BY".to_string(),
                        reason: "Cannot use SELECT * with GROUP BY".to_string(),
                    });
                }
                SelectClause::Count | SelectClause::Fields(_) => {}
            }
        }
        
        // ORDER BY field should exist in SELECT fields (if not SELECT *)
        if let (Some(order_by), SelectClause::Fields(fields)) = (&query.order_by, &query.select) {
            if !fields.contains(&order_by.field) {
                return Err(ParseError::InvalidOrderByField {
                    field: order_by.field.clone(),
                    available_fields: fields.clone(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Validate semantic constraints
    fn validate_semantic_constraints(&self, query: &Query) -> ParseResult<()> {
        // LIMIT should be positive
        if let Some(limit) = query.limit {
            if limit == 0 {
                return Err(ParseError::InvalidLimit {
                    value: limit,
                    reason: "LIMIT must be greater than 0".to_string(),
                });
            }
        }
        
        // Time range filters should be valid
        if let Some(ref time_range) = query.time_range {
            self.validate_time_range(time_range)?;
        }
        
        Ok(())
    }
    
    /// Validate time range constraints
    fn validate_time_range(&self, time_range: &TimeRange) -> ParseResult<()> {
        // Check for conflicting absolute times
        if let (Some(start), Some(end)) = (&time_range.start, &time_range.end) {
            if start >= end {
                return Err(ParseError::InvalidTimeRange {
                    start: start.to_string(),
                    end: end.to_string(),
                    reason: "Start time must be before end time".to_string(),
                });
            }
        }
        
        // Check relative time validity
        if let Some(ref relative) = time_range.relative {
            match relative {
                RelativeTime::LastHours(hours) => {
                    if *hours == 0 {
                        return Err(ParseError::InvalidRelativeTime {
                            value: *hours,
                            unit: "hours".to_string(),
                            reason: "Time value must be greater than 0".to_string(),
                        });
                    }
                }
                RelativeTime::LastDays(days) => {
                    if *days == 0 {
                        return Err(ParseError::InvalidRelativeTime {
                            value: *days,
                            unit: "days".to_string(),
                            reason: "Time value must be greater than 0".to_string(),
                        });
                    }
                }
                RelativeTime::LastWeeks(weeks) => {
                    if *weeks == 0 {
                        return Err(ParseError::InvalidRelativeTime {
                            value: *weeks,
                            unit: "weeks".to_string(),
                            reason: "Time value must be greater than 0".to_string(),
                        });
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::ParserState;
    use super::super::super::super::lexer::Lexer;

    #[test]
    fn test_query_validation() {
        // Create a minimal valid query
        let query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            time_range: None,
            group_by: None,
            order_by: None,
            limit: None,
        };
        
        let mut lexer = Lexer::new("SELECT * FROM diagnostics");
        let tokens = lexer.tokenize().unwrap();
        let mut state = ParserState::new(tokens);
        let mut context = super::super::super::types::ParsingContext::new();
        
        let parser = QueryRuleParser::new(&mut state, &mut context);
        assert!(parser.validate_query_structure(&query).is_ok());
    }
    
    #[test]
    fn test_invalid_group_by_with_select_all() {
        let query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            time_range: None,
            group_by: Some(GroupByClause { 
                fields: vec!["severity".to_string()]
            }),
            order_by: None,
            limit: None,
        };
        
        let mut lexer = Lexer::new("SELECT * FROM diagnostics");
        let tokens = lexer.tokenize().unwrap();
        let mut state = ParserState::new(tokens);
        let mut context = super::super::super::types::ParsingContext::new();
        
        let parser = QueryRuleParser::new(&mut state, &mut context);
        assert!(parser.validate_query_structure(&query).is_err());
    }
    
    #[test]
    fn test_invalid_limit() {
        let query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            time_range: None,
            group_by: None,
            order_by: None,
            limit: Some(0),
        };
        
        let mut lexer = Lexer::new("SELECT * FROM diagnostics");
        let tokens = lexer.tokenize().unwrap();
        let mut state = ParserState::new(tokens);
        let mut context = super::super::super::types::ParsingContext::new();
        
        let parser = QueryRuleParser::new(&mut state, &mut context);
        assert!(parser.validate_query_structure(&query).is_err());
    }
}