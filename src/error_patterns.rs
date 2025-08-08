//! Error handling patterns and best practices for LSPbridge
//! 
//! This module demonstrates the recommended error handling patterns to use
//! throughout the codebase.

use crate::error::{LspBridgeError, LspResult, ErrorContext};
use std::fs;
use std::path::Path;

/// Example: File operations with proper error handling
/// 
/// Best practices:
/// 1. Use LspResult for functions that can fail with domain-specific errors
/// 2. Add context to IO operations
/// 3. Convert external errors appropriately
pub fn read_config_file(path: &Path) -> LspResult<String> {
    fs::read_to_string(path)
        .io_context("reading configuration file", Some(path.to_path_buf()))
}

/// Example: Multiple operations with error propagation
/// 
/// Best practices:
/// 1. Use ? operator for clean error propagation
/// 2. Add context at each step where it adds value
/// 3. Use descriptive error messages
pub fn process_project_files(project_root: &Path) -> LspResult<Vec<String>> {
    let config_path = project_root.join("config.toml");
    let config = read_config_file(&config_path)?;
    
    let _parsed_config: toml::Value = toml::from_str(&config)
        .map_err(|e| LspBridgeError::Config {
            message: format!("Invalid TOML syntax: {e}"),
            path: Some(config_path.clone()),
        })?;
    
    let files = fs::read_dir(project_root)
        .io_context("listing project directory", Some(project_root.to_path_buf()))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.path().to_str().map(|s| s.to_string())
            })
        })
        .collect();
    
    Ok(files)
}

/// Example: Validation with custom errors
/// 
/// Best practices:
/// 1. Use custom error variants for domain-specific failures
/// 2. Provide helpful error messages
/// 3. Include relevant context (field names, values, etc.)
pub fn validate_project_name(name: &str) -> LspResult<()> {
    if name.is_empty() {
        return Err(LspBridgeError::Validation {
            field: "project_name".to_string(),
            reason: "Project name cannot be empty".to_string(),
        });
    }
    
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(LspBridgeError::Validation {
            field: "project_name".to_string(),
            reason: "Project name can only contain alphanumeric characters, hyphens, and underscores".to_string(),
        });
    }
    
    Ok(())
}

/// Example: Async operations with error handling
/// 
/// Best practices:
/// 1. Use async error handling consistently
/// 2. Add timeouts where appropriate
/// 3. Handle cancellation gracefully
/// 
/// Note: This is a commented example since reqwest is not a dependency
/// ```ignore
/// pub async fn fetch_remote_diagnostics(url: &str) -> LspResult<String> {
///     let response = reqwest::get(url)
///         .await
///         .map_err(|e| LspBridgeError::LspCommunication {
///             message: format!("Failed to fetch diagnostics from {}: {}", url, e),
///         })?;
///     
///     if !response.status().is_success() {
///         return Err(LspBridgeError::LspCommunication {
///             message: format!("Server returned error status: {}", response.status()),
///         });
///     }
///     
///     response.text()
///         .await
///         .map_err(|e| LspBridgeError::LspCommunication {
///             message: format!("Failed to read response body: {}", e),
///         })
/// }
/// ```

/// Example: Result transformation and error mapping
/// 
/// Best practices:
/// 1. Use map_err to convert between error types
/// 2. Preserve error context when converting
/// 3. Add additional context where helpful
pub fn parse_json_diagnostic(json: &str, source_file: &Path) -> LspResult<serde_json::Value> {
    serde_json::from_str(json)
        .map_err(|e| LspBridgeError::Json {
            context: format!("diagnostic data from {source_file:?}"),
            source: e,
        })
}

/// Example: Error recovery and fallback
/// 
/// Best practices:
/// 1. Log errors that are recovered from
/// 2. Provide sensible defaults
/// 3. Document recovery behavior
pub fn load_config_with_fallback(path: &Path) -> String {
    match read_config_file(path) {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!("Failed to load config from {:?}: {}, using default", path, e);
            include_str!("../resources/default.lspbridge.toml").to_string()
        }
    }
}

/// Example: Batch operations with partial failure handling
/// 
/// Best practices:
/// 1. Collect both successes and failures
/// 2. Return partial results when appropriate
/// 3. Provide detailed error reports
pub struct BatchResult<T> {
    pub successes: Vec<T>,
    pub failures: Vec<(String, LspBridgeError)>,
}

pub fn process_files_batch(files: Vec<&Path>) -> BatchResult<String> {
    let mut result = BatchResult {
        successes: Vec::new(),
        failures: Vec::new(),
    };
    
    for file in files {
        match read_config_file(file) {
            Ok(content) => result.successes.push(content),
            Err(e) => result.failures.push((file.display().to_string(), e)),
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_validation_errors() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("valid-name_123").is_ok());
        assert!(validate_project_name("invalid name!").is_err());
    }
    
    #[test]
    fn test_error_context() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent = temp_dir.path().join("missing.txt");
        
        let err = read_config_file(&non_existent).unwrap_err();
        let err_string = err.to_string();
        
        assert!(err_string.contains("reading configuration file"));
        assert!(err_string.contains("missing.txt"));
    }
}