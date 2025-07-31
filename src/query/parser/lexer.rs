//! Lexical analysis (tokenization) for the query language

use crate::core::errors::ParseError;
use std::collections::HashMap;
use std::fmt;

/// Token types in the query language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    Select,
    From,
    Where,
    And,
    Or,
    GroupBy,
    OrderBy,
    Limit,

    // Aggregation functions
    Count,
    Sum,
    Avg,
    Min,
    Max,

    // Operators
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    In,
    Like,

    // Time keywords
    Last,
    Days,
    Hours,
    Weeks,

    // Data sources
    Errors,
    Warnings,
    Files,
    Diagnostics,
    History,
    Trends,

    // Order directions
    Asc,
    Desc,

    // Punctuation
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Asterisk,
    Dot,

    // Literals
    Number(f64),
    String(String),
    Identifier(String),

    // Special
    Eof,
}

/// A token with its type and position information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

/// Lexical analyzer for the query language
pub struct Lexer {
    input: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
    keywords: HashMap<String, TokenType>,
}

impl Lexer {
    /// Create a new lexer with the given input
    pub fn new(input: &str) -> Self {
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

        Self {
            input: input.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
            keywords,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();
            
            if self.is_at_end() {
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        tokens.push(Token {
            token_type: TokenType::Eof,
            lexeme: String::new(),
            line: self.line,
            column: self.column,
        });

        Ok(tokens)
    }

    /// Get the next token
    fn next_token(&mut self) -> Result<Token, ParseError> {
        let start_line = self.line;
        let start_column = self.column;
        let ch = self.advance();

        let (token_type, lexeme) = match ch {
            '(' => (TokenType::LeftParen, ch.to_string()),
            ')' => (TokenType::RightParen, ch.to_string()),
            ',' => (TokenType::Comma, ch.to_string()),
            ';' => (TokenType::Semicolon, ch.to_string()),
            '*' => (TokenType::Asterisk, ch.to_string()),
            '.' => (TokenType::Dot, ch.to_string()),
            '=' => (TokenType::Equal, ch.to_string()),
            '!' if self.peek() == '=' => {
                self.advance();
                (TokenType::NotEqual, "!=".to_string())
            }
            '>' if self.peek() == '=' => {
                self.advance();
                (TokenType::GreaterThanOrEqual, ">=".to_string())
            }
            '>' => (TokenType::GreaterThan, ch.to_string()),
            '<' if self.peek() == '=' => {
                self.advance();
                (TokenType::LessThanOrEqual, "<=".to_string())
            }
            '<' => (TokenType::LessThan, ch.to_string()),
            '"' | '\'' => {
                let string_val = self.string(ch)?;
                (TokenType::String(string_val.clone()), string_val)
            }
            ch if ch.is_ascii_digit() => {
                let (number, lexeme) = self.number(ch)?;
                (TokenType::Number(number), lexeme)
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let identifier = self.identifier(ch);
                let token_type = self.keywords
                    .get(&identifier.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| TokenType::Identifier(identifier.clone()));
                (token_type, identifier)
            }
            _ => {
                return Err(ParseError::UnexpectedCharacter {
                    character: ch,
                    line: start_line,
                    column: start_column,
                });
            }
        };

        Ok(Token {
            token_type,
            lexeme,
            line: start_line,
            column: start_column,
        })
    }

    /// Parse a string literal
    fn string(&mut self, quote: char) -> Result<String, ParseError> {
        let mut value = String::new();
        let start_line = self.line;
        let start_column = self.column;

        while self.peek() != quote && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }
            value.push(self.advance());
        }

        if self.is_at_end() {
            return Err(ParseError::UnterminatedString {
                line: start_line,
                column: start_column,
            });
        }

        // Consume the closing quote
        self.advance();
        Ok(value)
    }

    /// Parse a number literal
    fn number(&mut self, first_digit: char) -> Result<(f64, String), ParseError> {
        let mut lexeme = String::new();
        lexeme.push(first_digit);

        while self.peek().is_ascii_digit() {
            lexeme.push(self.advance());
        }

        // Handle decimal numbers
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            lexeme.push(self.advance()); // consume '.'
            while self.peek().is_ascii_digit() {
                lexeme.push(self.advance());
            }
        }

        let number = lexeme.parse::<f64>().map_err(|_| ParseError::InvalidNumber {
            value: lexeme.clone(),
            line: self.line,
            column: self.column,
        })?;

        Ok((number, lexeme))
    }

    /// Parse an identifier
    fn identifier(&mut self, first_char: char) -> String {
        let mut identifier = String::new();
        identifier.push(first_char);

        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            identifier.push(self.advance());
        }

        identifier
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Check if we're at the end of input
    fn is_at_end(&self) -> bool {
        self.current >= self.input.len()
    }

    /// Advance to the next character
    fn advance(&mut self) -> char {
        let ch = self.input[self.current];
        self.current += 1;
        self.column += 1;
        ch
    }

    /// Peek at the current character without advancing
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.current]
        }
    }

    /// Peek at the next character without advancing
    fn peek_next(&self) -> char {
        if self.current + 1 >= self.input.len() {
            '\0'
        } else {
            self.input[self.current + 1]
        }
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Select => write!(f, "SELECT"),
            TokenType::From => write!(f, "FROM"),
            TokenType::Where => write!(f, "WHERE"),
            TokenType::And => write!(f, "AND"),
            TokenType::Or => write!(f, "OR"),
            TokenType::GroupBy => write!(f, "GROUP BY"),
            TokenType::OrderBy => write!(f, "ORDER BY"),
            TokenType::Limit => write!(f, "LIMIT"),
            TokenType::Count => write!(f, "COUNT"),
            TokenType::Sum => write!(f, "SUM"),
            TokenType::Avg => write!(f, "AVG"),
            TokenType::Min => write!(f, "MIN"),
            TokenType::Max => write!(f, "MAX"),
            TokenType::Equal => write!(f, "="),
            TokenType::NotEqual => write!(f, "!="),
            TokenType::GreaterThan => write!(f, ">"),
            TokenType::LessThan => write!(f, "<"),
            TokenType::GreaterThanOrEqual => write!(f, ">="),
            TokenType::LessThanOrEqual => write!(f, "<="),
            TokenType::In => write!(f, "IN"),
            TokenType::Like => write!(f, "LIKE"),
            TokenType::Last => write!(f, "LAST"),
            TokenType::Days => write!(f, "DAYS"),
            TokenType::Hours => write!(f, "HOURS"),
            TokenType::Weeks => write!(f, "WEEKS"),
            TokenType::Errors => write!(f, "ERRORS"),
            TokenType::Warnings => write!(f, "WARNINGS"),
            TokenType::Files => write!(f, "FILES"),
            TokenType::Diagnostics => write!(f, "DIAGNOSTICS"),
            TokenType::History => write!(f, "HISTORY"),
            TokenType::Trends => write!(f, "TRENDS"),
            TokenType::Asc => write!(f, "ASC"),
            TokenType::Desc => write!(f, "DESC"),
            TokenType::LeftParen => write!(f, "("),
            TokenType::RightParen => write!(f, ")"),
            TokenType::Comma => write!(f, ","),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Asterisk => write!(f, "*"),
            TokenType::Dot => write!(f, "."),
            TokenType::Number(n) => write!(f, "{}", n),
            TokenType::String(s) => write!(f, "\"{}\"", s),
            TokenType::Identifier(id) => write!(f, "{}", id),
            TokenType::Eof => write!(f, "EOF"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic_tokens() {
        let mut lexer = Lexer::new("SELECT * FROM diagnostics");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 5); // including EOF
        assert_eq!(tokens[0].token_type, TokenType::Select);
        assert_eq!(tokens[1].token_type, TokenType::Asterisk);
        assert_eq!(tokens[2].token_type, TokenType::From);
        assert_eq!(tokens[3].token_type, TokenType::Diagnostics);
        assert_eq!(tokens[4].token_type, TokenType::Eof);
    }

    #[test]
    fn test_lexer_string_literals() {
        let mut lexer = Lexer::new("'hello world' \"test string\"");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 3); // including EOF
        assert_eq!(tokens[0].token_type, TokenType::String("hello world".to_string()));
        assert_eq!(tokens[1].token_type, TokenType::String("test string".to_string()));
    }

    #[test]
    fn test_lexer_numbers() {
        let mut lexer = Lexer::new("42 3.14 0");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 4); // including EOF
        assert_eq!(tokens[0].token_type, TokenType::Number(42.0));
        assert_eq!(tokens[1].token_type, TokenType::Number(3.14));
        assert_eq!(tokens[2].token_type, TokenType::Number(0.0));
    }

    #[test]
    fn test_lexer_operators() {
        let mut lexer = Lexer::new("= != > < >= <=");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 7); // including EOF
        assert_eq!(tokens[0].token_type, TokenType::Equal);
        assert_eq!(tokens[1].token_type, TokenType::NotEqual);
        assert_eq!(tokens[2].token_type, TokenType::GreaterThan);
        assert_eq!(tokens[3].token_type, TokenType::LessThan);
        assert_eq!(tokens[4].token_type, TokenType::GreaterThanOrEqual);
        assert_eq!(tokens[5].token_type, TokenType::LessThanOrEqual);
    }
}