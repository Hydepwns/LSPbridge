use super::parser::{
    CategoryFilter, Comparison, FromClause, MessageFilter, OrderDirection, PathFilter, Query,
    QueryAggregation, QueryFilter, SelectClause, SeverityFilter,
};
use crate::core::{Diagnostic, DiagnosticResult, DiagnosticSeverity};
use crate::history::HistoryStorage;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub metadata: QueryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub values: Vec<Value>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub data_source: String,
    pub filters_applied: usize,
    pub rows_scanned: usize,
    pub cache_hit: bool,
}

pub struct QueryExecutor {
    diagnostic_cache: Option<DiagnosticResult>,
    history_storage: Option<HistoryStorage>,
    query_cache: HashMap<String, CachedResult>,
}

#[derive(Clone)]
struct CachedResult {
    result: QueryResult,
    cached_at: std::time::Instant,
}

impl QueryExecutor {
    pub fn new() -> Self {
        Self {
            diagnostic_cache: None,
            history_storage: None,
            query_cache: HashMap::new(),
        }
    }

    pub fn with_diagnostics(&mut self, diagnostics: DiagnosticResult) -> &mut Self {
        self.diagnostic_cache = Some(diagnostics);
        self
    }

    pub fn with_history(&mut self, history: HistoryStorage) -> &mut Self {
        self.history_storage = Some(history);
        self
    }

    pub async fn execute(&mut self, query: &Query) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();

        // Check cache
        let cache_key = format!("{:?}", query);
        if let Some(cached) = self.query_cache.get(&cache_key) {
            if cached.cached_at.elapsed().as_secs() < 300 {
                // 5 minute cache
                let mut result = cached.result.clone();
                result.metadata.cache_hit = true;
                return Ok(result);
            }
        }

        let result = match &query.from {
            FromClause::Diagnostics => self.execute_diagnostics_query(query).await?,
            FromClause::Files => self.execute_files_query(query).await?,
            FromClause::History => self.execute_history_query(query).await?,
            FromClause::Trends => self.execute_trends_query(query).await?,
        };

        let mut final_result = result;
        final_result.query_time_ms = start_time.elapsed().as_millis() as u64;

        // Cache result
        self.query_cache.insert(
            cache_key,
            CachedResult {
                result: final_result.clone(),
                cached_at: std::time::Instant::now(),
            },
        );

        Ok(final_result)
    }

    async fn execute_diagnostics_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;

        let mut all_diagnostics = Vec::new();
        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            for diagnostic in file_diagnostics {
                all_diagnostics.push((file_path.clone(), diagnostic.clone()));
            }
        }

        // Apply filters
        let filtered = self.apply_filters(&all_diagnostics, &query.filters)?;
        let rows_scanned = all_diagnostics.len();

        // Build result based on select clause
        let (columns, rows) = match &query.select {
            SelectClause::All => {
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
                            Value::Path(file_path),
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
            SelectClause::Count => {
                let columns = vec!["count".to_string()];
                let rows = vec![Row {
                    values: vec![Value::Integer(filtered.len() as i64)],
                }];
                (columns, rows)
            }
            SelectClause::Fields(fields) => {
                let mut rows = Vec::new();
                for (file_path, diagnostic) in filtered {
                    let mut values = Vec::new();
                    for field in fields {
                        let value = match field.as_str() {
                            "file" | "path" => Value::Path(file_path.clone()),
                            "line" => Value::Integer(diagnostic.range.start.line as i64),
                            "column" => Value::Integer(diagnostic.range.start.character as i64),
                            "severity" => Value::Severity(diagnostic.severity),
                            "category" => {
                                Value::String(diagnostic.code.clone().unwrap_or_default())
                            }
                            "message" => Value::String(diagnostic.message.clone()),
                            "source" => Value::String(diagnostic.source.clone()),
                            _ => Value::Null,
                        };
                        values.push(value);
                    }
                    rows.push(Row { values });
                }
                (fields.clone(), rows)
            }
            SelectClause::Aggregations(aggs) => self.execute_aggregations(&filtered, aggs)?,
        };

        // Apply sorting
        let mut final_rows = rows;
        if let Some(order_by) = &query.order_by {
            self.apply_sorting(&mut final_rows, &columns, order_by)?;
        }

        // Apply limit
        let total_count = final_rows.len();
        if let Some(limit) = query.limit {
            final_rows.truncate(limit);
        }

        Ok(QueryResult {
            columns,
            rows: final_rows,
            total_count,
            query_time_ms: 0, // Set by caller
            metadata: QueryMetadata {
                data_source: "diagnostics".to_string(),
                filters_applied: query.filters.len(),
                rows_scanned,
                cache_hit: false,
            },
        })
    }

    async fn execute_files_query(&self, query: &Query) -> Result<QueryResult> {
        let diagnostics = self
            .diagnostic_cache
            .as_ref()
            .ok_or_else(|| anyhow!("No diagnostics loaded"))?;

        // Group diagnostics by file
        let mut file_stats: HashMap<PathBuf, FileStatistics> = HashMap::new();

        for (file_path, file_diagnostics) in &diagnostics.diagnostics {
            let mut stats = FileStatistics::default();
            for diagnostic in file_diagnostics {
                match diagnostic.severity {
                    DiagnosticSeverity::Error => stats.error_count += 1,
                    DiagnosticSeverity::Warning => stats.warning_count += 1,
                    DiagnosticSeverity::Information => stats.info_count += 1,
                    DiagnosticSeverity::Hint => stats.hint_count += 1,
                }
                stats.total_count += 1;
            }
            file_stats.insert(file_path.clone(), stats);
        }

        // Convert to list and apply filters
        let mut file_list: Vec<(PathBuf, FileStatistics)> = file_stats.into_iter().collect();
        file_list = self.apply_file_filters(file_list, &query.filters)?;

        // Build result
        let total_count = file_list.len();
        let (columns, rows) = match &query.select {
            SelectClause::All | SelectClause::Fields(_) => {
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
                            Value::Path(file_path),
                            Value::Integer(stats.error_count as i64),
                            Value::Integer(stats.warning_count as i64),
                            Value::Integer(stats.total_count as i64),
                        ],
                    });
                }

                (columns, rows)
            }
            SelectClause::Count => {
                let columns = vec!["count".to_string()];
                let rows = vec![Row {
                    values: vec![Value::Integer(total_count as i64)],
                }];
                (columns, rows)
            }
            _ => return Err(anyhow!("Unsupported select clause for files")),
        };

        Ok(QueryResult {
            columns,
            rows,
            total_count,
            query_time_ms: 0,
            metadata: QueryMetadata {
                data_source: "files".to_string(),
                filters_applied: query.filters.len(),
                rows_scanned: total_count,
                cache_hit: false,
            },
        })
    }

    async fn execute_history_query(&self, query: &Query) -> Result<QueryResult> {
        let _history = self
            .history_storage
            .as_ref()
            .ok_or_else(|| anyhow!("History storage not available"))?;

        // For now, return a placeholder
        // This would query the SQLite database based on the query filters
        Ok(QueryResult {
            columns: vec![
                "timestamp".to_string(),
                "file".to_string(),
                "errors".to_string(),
            ],
            rows: vec![],
            total_count: 0,
            query_time_ms: 0,
            metadata: QueryMetadata {
                data_source: "history".to_string(),
                filters_applied: query.filters.len(),
                rows_scanned: 0,
                cache_hit: false,
            },
        })
    }

    async fn execute_trends_query(&self, query: &Query) -> Result<QueryResult> {
        let _history = self
            .history_storage
            .as_ref()
            .ok_or_else(|| anyhow!("History storage not available"))?;

        // For now, return a placeholder
        // This would calculate trends from historical data
        Ok(QueryResult {
            columns: vec![
                "metric".to_string(),
                "value".to_string(),
                "trend".to_string(),
            ],
            rows: vec![],
            total_count: 0,
            query_time_ms: 0,
            metadata: QueryMetadata {
                data_source: "trends".to_string(),
                filters_applied: query.filters.len(),
                rows_scanned: 0,
                cache_hit: false,
            },
        })
    }

    fn apply_filters(
        &self,
        diagnostics: &[(PathBuf, Diagnostic)],
        filters: &[QueryFilter],
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        let mut result = diagnostics.to_vec();

        for filter in filters {
            result = match filter {
                QueryFilter::Path(path_filter) => self.filter_by_path(result, path_filter)?,
                QueryFilter::Severity(severity_filter) => {
                    self.filter_by_severity(result, severity_filter)?
                }
                QueryFilter::Category(category_filter) => {
                    self.filter_by_category(result, category_filter)?
                }
                QueryFilter::Message(message_filter) => {
                    self.filter_by_message(result, message_filter)?
                }
                _ => result, // Other filters not implemented yet
            };
        }

        Ok(result)
    }

    fn filter_by_path(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &PathFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        if filter.is_regex {
            let re = regex::Regex::new(&filter.pattern)?;
            Ok(diagnostics
                .into_iter()
                .filter(|(path, _)| re.is_match(path.to_str().unwrap_or("")))
                .collect())
        } else {
            Ok(diagnostics
                .into_iter()
                .filter(|(path, _)| path.to_str().unwrap_or("").contains(&filter.pattern))
                .collect())
        }
    }

    fn filter_by_severity(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &SeverityFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        Ok(diagnostics
            .into_iter()
            .filter(|(_, diagnostic)| match filter.comparison {
                Comparison::Equal => diagnostic.severity == filter.severity,
                Comparison::NotEqual => diagnostic.severity != filter.severity,
                Comparison::GreaterThan => (diagnostic.severity as u8) > (filter.severity as u8),
                Comparison::LessThan => (diagnostic.severity as u8) < (filter.severity as u8),
                Comparison::GreaterThanOrEqual => {
                    (diagnostic.severity as u8) >= (filter.severity as u8)
                }
                Comparison::LessThanOrEqual => {
                    (diagnostic.severity as u8) <= (filter.severity as u8)
                }
            })
            .collect())
    }

    fn filter_by_category(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &CategoryFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        Ok(diagnostics
            .into_iter()
            .filter(|(_, diagnostic)| {
                if let Some(code) = &diagnostic.code {
                    filter.categories.iter().any(|c| code.contains(c))
                } else {
                    false
                }
            })
            .collect())
    }

    fn filter_by_message(
        &self,
        diagnostics: Vec<(PathBuf, Diagnostic)>,
        filter: &MessageFilter,
    ) -> Result<Vec<(PathBuf, Diagnostic)>> {
        if filter.is_regex {
            let re = regex::Regex::new(&filter.pattern)?;
            Ok(diagnostics
                .into_iter()
                .filter(|(_, diagnostic)| re.is_match(&diagnostic.message))
                .collect())
        } else {
            Ok(diagnostics
                .into_iter()
                .filter(|(_, diagnostic)| diagnostic.message.contains(&filter.pattern))
                .collect())
        }
    }

    fn apply_file_filters(
        &self,
        files: Vec<(PathBuf, FileStatistics)>,
        filters: &[QueryFilter],
    ) -> Result<Vec<(PathBuf, FileStatistics)>> {
        let mut result = files;

        for filter in filters {
            result = match filter {
                QueryFilter::Path(path_filter) => {
                    if path_filter.is_regex {
                        let re = regex::Regex::new(&path_filter.pattern)?;
                        result
                            .into_iter()
                            .filter(|(path, _)| re.is_match(path.to_str().unwrap_or("")))
                            .collect()
                    } else {
                        result
                            .into_iter()
                            .filter(|(path, _)| {
                                path.to_str().unwrap_or("").contains(&path_filter.pattern)
                            })
                            .collect()
                    }
                }
                _ => result, // Other filters not applicable to files
            };
        }

        Ok(result)
    }

    fn execute_aggregations(
        &self,
        diagnostics: &[(PathBuf, Diagnostic)],
        aggregations: &[QueryAggregation],
    ) -> Result<(Vec<String>, Vec<Row>)> {
        // Simple implementation - just count for now
        let columns = vec!["count".to_string()];
        let rows = vec![Row {
            values: vec![Value::Integer(diagnostics.len() as i64)],
        }];
        Ok((columns, rows))
    }

    fn apply_sorting(
        &self,
        rows: &mut Vec<Row>,
        columns: &[String],
        order_by: &super::parser::OrderByClause,
    ) -> Result<()> {
        let column_index = columns
            .iter()
            .position(|c| c == &order_by.field)
            .ok_or_else(|| anyhow!("Unknown column: {}", order_by.field))?;

        rows.sort_by(|a, b| {
            let a_val = &a.values[column_index];
            let b_val = &b.values[column_index];

            let cmp = match (a_val, b_val) {
                (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                (Value::Number(a), Value::Number(b)) => {
                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                }
                (Value::String(a), Value::String(b)) => a.cmp(b),
                _ => std::cmp::Ordering::Equal,
            };

            match order_by.direction {
                OrderDirection::Ascending => cmp,
                OrderDirection::Descending => cmp.reverse(),
            }
        });

        Ok(())
    }
}

#[derive(Default)]
struct FileStatistics {
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    hint_count: usize,
    total_count: usize,
}

impl Value {
    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Path(p) => p.display().to_string(),
            Value::Severity(s) => format!("{:?}", s),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Position, Range};

    #[tokio::test]
    async fn test_simple_count_query() {
        let mut executor = QueryExecutor::new();

        // Create test diagnostics
        let mut diagnostics = DiagnosticResult::new();
        diagnostics.diagnostics.insert(
            PathBuf::from("test.rs"),
            vec![Diagnostic {
                id: "1".to_string(),
                file: "test.rs".to_string(),
                range: Range {
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 1,
                        character: 10,
                    },
                },
                severity: DiagnosticSeverity::Error,
                message: "Type error".to_string(),
                source: "rust".to_string(),
                code: None,
                related_information: None,
                tags: None,
                data: None,
            }],
        );

        executor.with_diagnostics(diagnostics);

        let query = Query {
            select: SelectClause::Count,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let result = executor.execute(&query).await.unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.rows.len(), 1);
    }
}
