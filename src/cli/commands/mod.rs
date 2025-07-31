use anyhow::Result;
use async_trait::async_trait;

pub mod export;
pub mod watch;
pub mod query;
pub mod history;
pub mod ai_training;
pub mod quick_fix;
pub mod config;

/// Trait for CLI command implementations
#[async_trait]
pub trait Command {
    /// Execute the command with the given arguments
    async fn execute(&self) -> Result<()>;
}

/// Common utilities for command implementations
pub mod utils {
    use crate::core::types::{DiagnosticFilter, DiagnosticSeverity};
    use anyhow::Result;

    /// Create a diagnostic filter from command line options
    pub fn create_diagnostic_filter(
        errors_only: bool,
        warnings_and_errors: bool,
        files: Option<String>,
        exclude: Option<String>,
        max_results: Option<usize>,
    ) -> Result<DiagnosticFilter> {
        let severities = if errors_only {
            Some(vec![DiagnosticSeverity::Error])
        } else if warnings_and_errors {
            Some(vec![DiagnosticSeverity::Error, DiagnosticSeverity::Warning])
        } else {
            None
        };

        let file_patterns = files.map(|f| f.split(',').map(String::from).collect());
        let exclude_patterns = exclude.map(|e| e.split(',').map(String::from).collect());

        Ok(DiagnosticFilter {
            severities,
            file_patterns,
            exclude_patterns,
            workspace: None,
            start_time: None,
            end_time: None,
            max_results,
            tags: None,
        })
    }
}