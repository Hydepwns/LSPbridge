//! Error handling and validation for query parsing

use crate::core::errors::ParseError;
use super::ast::Query;
use std::collections::HashSet;

/// Query validator for semantic analysis
pub struct QueryValidator {
    valid_fields: HashSet<String>,
    valid_data_sources: HashSet<String>,
}

impl QueryValidator {
    /// Create a new query validator
    pub fn new() -> Self {
        let mut valid_fields = HashSet::new();
        
        // Common diagnostic fields
        valid_fields.insert("path".to_string());
        valid_fields.insert("severity".to_string());
        valid_fields.insert("message".to_string());
        valid_fields.insert("category".to_string());
        valid_fields.insert("line".to_string());
        valid_fields.insert("column".to_string());
        valid_fields.insert("source".to_string());
        valid_fields.insert("timestamp".to_string());
        valid_fields.insert("file_count".to_string());
        valid_fields.insert("files".to_string());
        
        // File-related fields
        valid_fields.insert("file_path".to_string());
        valid_fields.insert("file_size".to_string());
        valid_fields.insert("file_type".to_string());
        valid_fields.insert("language".to_string());
        
        // Time-related fields
        valid_fields.insert("time".to_string());
        valid_fields.insert("created_at".to_string());
        valid_fields.insert("updated_at".to_string());

        let mut valid_data_sources = HashSet::new();
        valid_data_sources.insert("diagnostics".to_string());
        valid_data_sources.insert("files".to_string());
        valid_data_sources.insert("history".to_string());
        valid_data_sources.insert("trends".to_string());

        Self {
            valid_fields,
            valid_data_sources,
        }
    }

    /// Validate a parsed query for semantic correctness
    pub fn validate(&self, query: &Query) -> Result<(), Vec<ParseError>> {
        let mut errors = Vec::new();

        // Validate data source compatibility
        if let Err(error) = self.validate_data_source_compatibility(query) {
            errors.push(error);
        }

        // Validate field names
        if let Err(mut field_errors) = self.validate_field_names(query) {
            errors.append(&mut field_errors);
        }

        // Validate aggregation usage
        if let Err(error) = self.validate_aggregations(query) {
            errors.push(error);
        }

        // Validate time range constraints
        if let Err(error) = self.validate_time_ranges(query) {
            errors.push(error);
        }

        // Validate limit constraints
        if let Err(error) = self.validate_limits(query) {
            errors.push(error);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate data source compatibility with selected fields
    fn validate_data_source_compatibility(&self, query: &Query) -> Result<(), ParseError> {
        use super::ast::{FromClause, SelectClause};

        match (&query.from, &query.select) {
            (FromClause::Trends, SelectClause::Fields(fields)) => {
                // Trends data source requires specific fields
                for field in fields {
                    if !matches!(field.as_str(), "timestamp" | "count" | "category" | "trend") {
                        return Err(ParseError::IncompatibleDataSource {
                            data_source: "trends".to_string(),
                            field: field.clone(),
                            reason: "Trends data source only supports timestamp, count, category, and trend fields".to_string(),
                        });
                    }
                }
            }
            (FromClause::Files, SelectClause::Fields(fields)) => {
                // Files data source validation
                for field in fields {
                    if field.starts_with("message") || field.starts_with("severity") {
                        return Err(ParseError::IncompatibleDataSource {
                            data_source: "files".to_string(),
                            field: field.clone(),
                            reason: "Files data source does not support diagnostic-specific fields".to_string(),
                        });
                    }
                }
            }
            _ => {} // Other combinations are valid
        }

        Ok(())
    }

    /// Validate field names against known schema
    fn validate_field_names(&self, query: &Query) -> Result<(), Vec<ParseError>> {
        let mut errors = Vec::new();

        // Check SELECT clause fields
        if let super::ast::SelectClause::Fields(fields) = &query.select {
            for field in fields {
                if !self.valid_fields.contains(field) {
                    errors.push(ParseError::UnknownField {
                        field: field.clone(),
                        available_fields: self.valid_fields.iter().cloned().collect(),
                    });
                }
            }
        }

        // Check GROUP BY fields
        if let Some(group_by) = &query.group_by {
            for field in &group_by.fields {
                if !self.valid_fields.contains(field) {
                    errors.push(ParseError::UnknownField {
                        field: field.clone(),
                        available_fields: self.valid_fields.iter().cloned().collect(),
                    });
                }
            }
        }

        // Check ORDER BY field
        if let Some(order_by) = &query.order_by {
            if !self.valid_fields.contains(&order_by.field) {
                errors.push(ParseError::UnknownField {
                    field: order_by.field.clone(),
                    available_fields: self.valid_fields.iter().cloned().collect(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate aggregation function usage
    fn validate_aggregations(&self, query: &Query) -> Result<(), ParseError> {
        use super::ast::{SelectClause, QueryAggregation};

        if let SelectClause::Aggregations(aggregations) = &query.select {
            for aggregation in aggregations {
                match aggregation {
                    QueryAggregation::Sum(field) | 
                    QueryAggregation::Average(field) | 
                    QueryAggregation::Min(field) | 
                    QueryAggregation::Max(field) => {
                        if field != "*" && !self.is_numeric_field(field) {
                            return Err(ParseError::InvalidAggregation {
                                function: format!("{:?}", aggregation),
                                field: field.clone(),
                                reason: "Aggregation function can only be applied to numeric fields".to_string(),
                            });
                        }
                    }
                    QueryAggregation::Count(_) => {
                        // COUNT is valid on any field
                    }
                }
            }

            // If using aggregations, GROUP BY might be required
            if aggregations.len() > 1 && query.group_by.is_none() {
                return Err(ParseError::MissingGroupBy {
                    reason: "Multiple aggregations require GROUP BY clause".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate time range constraints
    fn validate_time_ranges(&self, query: &Query) -> Result<(), ParseError> {
        if let Some(time_range) = &query.time_range {
            // Check for conflicting time specifications
            if time_range.start.is_some() && time_range.relative.is_some() {
                return Err(ParseError::ConflictingTimeRange {
                    reason: "Cannot specify both absolute and relative time ranges".to_string(),
                });
            }

            // Check for valid relative time values
            if let Some(relative) = &time_range.relative {
                use super::ast::RelativeTime;
                match relative {
                    RelativeTime::LastHours(hours) if *hours == 0 => {
                        return Err(ParseError::InvalidTimeRange {
                            reason: "Time range cannot be zero hours".to_string(),
                        });
                    }
                    RelativeTime::LastDays(days) if *days == 0 => {
                        return Err(ParseError::InvalidTimeRange {
                            reason: "Time range cannot be zero days".to_string(),
                        });
                    }
                    RelativeTime::LastWeeks(weeks) if *weeks == 0 => {
                        return Err(ParseError::InvalidTimeRange {
                            reason: "Time range cannot be zero weeks".to_string(),
                        });
                    }
                    RelativeTime::LastHours(hours) if *hours > 8760 => {
                        return Err(ParseError::InvalidTimeRange {
                            reason: "Time range cannot exceed 1 year (8760 hours)".to_string(),
                        });
                    }
                    _ => {} // Valid
                }
            }

            // Check absolute time range ordering
            if let (Some(start), Some(end)) = (&time_range.start, &time_range.end) {
                if start >= end {
                    return Err(ParseError::InvalidTimeRange {
                        reason: "Start time must be before end time".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate LIMIT constraints
    fn validate_limits(&self, query: &Query) -> Result<(), ParseError> {
        if let Some(limit) = query.limit {
            if limit == 0 {
                return Err(ParseError::InvalidLimit {
                    limit,
                    reason: "LIMIT cannot be zero".to_string(),
                });
            }

            if limit > 10000 {
                return Err(ParseError::InvalidLimit {
                    limit,
                    reason: "LIMIT cannot exceed 10,000 for performance reasons".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check if a field is numeric (for aggregation validation)
    fn is_numeric_field(&self, field: &str) -> bool {
        matches!(
            field,
            "line" | "column" | "file_size" | "file_count" | "count" | "duration" | "size"
        )
    }

    /// Add a custom field to the validator
    pub fn add_valid_field(&mut self, field: String) {
        self.valid_fields.insert(field);
    }

    /// Get all valid fields
    pub fn get_valid_fields(&self) -> &HashSet<String> {
        &self.valid_fields
    }
}

impl Default for QueryValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Query optimization hints and suggestions
pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Analyze a query and provide optimization suggestions
    pub fn analyze(query: &Query) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest adding LIMIT for performance
        if query.limit.is_none() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: SuggestionType::Performance,
                message: "Consider adding a LIMIT clause to improve query performance".to_string(),
                severity: SuggestionSeverity::Info,
            });
        }

        // Suggest using COUNT instead of SELECT * when only counting
        if matches!(query.select, super::ast::SelectClause::All) && query.group_by.is_some() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: SuggestionType::Performance,
                message: "Consider using COUNT(*) instead of SELECT * with GROUP BY".to_string(),
                severity: SuggestionSeverity::Warning,
            });
        }

        // Suggest time range filters for better performance
        if query.time_range.is_none() && matches!(query.from, super::ast::FromClause::History) {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: SuggestionType::Performance,
                message: "Consider adding a time range filter when querying history data".to_string(),
                severity: SuggestionSeverity::Info,
            });
        }

        // Suggest specific fields instead of SELECT *
        if matches!(query.select, super::ast::SelectClause::All) {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: SuggestionType::Performance,
                message: "Consider selecting specific fields instead of * for better performance".to_string(),
                severity: SuggestionSeverity::Info,
            });
        }

        suggestions
    }
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    pub suggestion_type: SuggestionType,
    pub message: String,
    pub severity: SuggestionSeverity,
}

/// Types of optimization suggestions
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionType {
    Performance,
    Correctness,
    Style,
}

/// Severity of suggestions
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionSeverity {
    Error,
    Warning,
    Info,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::ast::*;

    #[test]
    fn test_validator_valid_query() {
        let validator = QueryValidator::new();
        
        let query = Query {
            select: SelectClause::Fields(vec!["path".to_string(), "severity".to_string()]),
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: Some(100),
            time_range: None,
        };

        assert!(validator.validate(&query).is_ok());
    }

    #[test]
    fn test_validator_invalid_field() {
        let validator = QueryValidator::new();
        
        let query = Query {
            select: SelectClause::Fields(vec!["invalid_field".to_string()]),
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        assert!(validator.validate(&query).is_err());
    }

    #[test]
    fn test_validator_zero_limit() {
        let validator = QueryValidator::new();
        
        let query = Query {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: Some(0),
            time_range: None,
        };

        assert!(validator.validate(&query).is_err());
    }

    #[test]
    fn test_optimizer_suggestions() {
        let query = Query {
            select: SelectClause::All,
            from: FromClause::History,
            filters: vec![],
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        };

        let suggestions = QueryOptimizer::analyze(&query);
        assert!(!suggestions.is_empty());
        
        // Should suggest adding LIMIT and time range
        assert!(suggestions.iter().any(|s| s.message.contains("LIMIT")));
        assert!(suggestions.iter().any(|s| s.message.contains("time range")));
    }
}