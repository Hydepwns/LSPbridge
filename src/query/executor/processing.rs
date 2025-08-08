//! Data processing utilities for aggregation and sorting
//!
//! This module provides utilities for processing query results, including
//! aggregation functions, sorting operations, and data transformation.

use crate::query::parser::{OrderByClause, OrderDirection, QueryAggregation};
use super::types::{Row, Value};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Processor for aggregating data based on query specifications
pub struct AggregationProcessor;

impl AggregationProcessor {
    /// Execute aggregation functions on the given data
    pub fn execute_aggregations(
        data: &[(String, Vec<Value>)], // (group_key, values)
        aggregations: &[QueryAggregation],
    ) -> Result<(Vec<String>, Vec<Row>)> {
        let mut columns = Vec::new();
        let mut result_rows = Vec::new();

        if data.is_empty() {
            return Ok((columns, result_rows));
        }

        // Build column names from aggregations
        for agg in aggregations {
            columns.push(Self::aggregation_column_name(agg));
        }

        // Process each group
        for (_group_key, values) in data {
            let mut row_values = Vec::new();
            
            for agg in aggregations {
                let agg_value = Self::compute_aggregation(agg, values)?;
                row_values.push(agg_value);
            }
            
            result_rows.push(Row::new(row_values));
        }

        Ok((columns, result_rows))
    }

    /// Compute a single aggregation on a set of values
    fn compute_aggregation(agg: &QueryAggregation, values: &[Value]) -> Result<Value> {
        match agg {
            QueryAggregation::Count(_) => Ok(Value::Integer(values.len() as i64)),
            QueryAggregation::Sum(field) => Self::compute_sum(values, field),
            QueryAggregation::Average(field) => Self::compute_average(values, field),
            QueryAggregation::Min(field) => Self::compute_min(values, field),
            QueryAggregation::Max(field) => Self::compute_max(values, field),
        }
    }

    /// Get column name for aggregation
    fn aggregation_column_name(agg: &QueryAggregation) -> String {
        match agg {
            QueryAggregation::Count(field) => {
                if field == "*" {
                    "count".to_string()
                } else {
                    format!("count_{field}")
                }
            }
            QueryAggregation::Sum(field) => format!("sum_{field}"),
            QueryAggregation::Average(field) => format!("avg_{field}"),
            QueryAggregation::Min(field) => format!("min_{field}"),
            QueryAggregation::Max(field) => format!("max_{field}"),
        }
    }

    /// Compute sum of numeric values
    fn compute_sum(values: &[Value], _field: &str) -> Result<Value> {
        let mut sum = 0.0;
        let mut has_values = false;

        for value in values {
            if let Some(num) = value.as_number() {
                sum += num;
                has_values = true;
            }
        }

        if has_values {
            Ok(Value::Number(sum))
        } else {
            Ok(Value::Null)
        }
    }

    /// Compute average of numeric values
    fn compute_average(values: &[Value], _field: &str) -> Result<Value> {
        let mut sum = 0.0;
        let mut count = 0;

        for value in values {
            if let Some(num) = value.as_number() {
                sum += num;
                count += 1;
            }
        }

        if count > 0 {
            Ok(Value::Number(sum / count as f64))
        } else {
            Ok(Value::Null)
        }
    }

    /// Compute minimum value
    fn compute_min(values: &[Value], _field: &str) -> Result<Value> {
        let mut min_val: Option<&Value> = None;

        for value in values {
            match min_val {
                None => min_val = Some(value),
                Some(current_min) => {
                    if Self::is_less_than(value, current_min) {
                        min_val = Some(value);
                    }
                }
            }
        }

        Ok(min_val.cloned().unwrap_or(Value::Null))
    }

    /// Compute maximum value
    fn compute_max(values: &[Value], _field: &str) -> Result<Value> {
        let mut max_val: Option<&Value> = None;

        for value in values {
            match max_val {
                None => max_val = Some(value),
                Some(current_max) => {
                    if Self::is_greater_than(value, current_max) {
                        max_val = Some(value);
                    }
                }
            }
        }

        Ok(max_val.cloned().unwrap_or(Value::Null))
    }

    /// Compare if left value is less than right value
    fn is_less_than(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a < b,
            (Value::Number(a), Value::Number(b)) => a < b,
            (Value::Integer(a), Value::Number(b)) => (*a as f64) < *b,
            (Value::Number(a), Value::Integer(b)) => *a < (*b as f64),
            (Value::String(a), Value::String(b)) => a < b,
            _ => false,
        }
    }

    /// Compare if left value is greater than right value
    fn is_greater_than(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a > b,
            (Value::Number(a), Value::Number(b)) => a > b,
            (Value::Integer(a), Value::Number(b)) => (*a as f64) > *b,
            (Value::Number(a), Value::Integer(b)) => *a > (*b as f64),
            (Value::String(a), Value::String(b)) => a > b,
            _ => false,
        }
    }
}

/// Processor for sorting query results
pub struct SortingProcessor;

impl SortingProcessor {
    /// Apply sorting to query result rows
    pub fn apply_sorting(
        rows: &mut Vec<Row>,
        columns: &[String],
        order_by: &OrderByClause,
    ) -> Result<()> {
        let column_index = columns
            .iter()
            .position(|c| c == &order_by.field)
            .ok_or_else(|| anyhow!("Unknown column: {}", order_by.field))?;

        rows.sort_by(|a, b| {
            let a_val = a.get(column_index).unwrap_or(&Value::Null);
            let b_val = b.get(column_index).unwrap_or(&Value::Null);

            let cmp = Self::compare_values(a_val, b_val);

            match order_by.direction {
                OrderDirection::Ascending => cmp,
                OrderDirection::Descending => cmp.reverse(),
            }
        });

        Ok(())
    }

    /// Compare two values for sorting
    fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (a, b) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Number(a), Value::Number(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (Value::Integer(a), Value::Number(b)) => {
                (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (Value::Number(a), Value::Integer(b)) => {
                a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal)
            }
            
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Path(a), Value::Path(b)) => a.cmp(b),
            (Value::String(a), Value::Path(b)) => a.cmp(&b.to_string_lossy().to_string()),
            (Value::Path(a), Value::String(b)) => a.to_string_lossy().as_ref().cmp(b),
            
            (Value::Boolean(a), Value::Boolean(b)) => a.cmp(b),
            (Value::Severity(a), Value::Severity(b)) => (*a as u8).cmp(&(*b as u8)),
            
            (Value::Array(a), Value::Array(b)) => {
                // Compare arrays by length first, then by first differing element
                match a.len().cmp(&b.len()) {
                    Ordering::Equal => {
                        for (av, bv) in a.iter().zip(b.iter()) {
                            match Self::compare_values(av, bv) {
                                Ordering::Equal => continue,
                                other => return other,
                            }
                        }
                        Ordering::Equal
                    }
                    other => other,
                }
            }
            
            // For mixed types, use string representation
            _ => a.to_string().cmp(&b.to_string()),
        }
    }

    /// Sort rows by multiple columns
    pub fn apply_multi_column_sorting(
        rows: &mut Vec<Row>,
        columns: &[String],
        sort_specs: &[(String, OrderDirection)],
    ) -> Result<()> {
        // Get column indices for all sort specifications
        let mut sort_indices = Vec::new();
        for (field, direction) in sort_specs {
            let index = columns
                .iter()
                .position(|c| c == field)
                .ok_or_else(|| anyhow!("Unknown column: {}", field))?;
            sort_indices.push((index, direction.clone()));
        }

        rows.sort_by(|a, b| {
            for (column_index, direction) in &sort_indices {
                let a_val = a.get(*column_index).unwrap_or(&Value::Null);
                let b_val = b.get(*column_index).unwrap_or(&Value::Null);

                let cmp = Self::compare_values(a_val, b_val);
                let final_cmp = match direction {
                    OrderDirection::Ascending => cmp,
                    OrderDirection::Descending => cmp.reverse(),
                };

                if final_cmp != std::cmp::Ordering::Equal {
                    return final_cmp;
                }
                // If equal, continue to next sort column
            }
            std::cmp::Ordering::Equal
        });

        Ok(())
    }
}

/// Processor for grouping data before aggregation
pub struct GroupingProcessor;

impl GroupingProcessor {
    /// Group rows by the specified columns
    pub fn group_by_columns(
        rows: &[Row],
        columns: &[String],
        group_by_fields: &[String],
    ) -> Result<HashMap<String, Vec<Row>>> {
        let mut groups: HashMap<String, Vec<Row>> = HashMap::new();

        // Get indices of group-by columns
        let mut group_indices = Vec::new();
        for field in group_by_fields {
            let index = columns
                .iter()
                .position(|c| c == field)
                .ok_or_else(|| anyhow!("Unknown group by column: {}", field))?;
            group_indices.push(index);
        }

        // Group rows by the key constructed from group-by columns
        for row in rows {
            let group_key = Self::build_group_key(row, &group_indices);
            groups.entry(group_key).or_default().push(row.clone());
        }

        Ok(groups)
    }

    /// Build a group key from the specified column values
    fn build_group_key(row: &Row, group_indices: &[usize]) -> String {
        let key_parts: Vec<String> = group_indices
            .iter()
            .map(|&index| {
                row.get(index)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "NULL".to_string())
            })
            .collect();
        
        key_parts.join("|")
    }

    /// Extract values for aggregation from grouped rows
    pub fn extract_aggregation_values(
        grouped_rows: &HashMap<String, Vec<Row>>,
        columns: &[String],
        field: &str,
    ) -> Result<Vec<(String, Vec<Value>)>> {
        let field_index = if field == "*" {
            None
        } else {
            Some(
                columns
                    .iter()
                    .position(|c| c == field)
                    .ok_or_else(|| anyhow!("Unknown field for aggregation: {}", field))?,
            )
        };

        let mut result = Vec::new();
        
        for (group_key, rows) in grouped_rows {
            let values = if let Some(index) = field_index {
                // Extract specific field values
                rows.iter()
                    .filter_map(|row| row.get(index).cloned())
                    .collect()
            } else {
                // For COUNT(*), just create dummy values
                vec![Value::Integer(1); rows.len()]
            };
            
            result.push((group_key.clone(), values));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DiagnosticSeverity;

    #[test]
    fn test_value_comparison() {
        assert_eq!(
            SortingProcessor::compare_values(&Value::Integer(5), &Value::Integer(10)),
            std::cmp::Ordering::Less
        );

        assert_eq!(
            SortingProcessor::compare_values(&Value::String("apple".to_string()), &Value::String("banana".to_string())),
            std::cmp::Ordering::Less
        );

        assert_eq!(
            SortingProcessor::compare_values(&Value::Number(3.14), &Value::Integer(3)),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_aggregation_count() {
        let values = vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ];

        let agg = QueryAggregation::Count("*".to_string());
        let result = AggregationProcessor::compute_aggregation(&agg, &values).unwrap();
        assert_eq!(result, Value::Integer(3));
    }

    #[test]
    fn test_aggregation_sum() {
        let values = vec![
            Value::Integer(10),
            Value::Number(5.5),
            Value::Integer(20),
        ];

        let agg = QueryAggregation::Sum("field".to_string());
        let result = AggregationProcessor::compute_aggregation(&agg, &values).unwrap();
        assert_eq!(result, Value::Number(35.5));
    }

    #[test]
    fn test_aggregation_average() {
        let values = vec![
            Value::Integer(10),
            Value::Integer(20),
            Value::Integer(30),
        ];

        let agg = QueryAggregation::Average("field".to_string());
        let result = AggregationProcessor::compute_aggregation(&agg, &values).unwrap();
        assert_eq!(result, Value::Number(20.0));
    }

    #[test]
    fn test_sorting() {
        let mut rows = vec![
            Row::new(vec![Value::Integer(3), Value::String("c".to_string())]),
            Row::new(vec![Value::Integer(1), Value::String("a".to_string())]),
            Row::new(vec![Value::Integer(2), Value::String("b".to_string())]),
        ];

        let columns = vec!["number".to_string(), "letter".to_string()];
        let order_by = OrderByClause {
            field: "number".to_string(),
            direction: OrderDirection::Ascending,
        };

        SortingProcessor::apply_sorting(&mut rows, &columns, &order_by).unwrap();

        assert_eq!(rows[0].get(0), Some(&Value::Integer(1)));
        assert_eq!(rows[1].get(0), Some(&Value::Integer(2)));
        assert_eq!(rows[2].get(0), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_grouping() {
        let rows = vec![
            Row::new(vec![Value::String("A".to_string()), Value::Integer(1)]),
            Row::new(vec![Value::String("B".to_string()), Value::Integer(2)]),
            Row::new(vec![Value::String("A".to_string()), Value::Integer(3)]),
            Row::new(vec![Value::String("B".to_string()), Value::Integer(4)]),
        ];

        let columns = vec!["category".to_string(), "value".to_string()];
        let group_by_fields = vec!["category".to_string()];

        let groups = GroupingProcessor::group_by_columns(&rows, &columns, &group_by_fields).unwrap();

        assert_eq!(groups.len(), 2);
        assert!(groups.contains_key("A"));
        assert!(groups.contains_key("B"));
        assert_eq!(groups["A"].len(), 2);
        assert_eq!(groups["B"].len(), 2);
    }
}