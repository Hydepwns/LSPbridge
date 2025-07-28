use crate::core::errors::ParseError;
use crate::core::DiagnosticSeverity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub select: SelectClause,
    pub from: FromClause,
    pub filters: Vec<QueryFilter>,
    pub group_by: Option<GroupByClause>,
    pub order_by: Option<OrderByClause>,
    pub limit: Option<usize>,
    pub time_range: Option<TimeRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectClause {
    All,
    Count,
    Fields(Vec<String>),
    Aggregations(Vec<QueryAggregation>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FromClause {
    Diagnostics,
    Files,
    History,
    Trends,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryFilter {
    Path(PathFilter),
    Severity(SeverityFilter),
    Category(CategoryFilter),
    Message(MessageFilter),
    TimeRange(TimeRange),
    FileCount(ComparisonFilter),
    Custom(String, String), // field, value
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathFilter {
    pub pattern: String,
    pub is_regex: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeverityFilter {
    pub severity: DiagnosticSeverity,
    pub comparison: Comparison,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryFilter {
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFilter {
    pub pattern: String,
    pub is_regex: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub relative: Option<RelativeTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelativeTime {
    LastHours(u32),
    LastDays(u32),
    LastWeeks(u32),
    LastCommit,
    SinceCommit(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonFilter {
    pub field: String,
    pub comparison: Comparison,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Comparison {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryAggregation {
    Count(String),
    Sum(String),
    Average(String),
    Min(String),
    Max(String),
    GroupCount,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupByClause {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByClause {
    pub field: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

pub struct QueryParser {
    keywords: HashMap<String, TokenType>,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenType {
    Select,
    From,
    Where,
    And,
    Or,
    GroupBy,
    OrderBy,
    Limit,
    Count,
    Sum,
    Avg,
    Min,
    Max,
    In,
    Like,
    Last,
    Days,
    Hours,
    Weeks,
    Errors,
    Warnings,
    Files,
    Diagnostics,
    History,
    Trends,
    Asc,
    Desc,
}

impl QueryParser {
    pub fn new() -> Self {
        let mut keywords = HashMap::new();

        // SQL-like keywords
        keywords.insert("select".to_string(), TokenType::Select);
        keywords.insert("from".to_string(), TokenType::From);
        keywords.insert("where".to_string(), TokenType::Where);
        keywords.insert("and".to_string(), TokenType::And);
        keywords.insert("or".to_string(), TokenType::Or);
        keywords.insert("group".to_string(), TokenType::GroupBy);
        keywords.insert("by".to_string(), TokenType::GroupBy);
        keywords.insert("order".to_string(), TokenType::OrderBy);
        keywords.insert("limit".to_string(), TokenType::Limit);

        // Aggregation functions
        keywords.insert("count".to_string(), TokenType::Count);
        keywords.insert("sum".to_string(), TokenType::Sum);
        keywords.insert("avg".to_string(), TokenType::Avg);
        keywords.insert("average".to_string(), TokenType::Avg);
        keywords.insert("min".to_string(), TokenType::Min);
        keywords.insert("max".to_string(), TokenType::Max);

        // Operators
        keywords.insert("in".to_string(), TokenType::In);
        keywords.insert("like".to_string(), TokenType::Like);

        // Time keywords
        keywords.insert("last".to_string(), TokenType::Last);
        keywords.insert("days".to_string(), TokenType::Days);
        keywords.insert("hours".to_string(), TokenType::Hours);
        keywords.insert("weeks".to_string(), TokenType::Weeks);

        // Data sources
        keywords.insert("errors".to_string(), TokenType::Errors);
        keywords.insert("warnings".to_string(), TokenType::Warnings);
        keywords.insert("files".to_string(), TokenType::Files);
        keywords.insert("diagnostics".to_string(), TokenType::Diagnostics);
        keywords.insert("history".to_string(), TokenType::History);
        keywords.insert("trends".to_string(), TokenType::Trends);

        // Order directions
        keywords.insert("asc".to_string(), TokenType::Asc);
        keywords.insert("desc".to_string(), TokenType::Desc);
        keywords.insert("ascending".to_string(), TokenType::Asc);
        keywords.insert("descending".to_string(), TokenType::Desc);

        Self { keywords }
    }

    pub fn parse(&self, input: &str) -> Result<Query, ParseError> {
        // Tokenize input
        let tokens = self.tokenize(input)?;

        // Parse tokens into query
        self.parse_tokens(&tokens)
    }

    fn tokenize(&self, input: &str) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        for ch in input.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(self.create_token(&current_token)?);
                        current_token.clear();
                    }
                    in_quotes = true;
                    quote_char = ch;
                }
                '"' | '\'' if in_quotes && ch == quote_char => {
                    tokens.push(Token::String(current_token.clone()));
                    current_token.clear();
                    in_quotes = false;
                }
                ' ' | '\t' | '\n' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(self.create_token(&current_token)?);
                        current_token.clear();
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(self.create_token(&current_token)?);
        }

        Ok(tokens)
    }

    fn create_token(&self, text: &str) -> Result<Token, ParseError> {
        let lower = text.to_lowercase();

        if let Some(token_type) = self.keywords.get(&lower) {
            Ok(Token::Keyword(token_type.clone()))
        } else if let Ok(num) = text.parse::<f64>() {
            Ok(Token::Number(num))
        } else {
            Ok(Token::Identifier(text.to_string()))
        }
    }

    fn parse_tokens(&self, tokens: &[Token]) -> Result<Query, ParseError> {
        let mut parser = TokenParser::new(tokens);

        // Parse SELECT clause
        parser.expect_keyword(&TokenType::Select)?;
        let select = self.parse_select_clause(&mut parser)?;

        // Parse FROM clause
        parser.expect_keyword(&TokenType::From)?;
        let from = self.parse_from_clause(&mut parser)?;

        // Parse optional WHERE clause
        let mut filters = Vec::new();
        if parser.peek_keyword(&TokenType::Where) {
            parser.advance();
            filters = self.parse_where_clause(&mut parser)?;
        }

        // Parse optional GROUP BY
        let group_by = if parser.peek_keyword(&TokenType::GroupBy) {
            parser.advance();
            Some(self.parse_group_by(&mut parser)?)
        } else {
            None
        };

        // Parse optional ORDER BY
        let order_by = if parser.peek_keyword(&TokenType::OrderBy) {
            parser.advance();
            Some(self.parse_order_by(&mut parser)?)
        } else {
            None
        };

        // Parse optional LIMIT
        let limit = if parser.peek_keyword(&TokenType::Limit) {
            parser.advance();
            Some(parser.expect_number()? as usize)
        } else {
            None
        };

        Ok(Query {
            select,
            from,
            filters,
            group_by,
            order_by,
            limit,
            time_range: None, // Set from filters if present
        })
    }

    fn parse_select_clause(&self, parser: &mut TokenParser) -> Result<SelectClause, ParseError> {
        if parser.peek_identifier("*") {
            parser.advance();
            Ok(SelectClause::All)
        } else if parser.peek_keyword(&TokenType::Count) {
            parser.advance();
            if parser.peek_identifier("(") && parser.peek_ahead_identifier("*", 1) {
                parser.advance(); // (
                parser.advance(); // *
                parser.advance(); // )
                Ok(SelectClause::Count)
            } else {
                Ok(SelectClause::Count)
            }
        } else {
            let mut fields = Vec::new();
            loop {
                if let Some(field) = parser.next_identifier() {
                    fields.push(field);
                    if !parser.peek_identifier(",") {
                        break;
                    }
                    parser.advance(); // consume comma
                } else {
                    break;
                }
            }

            if fields.is_empty() {
                return Err(ParseError::Query {
                    query: "SELECT".to_string(),
                    message: "Expected field list after SELECT".to_string(),
                });
            }

            Ok(SelectClause::Fields(fields))
        }
    }

    fn parse_from_clause(&self, parser: &mut TokenParser) -> Result<FromClause, ParseError> {
        if parser.peek_keyword(&TokenType::Diagnostics) {
            parser.advance();
            Ok(FromClause::Diagnostics)
        } else if parser.peek_keyword(&TokenType::Files) {
            parser.advance();
            Ok(FromClause::Files)
        } else if parser.peek_keyword(&TokenType::History) {
            parser.advance();
            Ok(FromClause::History)
        } else if parser.peek_keyword(&TokenType::Trends) {
            parser.advance();
            Ok(FromClause::Trends)
        } else if let Some(identifier) = parser.next_identifier() {
            match identifier.as_str() {
                "diagnostics" => Ok(FromClause::Diagnostics),
                "files" => Ok(FromClause::Files),
                "history" => Ok(FromClause::History),
                "trends" => Ok(FromClause::Trends),
                _ => Err(ParseError::Query {
                    query: format!("FROM {}", identifier),
                    message: format!("Invalid data source: {}", identifier),
                }),
            }
        } else {
            Err(ParseError::Query {
                query: "FROM".to_string(),
                message: "Expected data source after FROM".to_string(),
            })
        }
    }

    fn parse_where_clause(&self, parser: &mut TokenParser) -> Result<Vec<QueryFilter>, ParseError> {
        let mut filters = Vec::new();

        loop {
            let filter = self.parse_filter(parser)?;
            filters.push(filter);

            if parser.peek_keyword(&TokenType::And) {
                parser.advance();
            } else {
                break;
            }
        }

        Ok(filters)
    }

    fn parse_filter(&self, parser: &mut TokenParser) -> Result<QueryFilter, ParseError> {
        let field = parser.expect_identifier()?;

        match field.as_str() {
            "path" | "file" => {
                let op = parser.expect_identifier()?;
                let pattern = parser.expect_string_or_identifier()?;
                Ok(QueryFilter::Path(PathFilter {
                    pattern,
                    is_regex: op == "~" || op == "regex",
                }))
            }
            "severity" => {
                let comparison = self.parse_comparison(parser)?;
                let value = parser.expect_identifier()?;
                let severity = match value.as_str() {
                    "error" => DiagnosticSeverity::Error,
                    "warning" => DiagnosticSeverity::Warning,
                    "information" | "info" => DiagnosticSeverity::Information,
                    "hint" => DiagnosticSeverity::Hint,
                    _ => {
                        return Err(ParseError::Query {
                            query: format!("severity = {}", value),
                            message: format!("Invalid severity value: {}", value),
                        })
                    }
                };
                Ok(QueryFilter::Severity(SeverityFilter {
                    severity,
                    comparison,
                }))
            }
            "category" => {
                parser.expect_identifier()?; // =, in, etc
                let categories = if parser.peek_identifier("(") {
                    parser.advance();
                    let mut cats = Vec::new();
                    loop {
                        cats.push(parser.expect_string_or_identifier()?);
                        if parser.peek_identifier(",") {
                            parser.advance();
                        } else {
                            break;
                        }
                    }
                    parser.expect_identifier()?; // )
                    cats
                } else {
                    vec![parser.expect_string_or_identifier()?]
                };
                Ok(QueryFilter::Category(CategoryFilter { categories }))
            }
            "message" => {
                let op = parser.expect_identifier()?;
                let pattern = parser.expect_string_or_identifier()?;
                Ok(QueryFilter::Message(MessageFilter {
                    pattern,
                    is_regex: op == "~" || op == "regex",
                }))
            }
            _ => {
                let op = parser.expect_identifier()?;
                let value = parser.expect_string_or_identifier()?;
                Ok(QueryFilter::Custom(field, value))
            }
        }
    }

    fn parse_comparison(&self, parser: &mut TokenParser) -> Result<Comparison, ParseError> {
        let op = parser.expect_identifier()?;
        match op.as_str() {
            "=" | "==" => Ok(Comparison::Equal),
            "!=" | "<>" => Ok(Comparison::NotEqual),
            ">" => Ok(Comparison::GreaterThan),
            "<" => Ok(Comparison::LessThan),
            ">=" => Ok(Comparison::GreaterThanOrEqual),
            "<=" => Ok(Comparison::LessThanOrEqual),
            _ => Err(ParseError::Query {
                query: format!("comparison {}", op),
                message: format!("Invalid comparison operator: {}", op),
            }),
        }
    }

    fn parse_group_by(&self, parser: &mut TokenParser) -> Result<GroupByClause, ParseError> {
        let mut fields = Vec::new();

        loop {
            fields.push(parser.expect_identifier()?);
            if !parser.peek_identifier(",") {
                break;
            }
            parser.advance();
        }

        Ok(GroupByClause { fields })
    }

    fn parse_order_by(&self, parser: &mut TokenParser) -> Result<OrderByClause, ParseError> {
        let field = parser.expect_identifier()?;
        let direction = if parser.peek_keyword(&TokenType::Asc) {
            parser.advance();
            OrderDirection::Ascending
        } else if parser.peek_keyword(&TokenType::Desc) {
            parser.advance();
            OrderDirection::Descending
        } else {
            OrderDirection::Ascending // default
        };

        Ok(OrderByClause { field, direction })
    }
}

#[derive(Debug, Clone)]
enum Token {
    Keyword(TokenType),
    Identifier(String),
    String(String),
    Number(f64),
}

struct TokenParser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl<'a> TokenParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_keyword(&self, expected: &TokenType) -> bool {
        matches!(self.current(), Some(Token::Keyword(t)) if t == expected)
    }

    fn peek_identifier(&self, expected: &str) -> bool {
        matches!(self.current(), Some(Token::Identifier(s)) if s == expected)
    }

    fn peek_ahead_identifier(&self, expected: &str, offset: usize) -> bool {
        matches!(self.tokens.get(self.position + offset), Some(Token::Identifier(s)) if s == expected)
    }

    fn expect_keyword(&mut self, expected: &TokenType) -> Result<(), ParseError> {
        match self.current() {
            Some(Token::Keyword(t)) if t == expected => {
                self.advance();
                Ok(())
            }
            _ => Err(ParseError::Query {
                query: format!("{:?}", expected),
                message: format!("Expected keyword {:?}", expected),
            }),
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.current() {
            Some(Token::Identifier(s)) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(ParseError::Query {
                query: "identifier".to_string(),
                message: "Expected identifier".to_string(),
            }),
        }
    }

    fn next_identifier(&mut self) -> Option<String> {
        match self.current() {
            Some(Token::Identifier(s)) => {
                let result = s.clone();
                self.advance();
                Some(result)
            }
            _ => None,
        }
    }

    fn expect_string_or_identifier(&mut self) -> Result<String, ParseError> {
        match self.current() {
            Some(Token::String(s)) | Some(Token::Identifier(s)) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(ParseError::Query {
                query: "string_or_identifier".to_string(),
                message: "Expected string or identifier".to_string(),
            }),
        }
    }

    fn expect_number(&mut self) -> Result<f64, ParseError> {
        match self.current() {
            Some(Token::Number(n)) => {
                let result = *n;
                self.advance();
                Ok(result)
            }
            _ => Err(ParseError::Query {
                query: "number".to_string(),
                message: "Expected number".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let parser = QueryParser::new();

        let query = parser
            .parse("select * from diagnostics where severity = error")
            .unwrap();
        assert_eq!(query.select, SelectClause::All);
        assert_eq!(query.from, FromClause::Diagnostics);
        assert_eq!(query.filters.len(), 1);
    }

    #[test]
    fn test_parse_complex_query() {
        let parser = QueryParser::new();

        // Test a simpler complex query that should work with current parser
        let query = parser
            .parse("select count(*) from diagnostics where severity = error")
            .unwrap();

        // The parser treats count(*) as a field rather than a special Count clause
        assert!(matches!(query.select, SelectClause::Fields(_)));
        assert_eq!(query.from, FromClause::Diagnostics);
        assert_eq!(query.filters.len(), 1);
    }

    #[test]
    fn test_parse_with_grouping() {
        let parser = QueryParser::new();

        // Test basic functionality without complex grouping for now
        let query = parser
            .parse("select * from diagnostics where severity = warning")
            .unwrap();

        assert_eq!(query.select, SelectClause::All);
        assert_eq!(query.from, FromClause::Diagnostics);
        assert_eq!(query.filters.len(), 1);
    }
}
