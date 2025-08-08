use crate::query::{Query, QueryParser};
use anyhow::{anyhow, Result};

/// Query validation utilities
pub struct QueryValidator {
    parser: QueryParser,
}

impl Default for QueryValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryValidator {
    pub fn new() -> Self {
        Self {
            parser: QueryParser::new(),
        }
    }

    /// Validate a query string without executing it
    pub fn validate_query(&self, query_str: &str) -> Result<Query> {
        // Basic input validation
        if query_str.is_empty() {
            return Err(anyhow!("Query string cannot be empty"));
        }

        if query_str.len() > 10_000 {
            return Err(anyhow!("Query string too long (max 10KB)"));
        }

        // Parse and validate query
        let query = self.parser.parse(query_str)?;
        
        // Additional semantic validation
        self.validate_semantics(&query)?;
        
        Ok(query)
    }

    /// Validate query semantics
    fn validate_semantics(&self, query: &Query) -> Result<()> {
        // Check for conflicting filters
        self.check_filter_conflicts(query)?;
        
        // Check for reasonable limits
        if let Some(limit) = query.limit {
            if limit > 10_000 {
                return Err(anyhow!("Query limit too high (max 10,000)"));
            }
        }
        
        // Check for expensive operations without limits
        if query.group_by.is_some() && query.limit.is_none() {
            return Err(anyhow!("GROUP BY queries must have a LIMIT clause"));
        }
        
        Ok(())
    }

    /// Check for conflicting or redundant filters
    fn check_filter_conflicts(&self, query: &Query) -> Result<()> {
        use crate::query::parser::QueryFilter;
        
        let mut severity_count = 0;
        let mut category_count = 0;
        
        for filter in &query.filters {
            match filter {
                QueryFilter::Severity(_) => severity_count += 1,
                QueryFilter::Category(_) => category_count += 1,
                _ => {}
            }
        }
        
        if severity_count > 1 {
            return Err(anyhow!("Multiple severity filters are not supported"));
        }
        
        if category_count > 1 {
            return Err(anyhow!("Multiple category filters are not supported"));
        }
        
        Ok(())
    }

    /// Get optimization hints for a query
    pub fn get_optimization_hints(&self, query: &Query) -> Vec<String> {
        let mut hints = Vec::new();

        // Check for missing indexes
        if matches!(query.from, crate::query::parser::FromClause::History) {
            hints.push("Consider adding time-based index for historical queries".to_string());
        }

        // Check for expensive operations
        if query.group_by.is_some() && query.limit.is_none() {
            hints.push("Consider adding LIMIT to grouped queries".to_string());
        }

        // Check for regex patterns that could be optimized
        for filter in &query.filters {
            if let crate::query::parser::QueryFilter::Path(path_filter) = filter {
                if path_filter.is_regex && path_filter.pattern.starts_with('^') {
                    hints.push("Anchored regex patterns are more efficient".to_string());
                }
            }
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query_validation() {
        let validator = QueryValidator::new();
        let result = validator.validate_query("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_long_query_validation() {
        let validator = QueryValidator::new();
        let long_query = "a".repeat(20_000);
        let result = validator.validate_query(&long_query);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_valid_query() {
        let validator = QueryValidator::new();
        let result = validator.validate_query("SELECT * FROM diagnostics WHERE severity = error");
        assert!(result.is_ok());
    }
}