//! Query result types and value system
//!
//! This module defines the data structures used to represent query results,
//! including rows, values, and metadata. It provides a type-safe way to
//! handle heterogeneous data from different sources.

use crate::core::DiagnosticSeverity;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A complete query result containing rows, metadata, and timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column names for the result set
    pub columns: Vec<String>,
    
    /// Data rows matching the query
    pub rows: Vec<Row>,
    
    /// Total number of rows before LIMIT was applied
    pub total_count: usize,
    
    /// Time taken to execute the query in milliseconds
    pub query_time_ms: u64,
    
    /// Additional metadata about query execution
    pub metadata: QueryMetadata,
}

/// A single row in a query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    /// Values for each column in the row
    pub values: Vec<Value>,
}

/// A typed value that can appear in query results
///
/// Supports various data types commonly found in diagnostic and file data,
/// with proper serialization for JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Path(PathBuf),
    Severity(DiagnosticSeverity),
    Array(Vec<Value>),
    Null,
}

/// Metadata about query execution for debugging and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Which data source was queried
    pub data_source: String,
    
    /// Number of filters that were applied
    pub filters_applied: usize,
    
    /// Total number of rows examined before filtering
    pub rows_scanned: usize,
    
    /// Whether the result came from cache
    pub cache_hit: bool,
}

/// Statistics for file-based queries
#[derive(Debug, Clone, Default)]
pub struct FileStatistics {
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub hint_count: usize,
    pub total_count: usize,
}

impl Value {
    /// Convert any value to its string representation
    ///
    /// This is useful for display purposes and text-based operations.
    /// Each value type has a sensible string conversion.
    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Path(p) => p.display().to_string(),
            Value::Severity(s) => format!("{s:?}"),
            Value::Array(arr) => format!(
                "[{}]",
                arr.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Null => "null".to_string(),
        }
    }

    /// Check if this value is numeric (for sorting and aggregation)
    pub fn is_numeric(&self) -> bool {
        matches!(self, Value::Number(_) | Value::Integer(_))
    }

    /// Get numeric value if this is a number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Check if this value represents a string-like type
    pub fn is_string_like(&self) -> bool {
        matches!(self, Value::String(_) | Value::Path(_))
    }

    /// Get string representation for comparison purposes
    pub fn as_string(&self) -> String {
        self.to_string()
    }
}

impl QueryResult {
    /// Create a new empty query result
    pub fn empty(data_source: &str) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            total_count: 0,
            query_time_ms: 0,
            metadata: QueryMetadata {
                data_source: data_source.to_string(),
                filters_applied: 0,
                rows_scanned: 0,
                cache_hit: false,
            },
        }
    }

    /// Create a single-value result (used for COUNT queries)
    pub fn single_value(data_source: &str, column: &str, value: Value) -> Self {
        Self {
            columns: vec![column.to_string()],
            rows: vec![Row { values: vec![value] }],
            total_count: 1,
            query_time_ms: 0,
            metadata: QueryMetadata {
                data_source: data_source.to_string(),
                filters_applied: 0,
                rows_scanned: 0,
                cache_hit: false,
            },
        }
    }

    /// Set query execution time
    pub fn with_timing(mut self, query_time_ms: u64) -> Self {
        self.query_time_ms = query_time_ms;
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: QueryMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Mark result as coming from cache
    pub fn with_cache_hit(mut self) -> Self {
        self.metadata.cache_hit = true;
        self
    }
}

impl Row {
    /// Create a new row with the given values
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }

    /// Get value at specific column index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Get number of columns in this row
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if row is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl FileStatistics {
    /// Create new empty file statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment count for the given severity level
    pub fn increment_severity(&mut self, severity: DiagnosticSeverity) {
        match severity {
            DiagnosticSeverity::Error => self.error_count += 1,
            DiagnosticSeverity::Warning => self.warning_count += 1,
            DiagnosticSeverity::Information => self.info_count += 1,
            DiagnosticSeverity::Hint => self.hint_count += 1,
        }
        self.total_count += 1;
    }

    /// Get total diagnostic count
    pub fn total(&self) -> usize {
        self.total_count
    }

    /// Check if file has any diagnostics
    pub fn has_diagnostics(&self) -> bool {
        self.total_count > 0
    }

    /// Get count for specific severity
    pub fn get_severity_count(&self, severity: DiagnosticSeverity) -> usize {
        match severity {
            DiagnosticSeverity::Error => self.error_count,
            DiagnosticSeverity::Warning => self.warning_count,
            DiagnosticSeverity::Information => self.info_count,
            DiagnosticSeverity::Hint => self.hint_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversion() {
        let string_val = Value::String("test".to_string());
        assert_eq!(string_val.to_string(), "test");
        assert!(string_val.is_string_like());
        assert!(!string_val.is_numeric());

        let int_val = Value::Integer(42);
        assert_eq!(int_val.to_string(), "42");
        assert!(int_val.is_numeric());
        assert_eq!(int_val.as_number(), Some(42.0));

        let path_val = Value::Path(PathBuf::from("/test/path"));
        assert!(path_val.is_string_like());
        assert_eq!(path_val.as_string(), "/test/path");
    }

    #[test]
    fn test_query_result_builders() {
        let result = QueryResult::empty("diagnostics");
        assert_eq!(result.columns.len(), 0);
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.metadata.data_source, "diagnostics");

        let count_result = QueryResult::single_value(
            "files", 
            "count", 
            Value::Integer(10)
        );
        assert_eq!(count_result.columns, vec!["count"]);
        assert_eq!(count_result.rows.len(), 1);
        assert_eq!(count_result.total_count, 1);
    }

    #[test]
    fn test_file_statistics() {
        let mut stats = FileStatistics::new();
        assert!(!stats.has_diagnostics());

        stats.increment_severity(DiagnosticSeverity::Error);
        stats.increment_severity(DiagnosticSeverity::Warning);

        assert!(stats.has_diagnostics());
        assert_eq!(stats.total(), 2);
        assert_eq!(stats.get_severity_count(DiagnosticSeverity::Error), 1);
        assert_eq!(stats.get_severity_count(DiagnosticSeverity::Warning), 1);
    }

    #[test]
    fn test_row_operations() {
        let row = Row::new(vec![
            Value::String("test".to_string()),
            Value::Integer(42),
            Value::Null,
        ]);

        assert_eq!(row.len(), 3);
        assert!(!row.is_empty());
        assert_eq!(row.get(0), Some(&Value::String("test".to_string())));
        assert_eq!(row.get(1), Some(&Value::Integer(42)));
        assert_eq!(row.get(3), None);
    }
}