//! Query execution engines for different data sources
//!
//! This module provides specialized execution engines for each data source type:
//! diagnostics, files, history, and trends. Each engine knows how to query its
//! specific data source and convert results to the common QueryResult format.

use super::filters::FilterEngine;
use crate::query::parser::{FromClause, Query, SelectClause, QueryAggregation};
use super::types::{FileStatistics, QueryMetadata, QueryResult, Row, Value};
use crate::core::{Diagnostic, DiagnosticResult};
use crate::history::HistoryStorage;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

/// Engine for executing queries against diagnostic data
pub struct DiagnosticsEngine {
    filter_engine: FilterEngine,
}

impl DiagnosticsEngine {
    /// Create a new diagnostics query engine
    pub fn new() -> Self {
        Self {
            filter_engine: FilterEngine::new(),
        }
    }

    /// Execute a query against diagnostic data
    pub async fn execute(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        // Convert diagnostics to a flat list
        let mut all_diagnostics = Vec::new();
        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            for diagnostic in file_diagnostics {
                all_diagnostics.push((file_path.clone(), diagnostic.clone()));
            }
        }

        // Apply filters
        let filtered = self.filter_engine.apply_diagnostic_filters(&all_diagnostics, &query.filters)?;
        let rows_scanned = all_diagnostics.len();

        // Build result based on select clause
        let (columns, rows) = match &query.select {
            SelectClause::All => self.build_all_columns_result(&filtered),
            SelectClause::Count => self.build_count_result(filtered.len()),
            SelectClause::Fields(fields) => self.build_fields_result(&filtered, fields),
            SelectClause::Aggregations(aggs) => self.build_aggregation_result(&filtered, aggs)?,
        };

        let total_count = rows.len();
        let metadata = QueryMetadata {
            data_source: "diagnostics".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned,
            cache_hit: false,
        };

        Ok(QueryResult {
            columns,
            rows,
            total_count,
            query_time_ms: 0, // Set by caller
            metadata,
        })
    }

    /// Build result with all diagnostic columns
    fn build_all_columns_result(&self, filtered: &[(PathBuf, Diagnostic)]) -> (Vec<String>, Vec<Row>) {
        let columns = vec![
            "file".to_string(),
            "line".to_string(),
            "column".to_string(),
            "severity".to_string(),
            "category".to_string(),
            "message".to_string(),
        ];

        let mut rows = Vec::new();
        for (file_path, diagnostic) in filtered {
            rows.push(Row {
                values: vec![
                    Value::Path(file_path.clone()),
                    Value::Integer(diagnostic.range.start.line as i64),
                    Value::Integer(diagnostic.range.start.character as i64),
                    Value::Severity(diagnostic.severity),
                    Value::String(diagnostic.code.clone().unwrap_or_default()),
                    Value::String(diagnostic.message.clone()),
                ],
            });
        }

        (columns, rows)
    }

    /// Build count result
    fn build_count_result(&self, count: usize) -> (Vec<String>, Vec<Row>) {
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(count as i64)],
        }];
        (columns, rows)
    }

    /// Build result with specific fields
    fn build_fields_result(&self, filtered: &[(PathBuf, Diagnostic)], fields: &[String]) -> (Vec<String>, Vec<Row>) {
        let mut rows = Vec::new();
        for (file_path, diagnostic) in filtered {
            let mut values = Vec::new();
            for field in fields {
                let value = self.extract_diagnostic_field(file_path, diagnostic, field);
                values.push(value);
            }
            rows.push(Row { values });
        }
        (fields.to_vec(), rows)
    }

    /// Build aggregation result
    fn build_aggregation_result(&self, filtered: &[(PathBuf, Diagnostic)], _aggs: &[QueryAggregation]) -> Result<(Vec<String>, Vec<Row>)> {
        // Simple implementation - just count for now
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(filtered.len() as i64)],
        }];
        Ok((columns, rows))
    }

    /// Extract a specific field value from a diagnostic
    fn extract_diagnostic_field(&self, file_path: &PathBuf, diagnostic: &Diagnostic, field: &str) -> Value {
        match field {
            "file" | "path" => Value::Path(file_path.clone()),
            "line" => Value::Integer(diagnostic.range.start.line as i64),
            "column" => Value::Integer(diagnostic.range.start.character as i64),
            "severity" => Value::Severity(diagnostic.severity),
            "category" => Value::String(diagnostic.code.clone().unwrap_or_default()),
            "message" => Value::String(diagnostic.message.clone()),
            "source" => Value::String(diagnostic.source.clone()),
            _ => Value::Null,
        }
    }
}

/// Engine for executing queries against file statistics
pub struct FilesEngine {
    filter_engine: FilterEngine,
}

impl FilesEngine {
    /// Create a new files query engine
    pub fn new() -> Self {
        Self {
            filter_engine: FilterEngine::new(),
        }
    }

    /// Execute a query against file data
    pub async fn execute(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        // Group diagnostics by file to create statistics
        let mut file_stats: HashMap<PathBuf, FileStatistics> = HashMap::new();

        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            let mut stats = FileStatistics::new();
            for diagnostic in file_diagnostics {
                stats.increment_severity(diagnostic.severity);
            }
            file_stats.insert(file_path.clone(), stats);
        }

        // Convert to list and apply filters
        let mut file_list: Vec<(PathBuf, FileStatistics)> = file_stats.into_iter().collect();
        file_list = self.filter_engine.apply_file_filters(file_list, &query.filters)?;

        // Build result
        let total_count = file_list.len();
        let (columns, rows) = match &query.select {
            SelectClause::All | SelectClause::Fields(_) => self.build_file_stats_result(&file_list),
            SelectClause::Count => self.build_count_result(total_count),
            _ => return Err(anyhow!("Unsupported select clause for files")),
        };

        let metadata = QueryMetadata {
            data_source: "files".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned: total_count,
            cache_hit: false,
        };

        Ok(QueryResult {
            columns,
            rows,
            total_count,
            query_time_ms: 0,
            metadata,
        })
    }

    /// Build file statistics result
    fn build_file_stats_result(&self, file_list: &[(PathBuf, FileStatistics)]) -> (Vec<String>, Vec<Row>) {
        let columns = vec![
            "file".to_string(),
            "errors".to_string(),
            "warnings".to_string(),
            "total".to_string(),
        ];

        let mut rows = Vec::new();
        for (file_path, stats) in file_list {
            rows.push(Row {
                values: vec![
                    Value::Path(file_path.clone()),
                    Value::Integer(stats.error_count as i64),
                    Value::Integer(stats.warning_count as i64),
                    Value::Integer(stats.total_count as i64),
                ],
            });
        }

        (columns, rows)
    }

    /// Build count result
    fn build_count_result(&self, count: usize) -> (Vec<String>, Vec<Row>) {
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(count as i64)],
        }];
        (columns, rows)
    }
}

/// Engine for executing queries against historical data
pub struct HistoryEngine;

impl HistoryEngine {
    /// Create a new history query engine
    pub fn new() -> Self {
        Self
    }

    /// Execute a query against historical data
    pub async fn execute(&self, query: &Query, _history: &HistoryStorage) -> Result<QueryResult> {
        // For now, return a placeholder
        // This would query the SQLite database based on the query filters
        let metadata = QueryMetadata {
            data_source: "history".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned: 0,
            cache_hit: false,
        };

        Ok(QueryResult {
            columns: vec![
                "timestamp".to_string(),
                "file".to_string(),
                "errors".to_string(),
            ],
            rows: vec![],
            total_count: 0,
            query_time_ms: 0,
            metadata,
        })
    }
}

/// Engine for executing queries against trend data
pub struct TrendsEngine;

impl TrendsEngine {
    /// Create a new trends query engine
    pub fn new() -> Self {
        Self
    }

    /// Execute a query against trend data
    pub async fn execute(&self, query: &Query, _history: &HistoryStorage) -> Result<QueryResult> {
        // For now, return a placeholder
        // This would calculate trends from historical data
        let metadata = QueryMetadata {
            data_source: "trends".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned: 0,
            cache_hit: false,
        };

        Ok(QueryResult {
            columns: vec![
                "metric".to_string(),
                "value".to_string(),
                "trend".to_string(),
            ],
            rows: vec![],
            total_count: 0,
            query_time_ms: 0,
            metadata,
        })
    }
}

/// Factory for creating appropriate execution engines
pub struct EngineFactory;

impl EngineFactory {
    /// Create an execution engine for the given data source
    pub fn create_engine(data_source: &FromClause) -> Box<dyn QueryEngine> {
        match data_source {
            FromClause::Diagnostics => Box::new(DiagnosticsEngine::new()),
            FromClause::Files => Box::new(FilesEngine::new()),
            FromClause::History => Box::new(HistoryEngine::new()),
            FromClause::Trends => Box::new(TrendsEngine::new()),
        }
    }
}

/// Trait for query execution engines
pub trait QueryEngine {
    /// Execute a query and return results
    fn execute_diagnostics(&self, _query: &Query, _diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        Err(anyhow!("Diagnostics execution not supported by this engine"))
    }

    /// Execute a query against history storage
    fn execute_history(&self, _query: &Query, _history: &HistoryStorage) -> Result<QueryResult> {
        Err(anyhow!("History execution not supported by this engine"))
    }
}

// Implement QueryEngine for each engine type
impl QueryEngine for DiagnosticsEngine {
    fn execute_diagnostics(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        // Use async runtime for the sync trait
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, diagnostics))
        })
    }
}

impl QueryEngine for FilesEngine {
    fn execute_diagnostics(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, diagnostics))
        })
    }
}

impl QueryEngine for HistoryEngine {
    fn execute_history(&self, query: &Query, history: &HistoryStorage) -> Result<QueryResult> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, history))
        })
    }
}

impl QueryEngine for TrendsEngine {
    fn execute_history(&self, query: &Query, history: &HistoryStorage) -> Result<QueryResult> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, history))
        })
    }
}

impl Default for DiagnosticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for FilesEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for HistoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TrendsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Position, Range, DiagnosticSeverity};

    fn create_test_diagnostic(severity: DiagnosticSeverity, message: &str) -> Diagnostic {
        Diagnostic {
            id: "1".to_string(),
            file: "test.rs".to_string(),
            range: Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 10 },
            },
            severity,
            message: message.to_string(),
            source: "rust".to_string(),
            code: None,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    #[tokio::test]
    async fn test_diagnostics_engine_count() {
        let engine = DiagnosticsEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Error, "Error 1"),
                create_test_diagnostic(DiagnosticSeverity::Warning, "Warning 1"),
            ],
        );

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.rows[0].values[0], Value::Integer(2));
    }

    #[tokio::test]
    async fn test_files_engine() {
        let engine = FilesEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test1.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Error, "Error 1"),
                create_test_diagnostic(DiagnosticSeverity::Warning, "Warning 1"),
            ],
        );
        diagnostics.diagnostics.insert(
            PathBuf::from("test2.rs"),
            vec![create_test_diagnostic(DiagnosticSeverity::Error, "Error 2")],
        );

        let query = Query {
            select: SelectClause::All,
            from: FromClause::Files,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 2);
        assert_eq!(result.columns, vec!["file", "errors", "warnings", "total"]);
    }
}