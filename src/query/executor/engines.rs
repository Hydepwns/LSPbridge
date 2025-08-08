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

/// Engine for executing queries against symbol data
pub struct SymbolsEngine {
    #[allow(dead_code)]
    filter_engine: FilterEngine,
}

impl SymbolsEngine {
    /// Create a new symbols query engine
    pub fn new() -> Self {
        Self {
            filter_engine: FilterEngine::new(),
        }
    }

    /// Execute a query against symbol data
    pub async fn execute(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        // Extract symbols from diagnostics that reference functions, classes, etc.
        let mut symbols = Vec::new();
        
        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            for diagnostic in file_diagnostics {
                // Extract symbol information from diagnostic code and message
                if let Some(code) = &diagnostic.code {
                    // Check if this diagnostic references a symbol
                    if code.contains("function") || code.contains("class") || 
                       code.contains("struct") || code.contains("trait") ||
                       code.contains("enum") || code.contains("type") ||
                       diagnostic.message.contains("method") ||
                       diagnostic.message.contains("variable") {
                        symbols.push((file_path.clone(), diagnostic.clone()));
                    }
                }
            }
        }

        // Apply filters
        let filtered = self.filter_engine.apply_diagnostic_filters(&symbols, &query.filters)?;
        let rows_scanned = symbols.len();

        // Build result
        let (columns, rows) = match &query.select {
            SelectClause::All => self.build_all_columns_result(&filtered),
            SelectClause::Count => self.build_count_result(filtered.len()),
            SelectClause::Fields(fields) => self.build_fields_result(&filtered, fields),
            SelectClause::Aggregations(aggs) => self.build_aggregation_result(&filtered, aggs)?,
        };

        let metadata = QueryMetadata {
            data_source: "symbols".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned,
            cache_hit: false,
        };

        let total_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            total_count,
            query_time_ms: 0,
            metadata,
        })
    }

    fn build_all_columns_result(&self, filtered: &[(PathBuf, Diagnostic)]) -> (Vec<String>, Vec<Row>) {
        let columns = vec![
            "file".to_string(),
            "symbol_type".to_string(),
            "symbol_name".to_string(),
            "line".to_string(),
            "severity".to_string(),
            "message".to_string(),
        ];

        let mut rows = Vec::new();
        for (file_path, diagnostic) in filtered {
            let symbol_type = self.extract_symbol_type(diagnostic);
            let symbol_name = self.extract_symbol_name(diagnostic);
            
            rows.push(Row {
                values: vec![
                    Value::Path(file_path.clone()),
                    Value::String(symbol_type),
                    Value::String(symbol_name),
                    Value::Integer(diagnostic.range.start.line as i64),
                    Value::Severity(diagnostic.severity),
                    Value::String(diagnostic.message.clone()),
                ],
            });
        }

        (columns, rows)
    }

    fn build_count_result(&self, count: usize) -> (Vec<String>, Vec<Row>) {
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(count as i64)],
        }];
        (columns, rows)
    }

    fn build_fields_result(&self, filtered: &[(PathBuf, Diagnostic)], fields: &[String]) -> (Vec<String>, Vec<Row>) {
        let mut rows = Vec::new();
        
        for (file_path, diagnostic) in filtered {
            let mut values = Vec::new();
            for field in fields {
                match field.as_str() {
                    "file" => values.push(Value::Path(file_path.clone())),
                    "symbol_type" => values.push(Value::String(self.extract_symbol_type(diagnostic))),
                    "symbol_name" => values.push(Value::String(self.extract_symbol_name(diagnostic))),
                    "line" => values.push(Value::Integer(diagnostic.range.start.line as i64)),
                    "severity" => values.push(Value::Severity(diagnostic.severity)),
                    "message" => values.push(Value::String(diagnostic.message.clone())),
                    _ => values.push(Value::Null),
                }
            }
            rows.push(Row { values });
        }

        (fields.to_vec(), rows)
    }

    fn build_aggregation_result(&self, filtered: &[(PathBuf, Diagnostic)], aggs: &[QueryAggregation]) -> Result<(Vec<String>, Vec<Row>)> {
        let mut columns = Vec::new();
        let mut values = Vec::new();

        for agg in aggs {
            match agg {
                QueryAggregation::Count(field) => {
                    columns.push(format!("count_{}", field));
                    values.push(Value::Integer(filtered.len() as i64));
                }
                QueryAggregation::Sum(_) | 
                QueryAggregation::Average(_) |
                QueryAggregation::Min(_) |
                QueryAggregation::Max(_) => {
                    return Err(anyhow!("Aggregation not supported for symbol queries"));
                }
            }
        }

        Ok((columns, vec![Row { values }]))
    }

    fn extract_symbol_type(&self, diagnostic: &Diagnostic) -> String {
        let message = &diagnostic.message;
        let code = diagnostic.code.as_ref().map(|c| c.as_str()).unwrap_or("");
        
        if message.contains("function") || code.contains("function") {
            "function".to_string()
        } else if message.contains("class") || code.contains("class") {
            "class".to_string()
        } else if message.contains("struct") || code.contains("struct") {
            "struct".to_string()
        } else if message.contains("trait") || code.contains("trait") {
            "trait".to_string()
        } else if message.contains("enum") || code.contains("enum") {
            "enum".to_string()
        } else if message.contains("method") {
            "method".to_string()
        } else if message.contains("variable") {
            "variable".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn extract_symbol_name(&self, diagnostic: &Diagnostic) -> String {
        // Try to extract symbol name from message using common patterns
        let message = &diagnostic.message;
        
        // Common patterns: "function 'name'", "struct `name`", etc.
        if let Some(start) = message.find('`') {
            if let Some(end) = message[start+1..].find('`') {
                return message[start+1..start+1+end].to_string();
            }
        }
        
        if let Some(start) = message.find('\'') {
            if let Some(end) = message[start+1..].find('\'') {
                return message[start+1..start+1+end].to_string();
            }
        }
        
        // Fallback to code if available
        diagnostic.code.clone().unwrap_or_else(|| "unknown".to_string())
    }
}

/// Engine for executing queries against reference data
pub struct ReferencesEngine {
    #[allow(dead_code)]
    filter_engine: FilterEngine,
}

impl ReferencesEngine {
    /// Create a new references query engine
    pub fn new() -> Self {
        Self {
            filter_engine: FilterEngine::new(),
        }
    }

    /// Execute a query against reference data
    pub async fn execute(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        // Extract references from diagnostics (undefined references, missing imports, etc.)
        let mut references = Vec::new();
        
        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            for diagnostic in file_diagnostics {
                // Check if this diagnostic is about references
                if diagnostic.message.contains("undefined") ||
                   diagnostic.message.contains("not found") ||
                   diagnostic.message.contains("cannot find") ||
                   diagnostic.message.contains("unresolved") ||
                   diagnostic.message.contains("import") ||
                   diagnostic.message.contains("reference") {
                    references.push((file_path.clone(), diagnostic.clone()));
                }
            }
        }

        // Apply filters
        let filtered = self.filter_engine.apply_diagnostic_filters(&references, &query.filters)?;
        let rows_scanned = references.len();

        // Build result
        let (columns, rows) = match &query.select {
            SelectClause::All => self.build_all_columns_result(&filtered),
            SelectClause::Count => self.build_count_result(filtered.len()),
            SelectClause::Fields(fields) => self.build_fields_result(&filtered, fields),
            SelectClause::Aggregations(aggs) => self.build_aggregation_result(&filtered, aggs)?,
        };

        let metadata = QueryMetadata {
            data_source: "references".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned,
            cache_hit: false,
        };

        let total_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            total_count,
            query_time_ms: 0,
            metadata,
        })
    }

    fn build_all_columns_result(&self, filtered: &[(PathBuf, Diagnostic)]) -> (Vec<String>, Vec<Row>) {
        let columns = vec![
            "file".to_string(),
            "reference_type".to_string(),
            "reference_name".to_string(),
            "line".to_string(),
            "severity".to_string(),
            "message".to_string(),
        ];

        let mut rows = Vec::new();
        for (file_path, diagnostic) in filtered {
            let ref_type = self.extract_reference_type(diagnostic);
            let ref_name = self.extract_reference_name(diagnostic);
            
            rows.push(Row {
                values: vec![
                    Value::Path(file_path.clone()),
                    Value::String(ref_type),
                    Value::String(ref_name),
                    Value::Integer(diagnostic.range.start.line as i64),
                    Value::Severity(diagnostic.severity),
                    Value::String(diagnostic.message.clone()),
                ],
            });
        }

        (columns, rows)
    }

    fn build_count_result(&self, count: usize) -> (Vec<String>, Vec<Row>) {
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(count as i64)],
        }];
        (columns, rows)
    }

    fn build_fields_result(&self, filtered: &[(PathBuf, Diagnostic)], fields: &[String]) -> (Vec<String>, Vec<Row>) {
        let mut rows = Vec::new();
        
        for (file_path, diagnostic) in filtered {
            let mut values = Vec::new();
            for field in fields {
                match field.as_str() {
                    "file" => values.push(Value::Path(file_path.clone())),
                    "reference_type" => values.push(Value::String(self.extract_reference_type(diagnostic))),
                    "reference_name" => values.push(Value::String(self.extract_reference_name(diagnostic))),
                    "line" => values.push(Value::Integer(diagnostic.range.start.line as i64)),
                    "severity" => values.push(Value::Severity(diagnostic.severity)),
                    "message" => values.push(Value::String(diagnostic.message.clone())),
                    _ => values.push(Value::Null),
                }
            }
            rows.push(Row { values });
        }

        (fields.to_vec(), rows)
    }

    fn build_aggregation_result(&self, filtered: &[(PathBuf, Diagnostic)], aggs: &[QueryAggregation]) -> Result<(Vec<String>, Vec<Row>)> {
        let mut columns = Vec::new();
        let mut values = Vec::new();

        for agg in aggs {
            match agg {
                QueryAggregation::Count(field) => {
                    columns.push(format!("count_{}", field));
                    values.push(Value::Integer(filtered.len() as i64));
                }
                _ => {
                    return Err(anyhow!("Aggregation not supported for reference queries"));
                }
            }
        }

        Ok((columns, vec![Row { values }]))
    }

    fn extract_reference_type(&self, diagnostic: &Diagnostic) -> String {
        let message = &diagnostic.message;
        
        if message.contains("import") {
            "import".to_string()
        } else if message.contains("module") {
            "module".to_string()
        } else if message.contains("type") {
            "type".to_string()
        } else if message.contains("function") {
            "function".to_string()
        } else if message.contains("variable") {
            "variable".to_string()
        } else {
            "undefined".to_string()
        }
    }

    fn extract_reference_name(&self, diagnostic: &Diagnostic) -> String {
        let message = &diagnostic.message;
        
        // Try to extract reference name from message
        if let Some(start) = message.find('`') {
            if let Some(end) = message[start+1..].find('`') {
                return message[start+1..start+1+end].to_string();
            }
        }
        
        if let Some(start) = message.find('\'') {
            if let Some(end) = message[start+1..].find('\'') {
                return message[start+1..start+1+end].to_string();
            }
        }
        
        "unknown".to_string()
    }
}

/// Engine for executing queries against project data
pub struct ProjectsEngine {
    #[allow(dead_code)]
    filter_engine: FilterEngine,
}

impl ProjectsEngine {
    /// Create a new projects query engine
    pub fn new() -> Self {
        Self {
            filter_engine: FilterEngine::new(),
        }
    }

    /// Execute a query against project data
    pub async fn execute(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        // Group diagnostics by project/module
        let mut project_stats: HashMap<String, (usize, usize, usize)> = HashMap::new();
        
        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            // Extract project name from path (e.g., src/module_name/...)
            let project_name = self.extract_project_name(file_path);
            
            let entry = project_stats.entry(project_name).or_insert((0, 0, 0));
            entry.0 += 1; // file count
            entry.1 += file_diagnostics.len(); // diagnostic count
            
            // Count error severity
            for diagnostic in file_diagnostics {
                if diagnostic.severity == crate::core::DiagnosticSeverity::Error {
                    entry.2 += 1;
                }
            }
        }

        // Build result
        let (columns, rows) = match &query.select {
            SelectClause::All => self.build_all_columns_result(&project_stats),
            SelectClause::Count => self.build_count_result(project_stats.len()),
            SelectClause::Fields(fields) => self.build_fields_result(&project_stats, fields),
            SelectClause::Aggregations(aggs) => self.build_aggregation_result(&project_stats, aggs)?,
        };

        let metadata = QueryMetadata {
            data_source: "projects".to_string(),
            filters_applied: query.filters.len(),
            rows_scanned: project_stats.len(),
            cache_hit: false,
        };

        let total_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            total_count,
            query_time_ms: 0,
            metadata,
        })
    }

    fn build_all_columns_result(&self, stats: &HashMap<String, (usize, usize, usize)>) -> (Vec<String>, Vec<Row>) {
        let columns = vec![
            "project".to_string(),
            "file_count".to_string(),
            "diagnostic_count".to_string(),
            "error_count".to_string(),
        ];

        let mut rows = Vec::new();
        for (project, (files, diagnostics, errors)) in stats {
            rows.push(Row {
                values: vec![
                    Value::String(project.clone()),
                    Value::Integer(*files as i64),
                    Value::Integer(*diagnostics as i64),
                    Value::Integer(*errors as i64),
                ],
            });
        }

        (columns, rows)
    }

    fn build_count_result(&self, count: usize) -> (Vec<String>, Vec<Row>) {
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(count as i64)],
        }];
        (columns, rows)
    }

    fn build_fields_result(&self, stats: &HashMap<String, (usize, usize, usize)>, fields: &[String]) -> (Vec<String>, Vec<Row>) {
        let mut rows = Vec::new();
        
        for (project, (files, diagnostics, errors)) in stats {
            let mut values = Vec::new();
            for field in fields {
                match field.as_str() {
                    "project" => values.push(Value::String(project.clone())),
                    "file_count" => values.push(Value::Integer(*files as i64)),
                    "diagnostic_count" => values.push(Value::Integer(*diagnostics as i64)),
                    "error_count" => values.push(Value::Integer(*errors as i64)),
                    _ => values.push(Value::Null),
                }
            }
            rows.push(Row { values });
        }

        (fields.to_vec(), rows)
    }

    fn build_aggregation_result(&self, stats: &HashMap<String, (usize, usize, usize)>, aggs: &[QueryAggregation]) -> Result<(Vec<String>, Vec<Row>)> {
        let mut columns = Vec::new();
        let mut values = Vec::new();

        for agg in aggs {
            match agg {
                QueryAggregation::Count(field) => {
                    columns.push(format!("count_{}", field));
                    values.push(Value::Integer(stats.len() as i64));
                }
                QueryAggregation::Sum(field) => {
                    let sum = match field.as_str() {
                        "diagnostic_count" => stats.values().map(|(_, d, _)| *d as i64).sum(),
                        "error_count" => stats.values().map(|(_, _, e)| *e as i64).sum(),
                        "file_count" => stats.values().map(|(f, _, _)| *f as i64).sum(),
                        _ => 0,
                    };
                    columns.push(format!("sum_{}", field));
                    values.push(Value::Integer(sum));
                }
                _ => {
                    return Err(anyhow!("Aggregation not supported for project queries"));
                }
            }
        }

        Ok((columns, vec![Row { values }]))
    }

    fn extract_project_name(&self, path: &PathBuf) -> String {
        let path_str = path.to_string_lossy();
        
        // Try to extract module/project name from path
        if let Some(src_idx) = path_str.find("src/") {
            let after_src = &path_str[src_idx + 4..];
            if let Some(slash_idx) = after_src.find('/') {
                return after_src[..slash_idx].to_string();
            }
        }
        
        // Fallback to parent directory name
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string()
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
            FromClause::Symbols => Box::new(SymbolsEngine::new()),
            FromClause::References => Box::new(ReferencesEngine::new()),
            FromClause::Projects => Box::new(ProjectsEngine::new())
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

impl QueryEngine for SymbolsEngine {
    fn execute_diagnostics(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, diagnostics))
        })
    }
}

impl QueryEngine for ReferencesEngine {
    fn execute_diagnostics(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, diagnostics))
        })
    }
}

impl QueryEngine for ProjectsEngine {
    fn execute_diagnostics(&self, query: &Query, diagnostics: &DiagnosticResult) -> Result<QueryResult> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.execute(query, diagnostics))
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

impl Default for SymbolsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ReferencesEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ProjectsEngine {
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

    #[tokio::test]
    async fn test_symbols_engine() {
        let engine = SymbolsEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        let mut func_diagnostic = create_test_diagnostic(DiagnosticSeverity::Error, "undefined function 'foo'");
        func_diagnostic.code = Some("function".to_string());
        
        let mut class_diagnostic = create_test_diagnostic(DiagnosticSeverity::Error, "class `MyClass` not found");
        class_diagnostic.code = Some("class".to_string());
        
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![func_diagnostic, class_diagnostic],
        );

        let query = Query {
            select: SelectClause::All,
            from: FromClause::Symbols,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 2);
        assert_eq!(result.columns, vec!["file", "symbol_type", "symbol_name", "line", "severity", "message"]);
        
        // Check first symbol is a function
        assert_eq!(result.rows[0].values[1], Value::String("function".to_string()));
        assert_eq!(result.rows[0].values[2], Value::String("foo".to_string()));
        
        // Check second symbol is a class
        assert_eq!(result.rows[1].values[1], Value::String("class".to_string()));
        assert_eq!(result.rows[1].values[2], Value::String("MyClass".to_string()));
    }

    #[tokio::test]
    async fn test_symbols_engine_filtering() {
        let engine = SymbolsEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        
        // Create various symbol-related diagnostics
        let mut method_diagnostic = create_test_diagnostic(DiagnosticSeverity::Warning, "method 'calculate' is deprecated");
        method_diagnostic.code = Some("W123".to_string());
        
        let mut var_diagnostic = create_test_diagnostic(DiagnosticSeverity::Error, "variable 'count' not initialized");
        var_diagnostic.code = Some("E456".to_string());
        
        let non_symbol_diagnostic = create_test_diagnostic(DiagnosticSeverity::Error, "syntax error");
        
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![method_diagnostic, var_diagnostic, non_symbol_diagnostic],
        );

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Symbols,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        // Only method and variable diagnostics should be included
        assert_eq!(result.rows[0].values[0], Value::Integer(2));
    }

    #[tokio::test]
    async fn test_references_engine() {
        let engine = ReferencesEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        
        let undefined_ref = create_test_diagnostic(DiagnosticSeverity::Error, "undefined reference to `external_func`");
        let unresolved_import = create_test_diagnostic(DiagnosticSeverity::Error, "cannot find module 'utils'");
        let missing_import = create_test_diagnostic(DiagnosticSeverity::Warning, "missing import for type 'Config'");
        
        diagnostics.diagnostics.insert(
            PathBuf::from("main.rs"),
            vec![undefined_ref, unresolved_import, missing_import],
        );

        let query = Query {
            select: SelectClause::All,
            from: FromClause::References,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 3);
        assert_eq!(result.columns, vec!["file", "reference_type", "reference_name", "line", "severity", "message"]);
        
        // Check reference names are extracted
        assert_eq!(result.rows[0].values[2], Value::String("external_func".to_string()));
        assert_eq!(result.rows[1].values[2], Value::String("utils".to_string()));
        assert_eq!(result.rows[2].values[2], Value::String("Config".to_string()));
    }

    #[tokio::test]
    async fn test_references_engine_count() {
        let engine = ReferencesEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        
        // Mix of reference and non-reference diagnostics
        diagnostics.diagnostics.insert(
            PathBuf::from("lib.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Error, "undefined function"),
                create_test_diagnostic(DiagnosticSeverity::Error, "syntax error"),
                create_test_diagnostic(DiagnosticSeverity::Error, "unresolved import"),
            ],
        );

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::References,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        // Only undefined and unresolved should be counted
        assert_eq!(result.rows[0].values[0], Value::Integer(2));
    }

    #[tokio::test]
    async fn test_projects_engine() {
        let engine = ProjectsEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        
        // Add diagnostics for different modules/projects
        diagnostics.diagnostics.insert(
            PathBuf::from("src/parser/lexer.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Error, "Error in lexer"),
                create_test_diagnostic(DiagnosticSeverity::Warning, "Warning in lexer"),
            ],
        );
        
        diagnostics.diagnostics.insert(
            PathBuf::from("src/parser/ast.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Error, "Error in ast"),
            ],
        );
        
        diagnostics.diagnostics.insert(
            PathBuf::from("src/analyzer/type_check.rs"),
            vec![
                create_test_diagnostic(DiagnosticSeverity::Warning, "Warning in analyzer"),
            ],
        );

        let query = Query {
            select: SelectClause::All,
            from: FromClause::Projects,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 2); // parser and analyzer projects
        assert_eq!(result.columns, vec!["project", "file_count", "diagnostic_count", "error_count"]);
        
        // Verify project statistics
        let parser_row = result.rows.iter().find(|r| r.values[0] == Value::String("parser".to_string())).unwrap();
        assert_eq!(parser_row.values[1], Value::Integer(2)); // 2 files
        assert_eq!(parser_row.values[2], Value::Integer(3)); // 3 diagnostics
        assert_eq!(parser_row.values[3], Value::Integer(2)); // 2 errors
        
        let analyzer_row = result.rows.iter().find(|r| r.values[0] == Value::String("analyzer".to_string())).unwrap();
        assert_eq!(analyzer_row.values[1], Value::Integer(1)); // 1 file
        assert_eq!(analyzer_row.values[2], Value::Integer(1)); // 1 diagnostic
        assert_eq!(analyzer_row.values[3], Value::Integer(0)); // 0 errors
    }

    #[tokio::test]
    async fn test_projects_engine_aggregation() {
        let engine = ProjectsEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        
        // Add diagnostics for multiple projects
        for i in 0..3 {
            diagnostics.diagnostics.insert(
                PathBuf::from(format!("src/module{}/file.rs", i)),
                vec![
                    create_test_diagnostic(DiagnosticSeverity::Error, "Error"),
                    create_test_diagnostic(DiagnosticSeverity::Warning, "Warning"),
                ],
            );
        }

        let query = Query {
            select: SelectClause::Aggregations(vec![
                QueryAggregation::Count("*".to_string()),
                QueryAggregation::Sum("diagnostic_count".to_string()),
                QueryAggregation::Sum("error_count".to_string()),
            ]),
            from: FromClause::Projects,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.columns, vec!["count_*", "sum_diagnostic_count", "sum_error_count"]);
        assert_eq!(result.rows[0].values[0], Value::Integer(3)); // 3 projects
        assert_eq!(result.rows[0].values[1], Value::Integer(6)); // 6 total diagnostics
        assert_eq!(result.rows[0].values[2], Value::Integer(3)); // 3 total errors
    }

    #[tokio::test]
    async fn test_symbols_engine_fields_selection() {
        let engine = SymbolsEngine::new();
        
        let mut diagnostics = DiagnosticResult::new();
        let mut diagnostic = create_test_diagnostic(DiagnosticSeverity::Error, "struct `Config` has no field 'timeout'");
        diagnostic.code = Some("struct".to_string());
        
        diagnostics.diagnostics.insert(
            PathBuf::from("config.rs"),
            vec![diagnostic],
        );

        let query = Query {
            select: SelectClause::Fields(vec!["symbol_type".to_string(), "symbol_name".to_string()]),
            from: FromClause::Symbols,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = engine.execute(&query, &diagnostics).await.unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.columns, vec!["symbol_type", "symbol_name"]);
        assert_eq!(result.rows[0].values[0], Value::String("struct".to_string()));
        assert_eq!(result.rows[0].values[1], Value::String("Config".to_string()));
    }
}