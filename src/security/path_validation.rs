//! Path validation utilities for security
//!
//! This module provides secure path validation to prevent:
//! - Path traversal attacks (../)
//! - Absolute path escapes
//! - Symbolic link attacks
//! - Invalid characters in paths

use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::error::{LspBridgeError, LspResult};

/// Validates and normalizes a user-provided path
pub fn validate_path(path: &Path) -> LspResult<PathBuf> {
    // Check for null bytes which are invalid in paths
    let path_str = path.to_str()
        .ok_or_else(|| LspBridgeError::Validation {
            field: "path".to_string(),
            reason: "Path contains invalid UTF-8".to_string(),
        })?;
    
    if path_str.contains('\0') {
        return Err(LspBridgeError::Validation {
            field: "path".to_string(),
            reason: "Path contains null bytes".to_string(),
        });
    }
    
    // Check for path traversal attempts
    if path_str.contains("..") {
        return Err(LspBridgeError::Validation {
            field: "path".to_string(),
            reason: "Path traversal detected".to_string(),
        });
    }
    
    // Normalize the path to remove any redundant components
    let canonical = path.canonicalize()
        .map_err(|e| LspBridgeError::Validation {
            field: "path".to_string(),
            reason: format!("Invalid path: {e}"),
        })?;
    
    Ok(canonical)
}

/// Validates a path pattern (for glob matching)
pub fn validate_pattern(pattern: &str) -> LspResult<String> {
    // Check for null bytes
    if pattern.contains('\0') {
        return Err(LspBridgeError::Validation {
            field: "pattern".to_string(),
            reason: "Pattern contains null bytes".to_string(),
        });
    }
    
    // Check for shell metacharacters that could be dangerous
    const DANGEROUS_CHARS: &[char] = &['$', '`', '\\', '!', '\n', '\r'];
    for ch in DANGEROUS_CHARS {
        if pattern.contains(*ch) {
            return Err(LspBridgeError::Validation {
                field: "pattern".to_string(),
                reason: format!("Pattern contains potentially dangerous character: {ch}"),
            });
        }
    }
    
    // Limit pattern length to prevent DoS
    const MAX_PATTERN_LENGTH: usize = 1024;
    if pattern.len() > MAX_PATTERN_LENGTH {
        return Err(LspBridgeError::Validation {
            field: "pattern".to_string(),
            reason: format!("Pattern too long (max {MAX_PATTERN_LENGTH} characters)"),
        });
    }
    
    Ok(pattern.to_string())
}

/// Validates a path is within a workspace directory
pub fn validate_workspace_path(path: &Path, workspace_root: &Path) -> LspResult<PathBuf> {
    let validated = validate_path(path)?;
    
    // Canonicalize workspace root for proper comparison
    let canonical_workspace = workspace_root.canonicalize()
        .map_err(|e| LspBridgeError::Validation {
            field: "workspace_root".to_string(),
            reason: format!("Invalid workspace root: {e}"),
        })?;
    
    // Ensure the path is within the workspace
    if !validated.starts_with(&canonical_workspace) {
        return Err(LspBridgeError::Validation {
            field: "workspace_path".to_string(),
            reason: "Path is outside of workspace directory".to_string(),
        });
    }
    
    Ok(validated)
}

/// Validates multiple paths efficiently
pub fn validate_paths(paths: &[PathBuf]) -> LspResult<Vec<PathBuf>> {
    paths.iter()
        .map(|p| validate_path(p))
        .collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_path_validation() {
        // Valid paths should pass
        let temp_dir = TempDir::new().unwrap();
        let valid_path = temp_dir.path().join("test.rs");
        fs::write(&valid_path, "test").unwrap();
        
        assert!(validate_path(&valid_path).is_ok());
        
        // Path traversal should fail
        let traversal = Path::new("../../../etc/passwd");
        assert!(validate_path(&traversal).is_err());
        
        // Paths with null bytes should fail
        // Note: Can't easily test this as Rust's Path doesn't accept null bytes
    }
    
    #[test]
    fn test_pattern_validation() {
        // Valid patterns
        assert!(validate_pattern("**/*.rs").is_ok());
        assert!(validate_pattern("src/main.rs").is_ok());
        assert!(validate_pattern("[a-z]*.txt").is_ok());
        
        // Invalid patterns
        assert!(validate_pattern("$(rm -rf /)").is_err());
        assert!(validate_pattern("`echo hack`").is_err());
        assert!(validate_pattern("path\nwith\nnewline").is_err());
        
        // Too long pattern
        let long_pattern = "a".repeat(2000);
        assert!(validate_pattern(&long_pattern).is_err());
    }
    
    #[test]
    fn test_workspace_validation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        
        // Create a file in workspace
        let valid_file = workspace.join("src/main.rs");
        fs::create_dir_all(valid_file.parent().unwrap()).unwrap();
        fs::write(&valid_file, "test").unwrap();
        
        // Valid workspace path
        assert!(validate_workspace_path(&valid_file, workspace).is_ok());
        
        // Path outside workspace should fail
        let outside = Path::new("/tmp/outside.rs");
        assert!(validate_workspace_path(&outside, workspace).is_err());
    }
}