//! Range conversion utilities for different diagnostic formats

use crate::core::errors::ParseError;
use crate::core::{Position, Range};
use serde_json::Value;

/// Converter for diagnostic ranges across different formats
pub struct RangeConverter;

impl RangeConverter {
    /// Convert TypeScript range format
    pub fn convert_typescript(
        start: Option<&Value>,
        end: Option<&Value>,
    ) -> Result<Range, ParseError> {
        let start_pos = match start {
            Some(s) => Position {
                line: s.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
                character: s.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
            },
            None => Position {
                line: 0,
                character: 0,
            },
        };

        let end_pos = match end {
            Some(e) => Position {
                line: e.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
                character: e.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
            },
            None => start_pos.clone(),
        };

        Ok(Range {
            start: start_pos,
            end: end_pos,
        })
    }

    /// Convert Rust analyzer range format
    pub fn convert_rust(span: &Value) -> Result<Range, ParseError> {
        let line_start = span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(1) as u32;
        let line_end = span
            .get("line_end")
            .and_then(|l| l.as_u64())
            .unwrap_or(line_start as u64) as u32;
        let column_start = span
            .get("column_start")
            .and_then(|c| c.as_u64())
            .unwrap_or(1) as u32;
        let column_end = span
            .get("column_end")
            .and_then(|c| c.as_u64())
            .unwrap_or(column_start as u64) as u32;

        Ok(Range {
            start: Position {
                line: line_start.saturating_sub(1), // Rust uses 1-based lines
                character: column_start.saturating_sub(1),
            },
            end: Position {
                line: line_end.saturating_sub(1),
                character: column_end.saturating_sub(1),
            },
        })
    }

    /// Convert ESLint range format
    pub fn convert_eslint(message: &Value) -> Result<Range, ParseError> {
        let line = message.get("line").and_then(|l| l.as_u64()).unwrap_or(1) as u32;
        let column = message.get("column").and_then(|c| c.as_u64()).unwrap_or(1) as u32;
        let end_line = message
            .get("endLine")
            .and_then(|l| l.as_u64())
            .unwrap_or(line as u64) as u32;
        let end_column = message
            .get("endColumn")
            .and_then(|c| c.as_u64())
            .unwrap_or(column as u64) as u32;

        Ok(Range {
            start: Position {
                line: line.saturating_sub(1), // ESLint uses 1-based lines
                character: column.saturating_sub(1),
            },
            end: Position {
                line: end_line.saturating_sub(1),
                character: end_column.saturating_sub(1),
            },
        })
    }

    /// Convert LSP standard range format
    pub fn convert_lsp(range: Option<&Value>) -> Result<Range, ParseError> {
        let range = range.ok_or_else(|| ParseError::InvalidFormat {
            context: "LSP diagnostic".to_string(),
            expected: "range object".to_string(),
            found: "missing range".to_string(),
        })?;

        let start = range
            .get("start")
            .ok_or_else(|| ParseError::InvalidFormat {
                context: "LSP range".to_string(),
                expected: "start position".to_string(),
                found: "missing start".to_string(),
            })?;
        let end = range.get("end").ok_or_else(|| ParseError::InvalidFormat {
            context: "LSP range".to_string(),
            expected: "end position".to_string(),
            found: "missing end".to_string(),
        })?;

        let start_pos = Position {
            line: start.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
            character: start.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
        };

        let end_pos = Position {
            line: end.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
            character: end.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
        };

        Ok(Range {
            start: start_pos,
            end: end_pos,
        })
    }
}