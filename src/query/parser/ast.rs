//! Abstract Syntax Tree (AST) definitions for query language

use crate::core::DiagnosticSeverity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Root query structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub select: SelectClause,
    pub from: FromClause,
    pub filters: Vec<QueryFilter>,
    pub group_by: Option<GroupByClause>,
    pub order_by: Option<OrderByClause>,
    pub limit: Option<u32>,
    pub time_range: Option<TimeRange>,
}

/// SELECT clause variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectClause {
    /// SELECT *
    All,
    /// SELECT COUNT(*)
    Count,
    /// SELECT field1, field2, ...
    Fields(Vec<String>),
    /// SELECT aggregation functions
    Aggregations(Vec<QueryAggregation>),
}

/// FROM clause data sources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FromClause {
    /// FROM diagnostics
    Diagnostics,
    /// FROM files
    Files,
    /// FROM symbols
    Symbols,
    /// FROM references
    References,
    /// FROM projects
    Projects,
    /// FROM history
    History,
    /// FROM trends
    Trends,
}

/// Query filter types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryFilter {
    /// File path filter
    Path(PathFilter),
    /// File name filter (alias for Path)
    File(FileFilter),
    /// Symbol filter
    Symbol(SymbolFilter),
    /// Diagnostic severity filter
    Severity(SeverityFilter),
    /// Category filter
    Category(CategoryFilter),
    /// Message pattern filter
    Message(MessageFilter),
    /// Time range filter
    TimeRange(TimeRange),
    /// File count comparison
    FileCount(ComparisonFilter),
    /// Custom field filter
    Custom(String, String), // field, value
}

/// Path-based filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathFilter {
    pub pattern: String,
    pub is_regex: bool,
}

/// Severity-based filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeverityFilter {
    pub severity: DiagnosticSeverity,
    pub comparison: Comparison,
}

/// Category-based filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryFilter {
    pub categories: Vec<String>,
}

/// Message pattern filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFilter {
    pub pattern: String,
    pub is_regex: bool,
}

/// File-based filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileFilter {
    pub pattern: String,
}

/// Symbol-based filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolFilter {
    pub pattern: String,
}

/// Time range specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub relative: Option<RelativeTime>,
}

/// Relative time specifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelativeTime {
    /// Last N hours
    LastHours(u32),
    /// Last N days
    LastDays(u32),
    /// Last N weeks
    LastWeeks(u32),
    /// Since last commit
    LastCommit,
    /// Since specific commit
    SinceCommit(String),
}

/// Comparison filter for numeric values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonFilter {
    pub field: String,
    pub comparison: Comparison,
    pub value: f64,
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Comparison {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Query aggregation functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryAggregation {
    Count(String),         // COUNT(field)
    Sum(String),           // SUM(field)
    Average(String),       // AVG(field)
    Min(String),           // MIN(field)
    Max(String),           // MAX(field)
}

/// GROUP BY clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupByClause {
    pub fields: Vec<String>,
}

/// ORDER BY clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByClause {
    pub field: String,
    pub direction: OrderDirection,
}

/// Sort order direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

impl Query {
    /// Create a new empty query
    pub fn new() -> Self {
        Self {
            select: SelectClause::All,
            from: FromClause::Diagnostics,
            filters: Vec::new(),
            group_by: None,
            order_by: None,
            limit: None,
            time_range: None,
        }
    }

    /// Add a filter to the query
    pub fn add_filter(mut self, filter: QueryFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Set the SELECT clause
    pub fn select(mut self, select: SelectClause) -> Self {
        self.select = select;
        self
    }

    /// Set the FROM clause
    pub fn from(mut self, from: FromClause) -> Self {
        self.from = from;
        self
    }

    /// Set the LIMIT
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the ORDER BY clause
    pub fn order_by(mut self, field: String, direction: OrderDirection) -> Self {
        self.order_by = Some(OrderByClause { field, direction });
        self
    }

    /// Set the GROUP BY clause
    pub fn group_by(mut self, fields: Vec<String>) -> Self {
        self.group_by = Some(GroupByClause { fields });
        self
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

impl PathFilter {
    /// Create a new path filter with exact matching
    pub fn exact(pattern: String) -> Self {
        Self {
            pattern,
            is_regex: false,
        }
    }

    /// Create a new path filter with regex matching
    pub fn regex(pattern: String) -> Self {
        Self {
            pattern,
            is_regex: true,
        }
    }
}

impl MessageFilter {
    /// Create a new message filter with exact matching
    pub fn exact(pattern: String) -> Self {
        Self {
            pattern,
            is_regex: false,
        }
    }

    /// Create a new message filter with regex matching
    pub fn regex(pattern: String) -> Self {
        Self {
            pattern,
            is_regex: true,
        }
    }
}

impl TimeRange {
    /// Create a time range from absolute start and end times
    pub fn absolute(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Self {
        Self {
            start,
            end,
            relative: None,
        }
    }

    /// Create a relative time range
    pub fn relative(relative: RelativeTime) -> Self {
        Self {
            start: None,
            end: None,
            relative: Some(relative),
        }
    }

    /// Create a time range for the last N hours
    pub fn last_hours(hours: u32) -> Self {
        Self::relative(RelativeTime::LastHours(hours))
    }

    /// Create a time range for the last N days
    pub fn last_days(days: u32) -> Self {
        Self::relative(RelativeTime::LastDays(days))
    }

    /// Create a time range since a specific datetime
    pub fn since(datetime: DateTime<Utc>) -> Self {
        Self {
            start: Some(datetime),
            end: None,
            relative: None,
        }
    }

    /// Create a time range before a specific datetime
    pub fn before(datetime: DateTime<Utc>) -> Self {
        Self {
            start: None,
            end: Some(datetime),
            relative: None,
        }
    }

    /// Create a time range after a specific datetime
    pub fn after(datetime: DateTime<Utc>) -> Self {
        Self {
            start: Some(datetime),
            end: None,
            relative: None,
        }
    }
}